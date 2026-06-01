import { LitElement, html, css } from "lit";
import { customElement, property, query } from "lit/decorators.js";

// Type-only imports to preserve compile-time safety without loading the modules during SSR
import type * as THREE_TYPE from "three/webgpu";
import type { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { M3Theme3D } from "../3d/internal/M3Theme3D.js";

/**
 * A Lit Web Component that initializes a WebGPURenderer (with WebGL2 fallback),
 * handles responsive resizing, sets up basic controls, and bridges M3 Theme variables.
 *
 * Fully SSR-safe via runtime dynamic imports.
 */
@customElement("md-3d-canvas")
export class M3Canvas3D extends LitElement {
  static override styles = css`
    :host {
      display: block;
      width: 100%;
      height: 100%;
      position: relative;
      overflow: hidden;
      border-radius: inherit;
    }
    canvas {
      display: block;
      width: 100%;
      height: 100%;
    }
  `;

  @query("canvas")
  protected canvasEl!: HTMLCanvasElement;

  // Public properties resolved on the client at runtime
  public THREE!: typeof THREE_TYPE;
  public renderer!: THREE_TYPE.WebGPURenderer;
  public scene!: THREE_TYPE.Scene;
  public camera!: THREE_TYPE.PerspectiveCamera;
  public controls!: OrbitControls;
  public theme!: M3Theme3D;

  private resizeObserver: ResizeObserver | null = null;
  private animationFrameId: number | null = null;

  @property({ type: Boolean, attribute: "enable-controls" })
  public enableControls = true;

  @property({ type: Boolean, attribute: "auto-rotate" })
  public autoRotate = false;

  override firstUpdated() {
    this.initThree();
  }

  protected async initThree() {
    // 1. Asynchronously load heavy browser-only dependencies
    const THREE = await import("three/webgpu");
    const { OrbitControls } = await import("three/examples/jsm/controls/OrbitControls.js");
    const { uniform } = await import("three/tsl");
    const { M3Theme3D } = await import("../3d/internal/M3Theme3D.js");

    this.THREE = THREE;

    // 2. Scene setup
    this.scene = new THREE.Scene();

    // 3. Camera setup
    const rect = this.getBoundingClientRect();
    const aspect = rect.width / (rect.height || 1);
    this.camera = new THREE.PerspectiveCamera(50, aspect, 0.1, 100);
    this.camera.position.set(0, 0, 5);

    // 4. Renderer setup
    this.renderer = new THREE.WebGPURenderer({
      canvas: this.canvasEl,
      antialias: true,
      alpha: true,
    });

    try {
      await this.renderer.init();
    } catch (e) {
      console.warn("WebGPU initialization failed. Falling back to WebGL2 backend.", e);
    }

    this.renderer.setSize(rect.width, rect.height);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    // 5. Initialize M3 Theme Bridge (injecting Three dependencies)
    this.theme = new M3Theme3D(THREE, uniform, this);

    // 6. Controls setup
    this.controls = new OrbitControls(this.camera, this.canvasEl);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.05;
    this.controls.enabled = this.enableControls;
    this.controls.autoRotate = this.autoRotate;

    // 7. Handle resizing dynamically
    this.resizeObserver = new ResizeObserver((entries) => {
      if (!entries || entries.length === 0) return;
      const { width, height } = entries[0].contentRect;
      this.onResize(width, height);
    });
    this.resizeObserver.observe(this);

    // 8. Start Tick Loop
    this.tick();

    // Dispatch initialized event
    this.dispatchEvent(
      new CustomEvent("md-3d-init", {
        detail: {
          canvas: this,
          scene: this.scene,
          camera: this.camera,
          renderer: this.renderer,
          theme: this.theme,
        },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private onResize(width: number, height: number) {
    if (!this.camera || !this.renderer) return;

    this.camera.aspect = width / (height || 1);
    this.camera.updateProjectionMatrix();

    this.renderer.setSize(width, height);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  }

  private tick = () => {
    this.animationFrameId = requestAnimationFrame(this.tick);

    if (this.controls && this.enableControls) {
      this.controls.update();
    }

    // Call custom render function hook or dispatch event
    this.onTick();

    if (this.renderer && this.scene && this.camera) {
      this.renderer.renderAsync(this.scene, this.camera);
    }
  };

  /**
   * Overridable hook for subclass ticking or frame mutations.
   */
  protected onTick() {
    this.dispatchEvent(
      new CustomEvent("md-3d-tick", {
        bubbles: false,
      }),
    );
  }

  override disconnectedCallback() {
    super.disconnectedCallback();

    if (this.animationFrameId) {
      cancelAnimationFrame(this.animationFrameId);
    }
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
    if (this.theme) {
      this.theme.dispose();
    }
    if (this.controls) {
      this.controls.dispose();
    }
    if (this.renderer) {
      this.renderer.dispose();
    }
  }

  override render() {
    return html`<canvas></canvas>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    "md-3d-canvas": M3Canvas3D;
  }
}
