# wgpu / WebGPU — Browser Setup

Source : `gfx-rs/wgpu` 26.0+ official docs (29.x is canary as of 2026-05-17).

## Cargo setup for browser

```toml
[dependencies]
wgpu = { version = "26", default-features = false, features = ["webgpu", "webgl"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["HtmlCanvasElement", "Window", "Document"] }
log = "0.4"
console_log = { version = "1.0", features = ["color"] }
```

Two features matter :
- `webgpu` — native WebGPU backend (Chrome 113+, Edge 113+, Safari 26 TP, Firefox Nightly).
- `webgl` — WebGL2 fallback for older browsers.

The crate will pick WebGPU first, fall back to WebGL2 when the surface advertises only `compatibility` capabilities.

`.cargo/config.toml` (workspace) :
```toml
[build]
rustflags = ["--cfg=web_sys_unstable_apis"]      # required for the WebGPU bindings in web-sys
```

## Init pipeline

```rust
use wgpu::{
    Instance, InstanceDescriptor, Backends, RequestAdapterOptions, PowerPreference,
    DeviceDescriptor, Features, Limits,
    SurfaceConfiguration, PresentMode, TextureUsages, CompositeAlphaMode,
};

pub async fn init(canvas: web_sys::HtmlCanvasElement) -> Result<RenderState, JsValue> {
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::BROWSER_WEBGPU | Backends::GL,
        ..InstanceDescriptor::new_without_display_handle()
    });

    // wgpu 26+: pass the canvas directly via raw-window-handle bridge
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("surface: {e}")))?;

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| JsValue::from_str("no compatible GPU adapter"))?;

    let info = adapter.get_info();
    log::info!("Using {} ({:?})", info.name, info.backend);

    // Choose limits based on capability — WebGL2 ≠ WebGPU
    let required_limits = if adapter.get_downlevel_capabilities()
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    {
        Limits::default()
    } else {
        Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    };

    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            label: Some("aphrody-wgpu-device"),
            required_features: Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("device: {e}")))?;

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats[0];

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: canvas.width(),
        height: canvas.height(),
        present_mode: PresentMode::AutoVsync,
        alpha_mode: CompositeAlphaMode::Auto,
        view_formats: vec![format.add_srgb_suffix()],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    Ok(RenderState { surface, device, queue, config })
}
```

## Render loop in the browser

`requestAnimationFrame` is the only correct way — wgpu's `surface.get_current_texture()` is a no-op on first frame ; the canvas isn't ready until the first paint :

```rust
use wasm_bindgen::{JsCast, prelude::Closure};
use std::{cell::RefCell, rc::Rc};

pub fn start_loop(state: Rc<RefCell<RenderState>>) {
    let window = web_sys::window().unwrap();
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let state = state.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        render_frame(&state.borrow_mut());
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window().unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("rAF");
}
```

## Frame-acquisition error handling

In wgpu 26+, `get_current_texture()` returns `CurrentSurfaceTexture` with a richer error set than older versions :

```rust
let frame = match state.surface.get_current_texture() {
    wgpu::CurrentSurfaceTexture::Success(f) => f,
    wgpu::CurrentSurfaceTexture::Timeout
    | wgpu::CurrentSurfaceTexture::Occluded => return,        // try next rAF
    wgpu::CurrentSurfaceTexture::Outdated
    | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
        state.surface.configure(&state.device, &state.config);
        return;
    }
    wgpu::CurrentSurfaceTexture::Lost => {
        // tab was backgrounded; recreate surface
        return;
    }
    wgpu::CurrentSurfaceTexture::Validation => unreachable!(),
};
```

## WebGL2 vs WebGPU at runtime

```rust
let downlevel = adapter.get_downlevel_capabilities();
let has_compute = downlevel.flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);
if !has_compute {
    log::warn!("running on WebGL2 fallback — compute shaders disabled");
    // pick a render-only pipeline
}
```

