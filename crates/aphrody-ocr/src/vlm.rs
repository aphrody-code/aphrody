// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Running a GGUF vision-language model over images via llama.cpp.
//
// One process per image. That is deliberate for a first cut: `llama-mtmd-cli`
// reloads the model each time (~1 s on this hardware for a 258M model), which
// is wasteful, but it is also crash-isolated — one malformed plate cannot take
// down a batch of ten thousand. Keeping a resident model is what
// `llama-server` is for, and moving to it is a contained change behind this
// same interface once throughput matters more than isolation.

use std::path::{Path, PathBuf};
use std::process::Command;

use aphrody_infer::llama::{self, LlamaTool};
use aphrody_models::{Catalog, ModelStore};

use crate::doctags::Document;
use crate::{OcrError, Result};

/// What a page turned out to hold.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PageText {
    /// Readable text, rendered as markdown.
    Text {
        /// The markdown transcription.
        markdown: String,
    },
    /// No readable text at all: a full-page illustration, or a plate the model
    /// could not read. Callers forward this as an explicit null rather than as
    /// an empty string — an empty string reads as "transcribed to nothing",
    /// which is a different and wrong claim.
    None,
}

impl PageText {
    /// The markdown, when there is any.
    #[must_use]
    pub fn markdown(&self) -> Option<&str> {
        match self {
            Self::Text { markdown } => Some(markdown),
            Self::None => None,
        }
    }

    /// Whether the page carries text.
    #[must_use]
    pub const fn has_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }
}

/// The outcome of reading one image.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PageResult {
    /// The image that was read.
    pub image: PathBuf,
    /// What was found.
    pub text: PageText,
    /// Wall-clock milliseconds the model took.
    pub elapsed_ms: u128,
    /// Raw model output, kept for auditing a surprising result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

/// Knobs for a run.
#[derive(Debug, Clone)]
pub struct OcrOptions {
    /// Catalog id of the vision model to use.
    pub model_id: String,
    /// Prompt handed to the model.
    pub prompt: String,
    /// Token budget per page. A dense plate needs room; a runaway generation
    /// wastes GPU time on a page that has nothing to say.
    pub max_tokens: u32,
    /// Layers to offload to the GPU. 99 means "all of them".
    pub gpu_layers: u32,
    /// Keep the model's raw output in each result.
    pub keep_raw: bool,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self::for_model("granite-docling-258m")
    }
}

impl OcrOptions {
    /// Options for a catalog model, with the prompt that model was trained on.
    ///
    /// The prompt is not interchangeable between families. Docling models
    /// answer to `Convert this page to docling.` and caption instead of
    /// transcribing when given a free-form instruction; dots.ocr answers to a
    /// plain extraction request and returns an empty turn for a Docling
    /// prompt. Getting this wrong looks exactly like a page with no text,
    /// which is the most expensive failure this pipeline can have.
    #[must_use]
    pub fn for_model(model_id: &str) -> Self {
        let prompt = default_prompt(model_id).to_owned();
        Self {
            model_id: model_id.to_owned(),
            prompt,
            max_tokens: 1024,
            gpu_layers: 99,
            keep_raw: false,
        }
    }
}

/// The trained instruction for a catalog model.
#[must_use]
pub fn default_prompt(model_id: &str) -> &'static str {
    if model_id.starts_with("dots-ocr") {
        "Extract all text from this image."
    } else {
        // Docling family (granite-docling, and anything else emitting DocTags).
        "Convert this page to docling."
    }
}

/// A configured vision-language runner.
pub struct VlmRunner {
    binary: PathBuf,
    weights: PathBuf,
    mmproj: PathBuf,
    options: OcrOptions,
}

