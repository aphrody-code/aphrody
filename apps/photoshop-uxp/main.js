// SPDX-License-Identifier: Apache-2.0
// aphrody — Photoshop UXP bridge panel.
//
// Connects to the aphrody-mcp local WebSocket bridge (ws://localhost:8765) and
// executes commands inside the running Photoshop, exposing the *entire* app
// surface from the inside:
//   - op "info"      → app version + active document + layer tree
//   - op "batchPlay" → action.batchPlay(commands, options)  (universal driver)
//   - op "eval"      → run arbitrary UXP JS with the full API in scope
//
// Protocol (JSON text frames):
//   aphrody → panel : { id, op, args }
//   panel → aphrody : { id, ok, result }  |  { id, ok:false, error }

const photoshop = require("photoshop");
const { app, action, core, constants } = photoshop;
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

const WS_URL = "ws://localhost:8765";
const RECONNECT_MS = 2000;

let ws = null;
let manualReconnect = false;

const statusEl = document.getElementById("status");
const logEl = document.getElementById("log");

function setStatus(text, cls) {
  statusEl.textContent = text;
  statusEl.className = cls;
}

function log(line) {
  const ts = new Date().toISOString().slice(11, 19);
  logEl.textContent = `[${ts}] ${line}\n` + logEl.textContent;
  if (logEl.textContent.length > 8000) {
    logEl.textContent = logEl.textContent.slice(0, 8000);
  }
}

function send(obj) {
  if (ws && ws.readyState === 1) {
    ws.send(JSON.stringify(obj));
  }
}

function connect() {
  setStatus("connecting…", "wait");
  try {
    ws = new WebSocket(WS_URL);
  } catch (e) {
    setStatus("disconnected", "bad");
    scheduleReconnect();
    return;
  }

  ws.onopen = () => {
    setStatus("Connected", "ok");
    log("connected to aphrody-mcp bridge");
  };

  ws.onclose = () => {
    setStatus("disconnected", "bad");
    scheduleReconnect();
  };

  ws.onerror = () => {
    // onclose follows; keep the panel quiet to avoid log spam while idle.
  };

  ws.onmessage = async (event) => {
    let req;
    try {
      req = JSON.parse(event.data);
    } catch (e) {
      return;
    }
    const { id, op, args } = req;
    log(`▶ ${op}`);
    try {
      const result = await handle(op, args || {});
      send({ id, ok: true, result });
      log(`✓ ${op}`);
    } catch (e) {
      const error = String((e && e.message) || e);
      send({ id, ok: false, error });
      log(`✗ ${op}: ${error}`);
    }
  };
}

function scheduleReconnect() {
  if (manualReconnect) return;
  setTimeout(connect, RECONNECT_MS);
}

async function handle(op, args) {
  switch (op) {
    case "info":
      return infoOp();
    case "batchPlay":
      return batchPlayOp(args);
    case "eval":
      return evalOp(args);
    default:
      throw new Error(`unknown op: ${op}`);
  }
}

// Read-only: no modal scope required.
function infoOp() {
  const info = {
    version: app.version,
    documentCount: app.documents.length,
  };
  const doc = app.activeDocument;
  if (doc) {
    info.activeDocument = {
      title: doc.title,
      width: doc.width,
      height: doc.height,
      resolution: doc.resolution,
      mode: String(doc.mode),
      layers: doc.layers.map((l) => ({
        id: l.id,
        name: l.name,
        kind: String(l.kind),
        visible: l.visible,
        opacity: l.opacity,
      })),
    };
  }
  return info;
}

// Universal driver — any Photoshop op as an ActionDescriptor array.
async function batchPlayOp(args) {
  const commands = args.commands || [];
  const options = args.options || {};
  let out;
  await core.executeAsModal(
    async () => {
      out = await action.batchPlay(commands, options);
    },
    { commandName: "aphrody batchPlay" }
  );
  return out;
}

// Full escape hatch — arbitrary UXP JS with the API in scope.
async function evalOp(args) {
  const fn = new AsyncFunction(
    "app",
    "photoshop",
    "constants",
    "core",
    "batchPlay",
    args.code
  );
  let out;
  await core.executeAsModal(
    async () => {
      out = await fn(app, photoshop, constants, core, action.batchPlay.bind(action));
    },
    { commandName: "aphrody eval" }
  );
  return out === undefined ? null : out;
}

document.getElementById("reconnect").addEventListener("click", () => {
  manualReconnect = true;
  if (ws) {
    try {
      ws.close();
    } catch (e) {
      /* ignore */
    }
  }
  manualReconnect = false;
  connect();
});

connect();
