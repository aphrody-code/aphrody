use std::sync::Arc;

use backend::{Md3Mirror, dns::DnsRecon};
use serde::{Deserialize, Serialize};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use tokio::runtime::Runtime;
use tracing::{error, info};
use wry::WebViewBuilder;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "lowercase")]
enum IpcMessage {
    Prompt(String),
}

fn dispatch_prompt(rt: &Arc<Runtime>, prompt: String) {
    let trimmed = prompt.trim().to_owned();

    if let Some(domain) = trimmed.strip_prefix("dns:") {
        let domain = domain.trim().to_owned();
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match DnsRecon::new().run_osint(&domain).await {
                Ok(records) => {
                    info!("DNS OSINT for {}: {} record(s) found", domain, records.len());
                    for r in &records {
                        info!("  {}", r);
                    }
                }
                Err(e) => error!("DNS OSINT failed for {}: {:#}", domain, e),
            }
        });
    } else if trimmed.starts_with("mirror") {
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match Md3Mirror::new() {
                Ok(mirror) => {
                    if let Err(e) = mirror.start_mirroring().await {
                        error!("Md3Mirror mirroring failed: {:#}", e);
                    }
                }
                Err(e) => error!("Md3Mirror init failed: {:#}", e),
            }
        });
    } else {
        // Default: treat the whole prompt as a domain for DNS OSINT.
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match DnsRecon::new().run_osint(&trimmed).await {
                Ok(records) => {
                    info!("DNS OSINT for {}: {} record(s) found", trimmed, records.len());
                    for r in &records {
                        info!("  {}", r);
                    }
                }
                Err(e) => error!("DNS OSINT failed for {}: {:#}", trimmed, e),
            }
        });
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let rt = Arc::new(Runtime::new()?);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("aphrody")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;

    let html_content = include_str!("index.html");

    let rt_ipc = Arc::clone(&rt);
    let _webview = WebViewBuilder::new()
        .with_html(html_content)
        .with_ipc_handler(move |request| {
            let body = request.body();
            match serde_json::from_str::<IpcMessage>(body) {
                Ok(IpcMessage::Prompt(p)) => dispatch_prompt(&rt_ipc, p),
                Err(e) => error!("invalid IPC message: {} ({:#})", body, e),
            }
        })
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}