impl VlmRunner {
    /// Resolve the binary and the model artefacts named by `options`.
    ///
    /// # Errors
    ///
    /// [`OcrError::NoRunner`] when llama.cpp is not installed, or
    /// [`OcrError::Model`] when the catalog entry's weights are not pulled.
    pub fn new(options: OcrOptions) -> Result<Self> {
        let source = llama::resolve(LlamaTool::Multimodal).ok_or(OcrError::NoRunner)?;

        let entry = Catalog::builtin().get(&options.model_id)?;
        let store = ModelStore::open()?;

        // A vision GGUF is useless without its projector, so both are resolved
        // up front: failing here beats failing on the first image of a batch.
        let weights = installed_path(&store, entry, "weights")?;
        let mmproj = installed_path(&store, entry, "mmproj")?;

        Ok(Self { binary: source.path().to_path_buf(), weights, mmproj, options })
    }

    /// The resolved runner binary.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Read one image.
    ///
    /// # Errors
    ///
    /// [`OcrError::Process`] when the model process fails.
    pub fn read(&self, image: &Path) -> Result<PageResult> {
        let started = std::time::Instant::now();

        let output = Command::new(&self.binary)
            .arg("-m")
            .arg(&self.weights)
            .arg("--mmproj")
            .arg(&self.mmproj)
            .arg("--image")
            .arg(image)
            .arg("-p")
            .arg(&self.options.prompt)
            .arg("-ngl")
            .arg(self.options.gpu_layers.to_string())
            .arg("-n")
            .arg(self.options.max_tokens.to_string())
            // Transcription must be reproducible: the same plate has to give
            // the same text on a re-run, or an idempotent deposit is a lie.
            .arg("--temp")
            .arg("0")
            .arg("--no-warmup")
            .output()
            .map_err(|source| OcrError::Io { path: self.binary.clone(), source })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OcrError::Process {
                command: self.binary.display().to_string(),
                status: output.status.to_string(),
                stderr: tail(&stderr, 400),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let document = Document::parse(&stdout);
        let text = document
            .to_markdown()
            .map_or(PageText::None, |markdown| PageText::Text { markdown });

        Ok(PageResult {
            image: image.to_path_buf(),
            text,
            elapsed_ms: started.elapsed().as_millis(),
            raw: self.options.keep_raw.then_some(stdout),
        })
    }

    /// Read every image in a directory, in sorted order.
    ///
    /// `on_page` is called after each image so a caller can stream results to
    /// disk or to an API instead of holding a whole batch in memory — which
    /// matters when a batch is four hundred plates.
    ///
    /// A page that fails is reported through `on_page` as an error and does not
    /// abort the run: one unreadable file must not cost the other 399.
    ///
    /// # Errors
    ///
    /// [`OcrError::Io`] when the directory cannot be read.
    pub fn read_dir(
        &self,
        dir: &Path,
        limit: Option<usize>,
        on_page: impl FnMut(core::result::Result<PageResult, OcrError>),
    ) -> Result<usize> {
        self.read_dir_filtered(dir, limit, &std::collections::BTreeSet::new(), on_page)
    }

    /// Like [`Self::read_dir`], but skipping images already in `done`.
    ///
    /// This is what makes a long run resumable: a batch of ten thousand plates
    /// takes hours, and re-reading the first nine thousand after an
    /// interruption would cost more than the interruption did.
    ///
    /// `limit` counts pages actually read, not pages considered, so
    /// `--limit 50` on a resumed run reads fifty NEW pages.
    ///
    /// # Errors
    ///
    /// [`OcrError::Io`] when the directory cannot be read.
    pub fn read_dir_filtered(
        &self,
        dir: &Path,
        limit: Option<usize>,
        done: &std::collections::BTreeSet<PathBuf>,
        mut on_page: impl FnMut(core::result::Result<PageResult, OcrError>),
    ) -> Result<usize> {
        let mut images = list_images(dir)?;
        images.sort();

        let pending: Vec<PathBuf> = images.into_iter().filter(|p| !done.contains(p)).collect();
        let total = pending.len();

        for image in pending.into_iter().take(limit.unwrap_or(usize::MAX)) {
            on_page(self.read(&image));
        }
        Ok(total)
    }
}

/// Every image file directly under `dir`, sorted.
///
/// Public because a caller that drives its own loop — one picking between the
/// per-process and resident backends — needs the same file selection and the
/// same order, or a resumed run would skip different pages than it recorded.
///
/// # Errors
///
/// [`OcrError::Io`] when the directory cannot be read.
pub fn list_images_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut images = list_images(dir)?;
    images.sort();
    Ok(images)
}

