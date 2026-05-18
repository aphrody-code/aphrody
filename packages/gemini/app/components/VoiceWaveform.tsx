// SPDX-License-Identifier: Apache-2.0
"use client";

import { useEffect, useRef, type JSX } from "react";

interface VoiceWaveformProps {
  analyser: AnalyserNode | null;
  width?: number;
  height?: number;
}

/**
 * Live PCM-time-domain waveform driven by a Web Audio AnalyserNode.
 * Rendered onto a 2D canvas, requestAnimationFrame loop. Gracefully
 * paints a flat line if no analyser is attached yet.
 */
export function VoiceWaveform({
  analyser,
  width = 120,
  height = 36,
}: VoiceWaveformProps): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let raf = 0;
    const bins = analyser?.fftSize ?? 256;
    const data = new Uint8Array(bins);

    const draw = (): void => {
      ctx.clearRect(0, 0, width, height);
      if (analyser) {
        analyser.getByteTimeDomainData(data);
      } else {
        data.fill(128);
      }
      const grad = ctx.createLinearGradient(0, 0, width, 0);
      grad.addColorStop(0, "#4285F4");
      grad.addColorStop(0.5, "#9168C0");
      grad.addColorStop(1, "#EC4899");
      ctx.strokeStyle = grad;
      ctx.lineWidth = 2;
      ctx.beginPath();
      const sliceWidth = width / data.length;
      let x = 0;
      for (let i = 0; i < data.length; i += 1) {
        const sample = (data[i] ?? 128) / 128;
        const y = (sample * height) / 2;
        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
        x += sliceWidth;
      }
      ctx.lineTo(width, height / 2);
      ctx.stroke();
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [analyser, width, height]);

  return (
    <canvas
      ref={canvasRef}
      className="voice-waveform"
      width={width}
      height={height}
      aria-hidden="true"
    />
  );
}