Always set `Limits::downlevel_webgl2_defaults()` when targeting WebGL2 — the defaults exceed what WebGL2 can expose.

## Bundle size

A minimal wgpu app post `wasm-opt -Oz` lands around **350-450 KB** gzipped. That's the price of cross-API safety. Cut further by :
- Disabling unused features (`default-features = false` and pick only what you need)
- Stripping debug info (`strip = true` in `[profile.release]`)
- Avoiding `wgpu::util::DeviceExt` if you can write the buffer setup by hand

## Version policy — pin 26.0.x in production

The `29.x` line shipped on 2026-03-18 (`29.0.0`) with bug-fix releases up to
`29.0.3` (2026-03-26+). It is **stable in name but breaking in shape**.
Production code in `aphrody-code/ui` and similar should **pin `wgpu = "26"`**
until you have time to absorb the migration below.

### Breaking changes 26 → 29 (verified from official CHANGELOG)

1. **`Surface::get_current_texture` no longer returns a `Result`** — instead, a
   `CurrentSurfaceTexture` enum with explicit variants. `SurfaceError` is gone ;
   the `suboptimal: bool` field on `SurfaceTexture` is now a dedicated
   `Suboptimal` variant.

   ```rust
   // v29
   match surface.get_current_texture() {
       wgpu::CurrentSurfaceTexture::Success(frame) => { /* render */ }
       wgpu::CurrentSurfaceTexture::Timeout
       | wgpu::CurrentSurfaceTexture::Occluded => { /* skip frame */ }
       wgpu::CurrentSurfaceTexture::Outdated
       | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
           surface.configure(&device, &config);
       }
       wgpu::CurrentSurfaceTexture::Lost => { /* recreate */ }
       wgpu::CurrentSurfaceTexture::Validation => { /* error scope captured it */ }
   }
   ```

   This is the API the snippet earlier in this page already shows — written for
   v29. If you're still on v26, use the older `Result<SurfaceTexture, SurfaceError>`
   pattern.

2. **`InstanceDescriptor` constructors changed.** The `Default` impl and the
   `from_env_or_default` static were removed. New static methods force you to
   declare whether a display handle is used :

   ```rust
   // v29
   let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
   // or
   let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
       Box::new(display_handle),
   ));
   ```

3. **`SurfaceTexture::present()` removed**, replaced by `Queue::present(surface_texture)`.

   ```diff
   - surface_texture.present();
   + queue.present(surface_texture);
   ```

4. **`PipelineLayoutDescriptor::bind_group_layouts`** is now
   `&[Option<&BindGroupLayout>]` (allows gaps + unbind). Migration :

   ```diff
   - bind_group_layouts: &[&bgl],
   + bind_group_layouts: &[Some(&bgl)],
   ```

5. **`VertexState::buffers`** is now `&[Option<VertexBufferLayout>]` for the
   same reason. Wrap existing layouts in `Some`.

6. `Features::CLIP_DISTANCE` renamed to `CLIP_DISTANCES` (plural), to match
   the WebGPU spec. Naga and built-ins follow.

7. `ComputePass`/`RenderPass` : `dispatch` / `dispatch_indirect` renamed to
   `dispatch_workgroups` / `dispatch_workgroups_indirect` (spec alignment).

8. **MSRV** : v29 lowered MSRV to 1.87 (v27 was 1.88, v28 was 1.92). Going
   forward wgpu commits to never bumping above `stable - 3`.

### When to migrate

- Stay on 26.0.x if : your project is a forked browser-side renderer using
  `Surface::get_current_texture().unwrap()` patterns extensively.
- Move to 29 if : you start fresh, want WebGPU-spec-correct error handling,
  or need the new `wgpu_int16`, mesh-shader DX12 support, AABB BLAS, or
  per-vertex Metal/DX12 features.

A migration commit looks like ~15 file changes for a small renderer
(`bind_group_layouts`, `buffers`, `present` call sites, the surface-texture
match). Plan one PR, run `cargo check` per platform.