/// Every image file directly under `dir`.
fn list_images(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries =
        std::fs::read_dir(dir).map_err(|source| OcrError::Io { path: dir.to_path_buf(), source })?;
    Ok(entries
        .filter_map(core::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_image(path))
        .collect())
}

/// Whether a path looks like an image this pipeline can hand to a model.
#[must_use]
pub fn is_image(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
        matches!(ext.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp")
    })
}

/// Resolve the weights and projector of a catalog vision entry.
///
/// Both are resolved up front: a vision GGUF is useless without its mmproj,
/// and failing here beats failing on the first image of a batch.
pub(crate) fn resolve_artifacts(model_id: &str) -> Result<(PathBuf, PathBuf)> {
    let entry = Catalog::builtin().get(model_id)?;
    let store = ModelStore::open()?;
    Ok((installed_path(&store, entry, "weights")?, installed_path(&store, entry, "mmproj")?))
}

/// Resolve one role of a catalog entry to an installed path.
fn installed_path(
    store: &ModelStore,
    entry: &aphrody_models::CatalogEntry,
    role: &str,
) -> Result<PathBuf> {
    let file = entry.file(role).ok_or_else(|| {
        OcrError::Infer(aphrody_infer::InferError::MissingRole {
            entry: entry.id.clone(),
            role: role.to_owned(),
        })
    })?;
    let installed = store
        .get(&file.reference)?
        .ok_or_else(|| aphrody_models::ModelError::NotInstalled(file.reference.to_string()))?;
    Ok(installed.path)
}

/// Last `limit` characters of a string, on a char boundary.
pub(crate) fn tail(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_owned();
    }
    trimmed.chars().skip(trimmed.chars().count() - limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_extensions_are_recognised_case_insensitively() {
        for name in ["a.jpg", "a.JPG", "a.jpeg", "a.png", "a.WebP", "a.bmp"] {
            assert!(is_image(Path::new(name)), "{name}");
        }
        for name in ["manifeste.json", "a.txt", "a", "a.jpg.bak", "a.pdf"] {
            assert!(!is_image(Path::new(name)), "{name}");
        }
    }

    #[test]
    fn listing_skips_the_manifest_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["2-0002.jpg", "1-0001.jpg", "manifeste.json", "notes.txt"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let mut found = list_images(dir.path()).unwrap();
        found.sort();
        let names: Vec<String> =
            found.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["1-0001.jpg", "2-0002.jpg"]);
    }

    #[test]
    fn listing_a_missing_directory_is_an_io_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(list_images(&dir.path().join("absent")), Err(OcrError::Io { .. })));
    }

    #[test]
    fn page_text_distinguishes_nothing_from_empty() {
        let none = PageText::None;
        assert!(!none.has_text());
        assert_eq!(none.markdown(), None);

        let some = PageText::Text { markdown: "hello".into() };
        assert!(some.has_text());
        assert_eq!(some.markdown(), Some("hello"));

        // The serialised shape is what a deposit script branches on.
        let json = serde_json::to_string(&none).unwrap();
        assert_eq!(json, r#"{"kind":"none"}"#);
    }

    #[test]
    fn default_options_use_the_docling_instruction() {
        let options = OcrOptions::default();
        assert_eq!(options.model_id, "granite-docling-258m");
        // A free-form prompt makes a document model caption instead of
        // transcribe; the trained instruction is not interchangeable.
        assert!(options.prompt.contains("docling"), "{}", options.prompt);
        assert_eq!(options.gpu_layers, 99);
    }

    #[test]
    fn stderr_tail_is_bounded_and_char_safe() {
        let long = "é".repeat(1000);
        let tailed = tail(&long, 400);
        assert_eq!(tailed.chars().count(), 400);
        assert_eq!(tail("short", 400), "short");
        assert_eq!(tail("  padded  ", 400), "padded");
    }
}
