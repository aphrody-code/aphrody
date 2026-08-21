// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Hardware probing and model selection.
//
// The catalog says what each model needs; this module says what the machine
// actually has, and the two together answer "which model should this box run
// for this task". That question is asked by `aphrody model recommend`, by the
// job scheduler before it queues a batch, and by every backend that has to
// pick an execution provider.
//
// The GPU probe shells out to `nvidia-smi` rather than linking NVML. That is a
// deliberate trade: NVML means an FFI dependency, a versioned .so/.dll to find
// at runtime, and a build-time story on three platforms — all to read four
// numbers that `nvidia-smi --query-gpu` prints in CSV on every driver since
// 2012, on both Linux and Windows. A missing binary simply means "no CUDA
// here", which is exactly the answer a CPU-only box should give.
//
// The `Accelerator` enum itself is pure and compiles for wasm32; only the
// probe is host-gated.

use crate::catalog::{Catalog, CatalogEntry, ModelTask, SpeedTier};

/// An execution provider a backend can run a model on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Accelerator {
    /// Plain CPU inference. Always available.
    Cpu,
    /// NVIDIA CUDA (ONNX Runtime CUDA EP, llama.cpp CUDA backend).
    Cuda,
    /// Apple Metal / `CoreML`.
    Metal,
    /// Vulkan compute.
    Vulkan,
    /// Windows `DirectML`.
    DirectMl,
}

impl Accelerator {
    /// Stable machine-friendly name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::Metal => "metal",
            Self::Vulkan => "vulkan",
            Self::DirectMl => "directml",
        }
    }

    /// Parse the machine-friendly name.
    #[must_use]
    pub fn from_str_opt(raw: &str) -> Option<Self> {
        [Self::Cpu, Self::Cuda, Self::Metal, Self::Vulkan, Self::DirectMl]
            .into_iter()
            .find(|a| a.as_str() == raw)
    }
}

impl core::fmt::Display for Accelerator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One discrete GPU as reported by the driver.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GpuInfo {
    /// Marketing name, e.g. `NVIDIA GeForce RTX 4070`.
    pub name: String,
    /// Total on-board memory in bytes.
    pub vram_bytes: u64,
    /// Free memory in bytes at probe time.
    pub vram_free_bytes: u64,
    /// Driver version string.
    pub driver: String,
    /// CUDA compute capability, e.g. `8.9`.
    pub compute_capability: String,
}

/// What this machine can run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HardwareProfile {
    /// Execution providers believed usable here, best first.
    pub accelerators: Vec<Accelerator>,
    /// Discrete GPUs found.
    pub gpus: Vec<GpuInfo>,
    /// CUDA toolkit version when one is installed, e.g. `13.3`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cuda_toolkit: Option<String>,
    /// Logical CPU count.
    pub cpu_threads: usize,
}

impl HardwareProfile {
    /// The best accelerator available: the first entry, CPU as the floor.
    #[must_use]
    pub fn best(&self) -> Accelerator {
        self.accelerators.first().copied().unwrap_or(Accelerator::Cpu)
    }

    /// Largest VRAM across the detected GPUs, `0` when there is none.
    #[must_use]
    pub fn max_vram_bytes(&self) -> u64 {
        self.gpus.iter().map(|g| g.vram_bytes).max().unwrap_or(0)
    }

    /// Whether an entry's requirements fit this machine.
    ///
    /// A model needing VRAM still qualifies on a CPU-only box if the backend
    /// can run it on CPU: it will be slow, not impossible. What disqualifies
    /// an entry is asking for MORE VRAM than the largest card has while also
    /// being unable to fall back to CPU.
    #[must_use]
    pub fn can_run(&self, entry: &CatalogEntry) -> bool {
        let cpu_capable = entry.accel.contains(&Accelerator::Cpu);
        if entry.vram_min_bytes == 0 {
            return true;
        }
        if self.max_vram_bytes() >= entry.vram_min_bytes
            && entry.accel.iter().any(|a| self.accelerators.contains(a))
        {
            return true;
        }
        cpu_capable
    }

    /// Whether an entry would actually be GPU-accelerated here.
    #[must_use]
    pub fn is_accelerated(&self, entry: &CatalogEntry) -> bool {
        entry
            .accel
            .iter()
            .any(|a| *a != Accelerator::Cpu && self.accelerators.contains(a))
            && self.max_vram_bytes() >= entry.vram_min_bytes
    }
}

