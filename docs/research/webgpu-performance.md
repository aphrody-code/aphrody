<!-- SPDX-License-Identifier: Apache-2.0 -->
# WebGPU / wgpu — ultra-performant rendering reference

For aphrody's WASM Web UI / WebGPU path (CLAUDE.md §2: Web UI = WASM Rust +
`wgpu`). Distilled from the WebGPU best-practices, the `wgpu` crate docs, and
the transferable D3D12 concepts (PSO / command lists / async compute) that the
WebGPU model mirrors.

## The model (wgpu, Rust)

`Instance` → `Adapter` (physical device) → `Device` (open connection) + `Queue`.
Resources: `Buffer`, `Texture`/`TextureView`, `Sampler`, `BindGroupLayout` +
`BindGroup`, `PipelineLayout`, `RenderPipeline`/`ComputePipeline`. Handles are
ref-counted (cloneable).

Render loop: `CommandEncoder` → `begin_render_pass` (`RenderPass`) → set
pipeline / bind groups / vertex+index buffers → draw → end → `encoder.finish()`
→ `Queue::submit([cmd_buf])`. wgpu inserts resource-state barriers
automatically (explicit in D3D12/Vulkan).

## The big perf levers (highest → lowest impact)

1. **Render bundles** — the headline feature. `RenderBundleEncoder` →
   `RenderBundle` (JS: `device.createRenderBundleEncoder({colorFormats,
   depthStencilFormat})` then `pass.executeBundles([bundle])`). Pre-record the
   draw commands once; replay every frame, **skipping JS↔GPU-process marshalling,
   IPC, validation and native translation**. Commands are validated *once at
   creation*. Best for largely-static geometry. Caveats: a bundle **resets all
   pipeline/bind-group/buffer state** (each must fully set its own state); it
   **cannot** set viewport/scissor, blend constants, stencil reference, run
   occlusion queries, or nest bundles.

2. **Instancing** — draw N copies in one call (`draw`/`draw_indexed` with an
   instance count; per-instance data in a vertex buffer with `step_mode =
   Instance`). Collapses thousands of draws into one.

3. **Indirect draws + GPU culling** — `draw_indirect` / `draw_indexed_indirect`
   read draw parameters from a GPU buffer. A **compute pass** does visibility
   culling and writes the indirect/draw-count buffer → zero CPU per-object cost.
   Combine with render bundles (bundles can record indirect draws) for
   compute-driven scenes (particles, large worlds).

4. **Pipeline reuse (the PSO lesson)** — build every `RenderPipeline` once at
   init, never per-frame; pipeline creation pre-bakes all dependent state
   (raster/blend/depth/shaders) so switching at draw time is cheap. **Sort draws
   by pipeline, then by bind group** to minimise state changes (each change
   costs driver validation/translation).

5. **Bind groups + dynamic offsets** — reuse `BindGroup`s; for per-draw
   uniforms use ONE large uniform/storage buffer and a **dynamic offset**
   (`set_bind_group(i, &bg, &[offset])`) instead of many small buffers/bind
   groups.

6. **Buffer uploads without stalls** — `mapped_at_creation: true` for one-shot
   init data; per-frame use `Queue::write_buffer` (or a staging-belt /
   ring-buffer of pre-mapped buffers) rather than map→write→unmap, which
   serialises CPU↔GPU. Never read back synchronously in the hot path.

7. **Interleave compute + render** — WebGPU has a single queue, but compute
   passes and render passes in one submission run back-to-back; offload skinning,
   particles, culling and post-processing to compute (the D3D12 "async compute"
   idea, expressed as compute passes here).

## aphrody fit

The 3D pipeline today renders via Blender/Cycles **OPTIX** (native GPU, see
[blender-api-notes.md](../python/blender-api-notes.md)). A *web* real-time
viewer (e.g. for the showcase page that currently uses `<model-viewer>`) would
be the `wgpu` target: load the generated `.glb`, build one `RenderPipeline`,
record a **render bundle** per static mesh, drive the turntable spin from a
small per-frame uniform (one `write_buffer`), and instance any repeated geometry.

## Sources
- WebGPU render bundles best practice — <https://toji.dev/webgpu-best-practices/render-bundles.html>
- `wgpu` crate docs — <https://docs.rs/wgpu/latest/wgpu/>
- D3D12 pipeline-state / command-list / async-compute concepts (transferable) — Microsoft Learn.
