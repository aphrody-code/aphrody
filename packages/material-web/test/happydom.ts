import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();
// happy-dom lacks ElementInternals — load the polyfill AFTER registering the
// DOM globals (dynamic import so it runs post-register, not hoisted before).
await import("element-internals-polyfill");