/// A ranked recommendation for one task on this machine.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Recommendation {
    /// Catalog id of the chosen model.
    pub id: String,
    /// Task the recommendation is for.
    pub task: ModelTask,
    /// Accelerator the model would run on here.
    pub accelerator: Accelerator,
    /// Total download size, when every file declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_bytes: Option<u64>,
    /// Why this entry won, in one sentence.
    pub rationale: String,
}

/// Rank the catalog for a task against a hardware profile.
///
/// The ordering is deliberate and, in this order:
///
/// 1. entries the machine can actually run at all;
/// 2. GPU-accelerated before CPU-only, when a GPU is present;
/// 3. throughput tier, honouring `prefer` (a mass-OCR job wants
///    [`SpeedTier::Fast`]; a one-off scan of a hard page wants
///    [`SpeedTier::Quality`]);
/// 4. smaller download as the tie-break, because a smaller artefact is a
///    faster first run and a cheaper eviction.
#[must_use]
pub fn rank_for(
    catalog: &Catalog,
    task: ModelTask,
    profile: &HardwareProfile,
    prefer: SpeedTier,
) -> Vec<Recommendation> {
    let mut candidates: Vec<&CatalogEntry> =
        catalog.by_task(task).into_iter().filter(|e| profile.can_run(e)).collect();

    candidates.sort_by_key(|entry| {
        let accelerated = u8::from(!profile.is_accelerated(entry));
        // Distance from the requested tier: 0 is an exact match.
        let tier_distance = (entry.speed as i8 - prefer as i8).unsigned_abs();
        (accelerated, tier_distance, entry.total_bytes().unwrap_or(u64::MAX))
    });

    candidates
        .into_iter()
        .map(|entry| {
            let accelerator = if profile.is_accelerated(entry) {
                entry
                    .accel
                    .iter()
                    .copied()
                    .find(|a| *a != Accelerator::Cpu && profile.accelerators.contains(a))
                    .unwrap_or(Accelerator::Cpu)
            } else {
                Accelerator::Cpu
            };
            Recommendation {
                id: entry.id.clone(),
                task: entry.task,
                accelerator,
                download_bytes: entry.total_bytes(),
                rationale: rationale(entry, accelerator, profile),
            }
        })
        .collect()
}

fn rationale(entry: &CatalogEntry, accelerator: Accelerator, profile: &HardwareProfile) -> String {
    let where_it_runs = match accelerator {
        Accelerator::Cpu => format!("on CPU ({} threads)", profile.cpu_threads),
        other => match profile.gpus.first() {
            Some(gpu) => format!("on {other} ({})", gpu.name),
            None => format!("on {other}"),
        },
    };
    format!(
        "{} tier for {}, {} backend, runs {where_it_runs}",
        entry.speed, entry.task, entry.backend
    )
}

// ---------------------------------------------------------------------------
// Probing (host-only)
// ---------------------------------------------------------------------------

/// Probe this machine for usable accelerators.
///
/// Never fails: an absent driver, an absent `nvidia-smi`, or an unparseable
/// line all degrade to "CPU only", which is the truthful answer for a box
/// where nothing else could be proven to work.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn probe() -> HardwareProfile {
    let gpus = probe_nvidia();
    let cuda_toolkit = probe_cuda_toolkit();

    let mut accelerators = Vec::new();
    // CUDA needs BOTH a card and a runtime: a driver with no toolkit still
    // runs prebuilt CUDA binaries, so the card alone is enough to claim it.
    if !gpus.is_empty() {
        accelerators.push(Accelerator::Cuda);
    }
    if cfg!(target_os = "macos") {
        accelerators.push(Accelerator::Metal);
    }
    if cfg!(target_os = "windows") {
        accelerators.push(Accelerator::DirectMl);
    }
    accelerators.push(Accelerator::Cpu);

    HardwareProfile {
        accelerators,
        gpus,
        cuda_toolkit,
        cpu_threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
    }
}

/// Ask `nvidia-smi` for every card it can see.
#[cfg(not(target_arch = "wasm32"))]
fn probe_nvidia() -> Vec<GpuInfo> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free,driver_version,compute_cap",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let Ok(output) = output else { return Vec::new() };
    if !output.status.success() {
        return Vec::new();
    }
    parse_nvidia_smi(&String::from_utf8_lossy(&output.stdout))
}

