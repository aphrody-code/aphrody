// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Helpers GPU/WebGPU/WGSL pour `n2b bin --flavor webgpu`.
//!
//! Séparé de `bin_cmd.rs` pour limiter la taille du fichier principal.
//! Tout ce module est `pub(super)` — il n'expose rien en dehors de `bin_cmd`.

use anyhow::Result;

use super::write_file;

// ---------------------------------------------------------------------------
// Scaffold WebGPU
// ---------------------------------------------------------------------------

pub(super) fn scaffold_webgpu(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(
        dir.join("Cargo.toml"),
        &render_webgpu_cargo_toml(name),
        quiet,
    )?;
    write_file(dir.join("src/lib.rs"), WEBGPU_LIB_RS, quiet)?;
    write_file(dir.join("src/compute.wgsl"), COMPUTE_WGSL, quiet)?;
    write_file(
        dir.join("package.json"),
        &render_webgpu_package_json(name),
        quiet,
    )?;
    write_file(dir.join("index.ts"), INDEX_TS_WEBGPU, quiet)?;
    write_file(
        dir.join("README.md"),
        &super::render_readme(
            name,
            "Rust + wgpu → WASM → Bun/WebGPU compute pipeline (WGSL inclus)",
        ),
        quiet,
    )?;
    write_file(dir.join(".gitignore"), super::GITIGNORE_WASM, quiet)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Renderers Cargo.toml / package.json
// ---------------------------------------------------------------------------

pub(super) fn render_webgpu_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
description = "wgpu (WebGPU) compute module for Bun via WASM"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wgpu = {{ version = "29", features = ["webgpu"] }}
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
bytemuck = {{ version = "1", features = ["derive"] }}
futures-channel = "0.3"
log = "0.4"

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = {{ version = "0.3", features = ["Window", "Document", "Navigator", "Gpu"] }}
console_error_panic_hook = "0.1"

[profile.release]
lto = true
codegen-units = 1
opt-level = "s"
"#,
    )
}

pub(super) fn render_webgpu_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "description": "wgpu compute WASM for Bun",
  "type": "module",
  "main": "index.ts",
  "scripts": {{
    "build": "wasm-pack build --target web --out-dir pkg -- --features webgpu",
    "build:bundler": "wasm-pack build --target bundler --out-dir pkg-bundler",
    "test": "bun test"
  }},
  "devDependencies": {{
    "@types/bun": "latest"
  }},
  "engines": {{
    "bun": ">=1.2.0"
  }}
}}
"#,
    )
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

pub(super) const WEBGPU_LIB_RS: &str = r#"//! Rust + wgpu → WASM compute pipeline for Bun.
//!
//! Shader : src/compute.wgsl (WGSL — voir https://gpuweb.github.io/gpuweb/wgsl/)
//! Runtime: navigator.gpu côté Bun (WebGPU est exposé dans Bun et Electrobun).
//!
//! Docs :
//!   - https://docs.rs/wgpu
//!   - https://gpuweb.github.io/gpuweb/
//!   - https://blackboard.sh/electrobun/docs/apis/webgpu/

use wasm_bindgen::prelude::*;

const SHADER: &str = include_str!("compute.wgsl");

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = wasm_bindgen::throw_str;
    }
    let _ = log::set_max_level;
}

/// Double chaque élément du buffer d'entrée via un compute shader WGSL.
/// Retourne un `Vec<u32>` de même taille.
#[wasm_bindgen]
pub async fn double_u32(input: Vec<u32>) -> Result<Vec<u32>, JsValue> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .ok_or_else(|| JsValue::from_str("no wgpu adapter"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .map_err(|e| JsValue::from_str(&format!("request_device: {e:?}")))?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("double.wgsl"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let size = (input.len() * std::mem::size_of::<u32>()) as u64;

    let storage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("storage"),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    queue.write_buffer(&storage, 0, bytemuck::cast_slice(&input));

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(input.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&storage, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = futures_channel::oneshot::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.await
        .map_err(|e| JsValue::from_str(&format!("map canceled: {e}")))?
        .map_err(|e| JsValue::from_str(&format!("map: {e:?}")))?;

    let data = slice.get_mapped_range();
    let out: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging.unmap();
    Ok(out)
}
"#;

pub(super) const COMPUTE_WGSL: &str = r#"// WGSL compute shader — double chaque u32 du storage buffer.
// Spec : https://gpuweb.github.io/gpuweb/wgsl/

@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let idx = gid.x;
  if (idx < arrayLength(&data)) {
    data[idx] = data[idx] * 2u;
  }
}
"#;

pub(super) const INDEX_TS_WEBGPU: &str = r#"// Bun loader for the wgpu+WASM compute module.
// Requires Bun >= 1.2 (WebGPU exposed via navigator.gpu) or Electrobun.

import init, * as wasm from "./pkg/index.js";

if (typeof navigator === "undefined" || !("gpu" in navigator)) {
  throw new Error(
    "WebGPU not available — run with `bun --enable-webgpu` or use Electrobun.",
  );
}

await init();

/** Double each element of `buf` via a WGSL compute shader. */
export async function double(buf: Uint32Array): Promise<Uint32Array> {
  // @ts-expect-error — wasm-bindgen generates async signatures
  const out = await wasm.double_u32(buf);
  return new Uint32Array(out);
}
"#;
