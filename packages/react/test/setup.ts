// Preloaded by bun (see bunfig.toml) before any test module. Registers a
// happy-dom environment so React 19 can render into a real `document`, plus an
// ElementInternals polyfill (happy-dom lacks attachInternals, which the
// form-associated md-* elements need). We drive rendering with `flushSync`
// (not React Testing Library) to keep the dependency surface minimal; signal we
// are NOT in an act() environment to silence the spurious warning.
import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();
// Load AFTER register (dynamic import so it runs post-DOM-globals, not hoisted).
await import("element-internals-polyfill");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = false;