/// Parse the CSV `nvidia-smi --format=csv,noheader,nounits` emits.
///
/// Split out from the process call so the parsing is testable without a GPU.
/// Memory columns are mebibytes under `nounits`.
#[must_use]
pub fn parse_nvidia_smi(stdout: &str) -> Vec<GpuInfo> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            // Fewer than five columns means a different query than ours ran.
            if fields.len() < 5 {
                return None;
            }
            let mib_to_bytes = |raw: &str| raw.parse::<u64>().ok().map(|mib| mib * 1024 * 1024);
            Some(GpuInfo {
                name: fields[0].to_owned(),
                vram_bytes: mib_to_bytes(fields[1])?,
                vram_free_bytes: mib_to_bytes(fields[2]).unwrap_or(0),
                driver: fields[3].to_owned(),
                compute_capability: fields[4].to_owned(),
            })
        })
        .collect()
}

/// Detect an installed CUDA toolkit from the usual environment markers.
#[cfg(not(target_arch = "wasm32"))]
fn probe_cuda_toolkit() -> Option<String> {
    for var in ["CUDA_PATH", "CUDA_HOME", "CUDA_ROOT"] {
        if let Ok(path) = std::env::var(var) {
            if !path.is_empty() {
                return Some(cuda_version_from_path(&path));
            }
        }
    }
    None
}

