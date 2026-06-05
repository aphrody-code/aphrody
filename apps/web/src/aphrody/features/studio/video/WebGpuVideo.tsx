// SPDX-License-Identifier: Apache-2.0
import React, { useEffect, useRef, useState } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";

const shaderWGSL = `
struct VertexOutput {
  @builtin(position) Position : vec4f,
  @location(0) uv : vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) VertexIndex : u32) -> VertexOutput {
  var pos = array<vec2f, 4>(
    vec2f(-1.0, -1.0),
    vec2f( 1.0, -1.0),
    vec2f(-1.0,  1.0),
    vec2f( 1.0,  1.0)
  );

  var uv = array<vec2f, 4>(
    vec2f(0.0, 1.0),
    vec2f(1.0, 1.0),
    vec2f(0.0, 0.0),
    vec2f(1.0, 0.0)
  );

  var output : VertexOutput;
  output.Position = vec4f(pos[VertexIndex], 0.0, 1.0);
  output.uv = uv[VertexIndex];
  return output;
}

@group(0) @binding(0) var<uniform> frame : f32;
@group(0) @binding(1) var<uniform> resolution : vec2f;

@fragment
fn fs_main(@location(0) uv : vec2f) -> @location(0) vec4f {
  let t = frame * 0.02;
  let p = uv * 2.0 - vec2f(1.0);
  let d = length(p);
  
  var color = vec3f(0.0);
  
  let r = 0.5 + 0.5 * sin(t + uv.x * 3.0 + uv.y * 2.0);
  let g = 0.5 + 0.5 * cos(t + uv.y * 4.0 - d);
  let b = 0.5 + 0.5 * sin(t + d * 5.0);
  
  color = vec3f(r, g, b);
  
  let glow = 0.05 / abs(d - (0.4 + 0.1 * sin(t * 2.0)));
  color += vec3f(glow * 0.5, glow * 0.2, glow * 0.9);

  return vec4f(color, 1.0);
}
`;

export function WebGpuVideo() {
  const frame = useCurrentFrame();
  const { width, height } = useVideoConfig();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  
  const [device, setDevice] = useState<any>(null);
  const [pipeline, setPipeline] = useState<any>(null);
  const [uniformBufferFrame, setUniformBufferFrame] = useState<any>(null);
  const [bindGroup, setBindGroup] = useState<any>(null);
  const [context, setContext] = useState<any>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    async function initWebGpu() {
      const nav = navigator as any;
      if (!nav.gpu) {
        setErrorMsg("WebGPU is not supported by your browser or environment.");
        return;
      }

      try {
        const adapter = await nav.gpu.requestAdapter();
        if (!adapter) {
          setErrorMsg("No GPU adapter found.");
          return;
        }
        const gpuDevice = await adapter.requestDevice();
        setDevice(gpuDevice);

        const canvas = canvasRef.current;
        if (!canvas) return;

        const gpuContext = canvas.getContext("webgpu") as any;
        if (!gpuContext) {
          setErrorMsg("Could not get WebGPU context.");
          return;
        }
        const format = nav.gpu.getPreferredCanvasFormat();
        
        gpuContext.configure({
          device: gpuDevice,
          format,
          alphaMode: "opaque",
        });
        setContext(gpuContext);

        const shaderModule = gpuDevice.createShaderModule({ code: shaderWGSL });
        const renderPipeline = gpuDevice.createRenderPipeline({
          layout: "auto",
          vertex: {
            module: shaderModule,
            entryPoint: "vs_main",
          },
          fragment: {
            module: shaderModule,
            entryPoint: "fs_main",
            targets: [{ format }],
          },
          primitive: {
            topology: "triangle-strip",
          },
        });
        setPipeline(renderPipeline);

        const frameBuffer = gpuDevice.createBuffer({
          size: 4,
          usage: 0x0040 | 0x0008, // GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        setUniformBufferFrame(frameBuffer);

        const resBuffer = gpuDevice.createBuffer({
          size: 8,
          usage: 0x0040 | 0x0008, // GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST
        });
        gpuDevice.queue.writeBuffer(resBuffer, 0, new Float32Array([width, height]));

        const group = gpuDevice.createBindGroup({
          layout: renderPipeline.getBindGroupLayout(0),
          entries: [
            { binding: 0, resource: { buffer: frameBuffer } },
            { binding: 1, resource: { buffer: resBuffer } },
          ],
        });
        setBindGroup(group);
      } catch (err: any) {
        setErrorMsg(`WebGPU Init Error: ${err.message || err}`);
      }
    }

    initWebGpu();
  }, [width, height]);

  useEffect(() => {
    if (!device || !pipeline || !uniformBufferFrame || !bindGroup || !context) return;

    try {
      device.queue.writeBuffer(uniformBufferFrame, 0, new Float32Array([frame]));

      const commandEncoder = device.createCommandEncoder();
      const textureView = context.getCurrentTexture().createView();
      
      const renderPass = commandEncoder.beginRenderPass({
        colorAttachments: [{
          view: textureView,
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        }],
      });

      renderPass.setPipeline(pipeline);
      renderPass.setBindGroup(0, bindGroup);
      renderPass.draw(4);
      renderPass.end();

      device.queue.submit([commandEncoder.finish()]);
    } catch (err) {
      console.error("WebGPU render error", err);
    }
  }, [frame, device, pipeline, uniformBufferFrame, bindGroup, context]);

  if (errorMsg) {
    return (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          background: "#1a1a2e",
          color: "#ff5f56",
          fontFamily: "sans-serif",
          padding: 20,
          textAlign: "center",
          boxSizing: "border-box",
        }}
      >
        <span style={{ fontSize: 48, marginBottom: 16 }}>⚠️</span>
        <div style={{ fontSize: 18, fontWeight: "bold", marginBottom: 8 }}>WebGPU Non Disponible</div>
        <div style={{ fontSize: 14, opacity: 0.8 }}>{errorMsg}</div>
      </div>
    );
  }

  return (
    <canvas 
      ref={canvasRef} 
      width={width} 
      height={height} 
      style={{ width: "100%", height: "100%", objectFit: "cover" }} 
    />
  );
}
