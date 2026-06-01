import { customElement } from "lit/decorators.js";
import type * as THREE_TYPE from "three/webgpu";
import { M3Canvas3D } from "../canvas3d/canvas-3d.js";

/**
 * An interactive 3D Glassmorphic Card component.
 * Tilts dynamically based on mouse position, reflects high-fidelity scene lights,
 * and updates its materials reactively using M3 theme color variables.
 *
 * Fully SSR-safe via runtime dynamic initialization.
 */
@customElement("md-3d-card")
export class M3Card3D extends M3Canvas3D {
  private cardGroup!: any; // THREE.Group
  private cardMesh!: any; // THREE.Mesh
  private glowMesh!: any; // THREE.Mesh
  private spotLight!: any; // THREE.PointLight

  private targetRotationX = 0;
  private targetRotationY = 0;

  protected override async initThree() {
    // 1. Initialize parent canvas
    await super.initThree();

    const THREE = this.THREE;

    // Disable default OrbitControls so we can handle custom mouse tilt
    this.controls.enabled = false;
    this.camera.position.set(0, 0, 3.5);

    // 2. Create card container group
    this.cardGroup = new THREE.Group();
    this.scene.add(this.cardGroup);

    // 3. Create physical card geometry (thin, rounded-like box)
    const geometry = new THREE.BoxGeometry(2.2, 1.4, 0.05);

    // 4. MeshPhysicalNodeMaterial for glassmorphism
    const material = new THREE.MeshPhysicalNodeMaterial({
      roughness: 0.15,
      metalness: 0.05,
      clearcoat: 1.0,
      clearcoatRoughness: 0.1,
      transmission: 0.5, // Semi-transparent glass
      thickness: 0.5,
      ior: 1.5,
      transparent: true,
      opacity: 0.9,
    });

    // Bind theme colors reactively to the material color node
    material.colorNode = this.theme.surfaceVariant;

    this.cardMesh = new THREE.Mesh(geometry, material);
    this.cardMesh.castShadow = true;
    this.cardGroup.add(this.cardMesh);

    // 5. Create a dynamic colored glow backing plane (colored shadow)
    const glowGeom = new THREE.PlaneGeometry(2.4, 1.6);
    const glowMat = new THREE.MeshBasicMaterial({
      transparent: true,
      opacity: 0.15,
      blending: THREE.AdditiveBlending,
      side: THREE.DoubleSide,
      depthWrite: false,
    });

    this.glowMesh = new THREE.Mesh(glowGeom, glowMat);
    this.glowMesh.position.z = -0.06;
    this.cardGroup.add(this.glowMesh);

    // 6. Setup complex lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
    this.scene.add(ambientLight);

    // Directional light for specular clearcoat highlights
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(5, 5, 5);
    this.scene.add(dirLight);

    // Dynamic colored point light behind the card to throw ambient coloring
    this.spotLight = new THREE.PointLight(new THREE.Color(), 3.0, 5.0);
    this.spotLight.position.set(0, 0, 0.5);
    this.cardGroup.add(this.spotLight);

    // 7. Event listeners for interactive mouse tilt
    this.canvasEl.addEventListener("mousemove", this.handleMouseMove);
    this.canvasEl.addEventListener("mouseleave", this.handleMouseLeave);
  }

  private handleMouseMove = (e: MouseEvent) => {
    const rect = this.canvasEl.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    const y = -((e.clientY - rect.top) / rect.height) * 2 + 1;

    // Set target rotations (clamped maximum rotation)
    this.targetRotationX = y * 0.35;
    this.targetRotationY = x * 0.35;
  };

  private handleMouseLeave = () => {
    this.targetRotationX = 0;
    this.targetRotationY = 0;
  };

  protected override onTick() {
    super.onTick();

    const THREE = this.THREE;

    // 1. Smoothly interpolate (lerp) rotations
    if (this.cardGroup) {
      this.cardGroup.rotation.x += (this.targetRotationX - this.cardGroup.rotation.x) * 0.1;
      this.cardGroup.rotation.y += (this.targetRotationY - this.cardGroup.rotation.y) * 0.1;
    }

    // 2. Synchronize lighting colors and glow backing with M3 Theme Primary color
    if (THREE && this.theme && this.glowMesh && this.spotLight) {
      const primaryColor = this.theme.primary.value;

      // Update basic material colors manually
      (this.glowMesh.material as THREE_TYPE.MeshBasicMaterial).color.copy(primaryColor);
      this.spotLight.color.copy(primaryColor);
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (this.canvasEl) {
      this.canvasEl.removeEventListener("mousemove", this.handleMouseMove);
      this.canvasEl.removeEventListener("mouseleave", this.handleMouseLeave);
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "md-3d-card": M3Card3D;
  }
}