/// Pull a `vX.Y` / `X.Y` version out of a CUDA install path, falling back to
/// the whole path when it carries no recognisable version.
#[must_use]
pub fn cuda_version_from_path(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|segment| {
            let candidate = segment.strip_prefix('v').unwrap_or(segment);
            !candidate.is_empty()
                && candidate.chars().all(|c| c.is_ascii_digit() || c == '.')
                && candidate.contains('.')
        })
        .map_or_else(|| path.to_owned(), |segment| segment.trim_start_matches('v').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu(name: &str, gib: u64) -> GpuInfo {
        GpuInfo {
            name: name.to_owned(),
            vram_bytes: gib * 1024 * 1024 * 1024,
            vram_free_bytes: gib * 1024 * 1024 * 1024,
            driver: "610.88".to_owned(),
            compute_capability: "8.9".to_owned(),
        }
    }

    fn cuda_box(gib: u64) -> HardwareProfile {
        HardwareProfile {
            accelerators: vec![Accelerator::Cuda, Accelerator::Cpu],
            gpus: vec![gpu("NVIDIA GeForce RTX 4070", gib)],
            cuda_toolkit: Some("13.3".to_owned()),
            cpu_threads: 16,
        }
    }

    fn cpu_box() -> HardwareProfile {
        HardwareProfile {
            accelerators: vec![Accelerator::Cpu],
            gpus: Vec::new(),
            cuda_toolkit: None,
            cpu_threads: 8,
        }
    }

    #[test]
    fn nvidia_smi_csv_is_parsed() {
        let stdout = "NVIDIA GeForce RTX 4070, 12282, 11000, 610.88, 8.9\n";
        let gpus = parse_nvidia_smi(stdout);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 4070");
        assert_eq!(gpus[0].vram_bytes, 12282 * 1024 * 1024);
        assert_eq!(gpus[0].vram_free_bytes, 11000 * 1024 * 1024);
        assert_eq!(gpus[0].compute_capability, "8.9");
    }

    #[test]
    fn multi_gpu_output_is_parsed() {
        let stdout = "A, 8000, 8000, 1, 8.6\nB, 24000, 20000, 1, 8.9\n";
        let gpus = parse_nvidia_smi(stdout);
        assert_eq!(gpus.len(), 2);
        let profile = HardwareProfile {
            accelerators: vec![Accelerator::Cuda, Accelerator::Cpu],
            gpus,
            cuda_toolkit: None,
            cpu_threads: 4,
        };
        assert_eq!(profile.max_vram_bytes(), 24000 * 1024 * 1024);
    }

    #[test]
    fn garbage_and_empty_smi_output_yield_no_gpus() {
        assert!(parse_nvidia_smi("").is_empty());
        assert!(parse_nvidia_smi("\n\n").is_empty());
        assert!(parse_nvidia_smi("some driver error\n").is_empty());
        // A truncated row is dropped, not half-parsed.
        assert!(parse_nvidia_smi("NVIDIA, 12282\n").is_empty());
        // A non-numeric memory column is dropped too.
        assert!(parse_nvidia_smi("NVIDIA, N/A, N/A, 610.88, 8.9\n").is_empty());
    }

    #[test]
    fn cuda_version_is_extracted_from_install_paths() {
        assert_eq!(
            cuda_version_from_path("C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v13.3"),
            "13.3"
        );
        assert_eq!(cuda_version_from_path("/usr/local/cuda-12.4"), "/usr/local/cuda-12.4");
        assert_eq!(cuda_version_from_path("/opt/cuda/12.8"), "12.8");
    }

    #[test]
    fn accelerator_names_round_trip() {
        for accel in [
            Accelerator::Cpu,
            Accelerator::Cuda,
            Accelerator::Metal,
            Accelerator::Vulkan,
            Accelerator::DirectMl,
        ] {
            assert_eq!(Accelerator::from_str_opt(accel.as_str()), Some(accel));
        }
        assert_eq!(Accelerator::from_str_opt("tpu"), None);
    }

    #[test]
    fn a_mass_ocr_job_on_a_gpu_box_gets_the_fast_tier_first() {
        let ranked =
            rank_for(Catalog::builtin(), ModelTask::Ocr, &cuda_box(12), SpeedTier::Fast);
        assert!(!ranked.is_empty());
        assert_eq!(ranked[0].id, "ppocr-v5-mobile", "{ranked:#?}");
        assert_eq!(ranked[0].accelerator, Accelerator::Cuda);
        assert!(ranked[0].rationale.contains("RTX 4070"), "{}", ranked[0].rationale);
    }

    #[test]
    fn asking_for_quality_reorders_the_same_candidates() {
        let ranked =
            rank_for(Catalog::builtin(), ModelTask::Ocr, &cuda_box(12), SpeedTier::Quality);
        assert_eq!(ranked[0].id, "dots-ocr", "{ranked:#?}");
    }

    #[test]
    fn a_small_card_cannot_hold_the_quality_tier_on_gpu() {
        // dots.ocr wants 5 GiB resident; a 4 GiB card must not be told it is
        // GPU-accelerated for it.
        let small = cuda_box(4);
        let entry = Catalog::builtin().get("dots-ocr").unwrap();
        assert!(!small.is_accelerated(entry));
        // It stays runnable, because the entry also lists CPU.
        assert!(small.can_run(entry));
    }

    #[test]
    fn a_cpu_only_box_still_gets_a_recommendation() {
        let ranked = rank_for(Catalog::builtin(), ModelTask::Ocr, &cpu_box(), SpeedTier::Fast);
        assert!(!ranked.is_empty());
        assert!(ranked.iter().all(|r| r.accelerator == Accelerator::Cpu));
        assert_eq!(ranked[0].id, "ppocr-v5-mobile");
        assert!(ranked[0].rationale.contains("8 threads"), "{}", ranked[0].rationale);
    }

    #[test]
    fn every_task_in_the_catalog_can_be_recommended_for() {
        let profile = cuda_box(12);
        for task in ModelTask::all() {
            let ranked = rank_for(Catalog::builtin(), *task, &profile, SpeedTier::Balanced);
            if !Catalog::builtin().by_task(*task).is_empty() {
                assert!(!ranked.is_empty(), "no recommendation for {task}");
            }
        }
    }

    #[test]
    fn best_falls_back_to_cpu_when_nothing_else_is_present() {
        assert_eq!(cpu_box().best(), Accelerator::Cpu);
        assert_eq!(cuda_box(12).best(), Accelerator::Cuda);
        assert_eq!(cpu_box().max_vram_bytes(), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn probing_this_machine_never_panics_and_always_offers_cpu() {
        let profile = probe();
        assert!(profile.accelerators.contains(&Accelerator::Cpu));
        assert!(profile.cpu_threads >= 1);
        // Whatever this box is, the profile must be self-consistent.
        if profile.gpus.is_empty() {
            assert_eq!(profile.max_vram_bytes(), 0);
        } else {
            assert!(profile.accelerators.contains(&Accelerator::Cuda));
            assert!(profile.max_vram_bytes() > 0);
        }
    }
}
