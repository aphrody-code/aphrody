import { customElement, property } from "lit/decorators.js";
import { M3Canvas3D } from "../canvas3d/canvas-3d.js";

/**
 * A beautiful WebGPU-powered 3D Particle Globe component.
 * Uses procedural TSL shaders to warp particle positions and color-blend
 * dynamically based on Material Design 3 theme colors.
 *
 * Fully SSR-safe via dynamic shader node resolution.
 */
@customElement("md-3d-globe")
export class M3Globe3D extends M3Canvas3D {
  @property({ type: Number, attribute: "particle-count" })
  public particleCount = 2000;

  @property({ type: Number })
  public speed = 1.0;

  private points!: any; // THREE.Points

  protected override async initThree() {
    // 1. Initialize core Three engine via parent class
    await super.initThree();

    const THREE = this.THREE;

    // 2. Setup camera starting position and autoRotate
    this.camera.position.set(0, 0, 4);
    this.controls.autoRotate = this.autoRotate;
    this.controls.autoRotateSpeed = 1.0;

    // 3. Create geometry (distribute points on a sphere)
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array(this.particleCount * 3);

    for (let i = 0; i < this.particleCount; i++) {
      const u = Math.random();
      const v = Math.random();
      const theta = u * 2.0 * Math.PI;
      const phi = Math.acos(2.0 * v - 1.0);

      const r = 1.2; // Sphere radius
      const x = r * Math.sin(phi) * Math.cos(theta);
      const y = r * Math.sin(phi) * Math.sin(theta);
      const z = r * Math.cos(phi);

      positions[i * 3] = x;
      positions[i * 3 + 1] = y;
      positions[i * 3 + 2] = z;
    }

    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));

    // 4. Dynamically import TSL nodes to avoid server-side execution crashes
    const { positionLocal, time, mix, uv } = await import("three/tsl");

    // 5. Create procedural TSL material
    const material = new THREE.PointsNodeMaterial({
      size: 0.15,
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });

    // Vertex animation: create a wave pulse on the sphere radius
    const waveFreq = 2.5;
    const waveAmp = 0.08;
    const pulseFactor = time.mul(this.speed).mul(waveFreq);
    const wave = positionLocal.y.mul(2.0).add(pulseFactor).sin().mul(waveAmp);

    // Offset local position along the normal direction (outward from sphere center)
    material.positionNode = positionLocal.add(positionLocal.normalize().mul(wave));

    // Fragment color: Blend dynamic Primary color and dynamic Tertiary color
    // Blend changes along the vertical axis and shifts over time
    const colorMixRatio = positionLocal.y
      .add(time.mul(this.speed * 0.4).sin())
      .add(1.0)
      .mul(0.5)
      .clamp(0.0, 1.0);

    material.colorNode = mix(this.theme.primary, this.theme.tertiary, colorMixRatio);

    // Procedural round particle mask with soft glowing edges
    const distToCenter = uv().sub(0.5).length();
    const alphaMask = distToCenter.sub(0.5).negate().mul(2.0).clamp(0.0, 1.0);

    material.opacityNode = alphaMask;

    // 6. Instantiation
    this.points = new THREE.Points(geometry, material);
    this.scene.add(this.points);

    // Add dynamic ambient lights for atmosphere
    const light = new THREE.AmbientLight(0xffffff, 0.5);
    this.scene.add(light);
  }

  protected override onTick() {
    super.onTick();
    if (this.points && !this.enableControls) {
      // Manual slow rotation when controls are disabled
      this.points.rotation.y += 0.003 * this.speed;
      this.points.rotation.x += 0.001 * this.speed;
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "md-3d-globe": M3Globe3D;
  }
}
