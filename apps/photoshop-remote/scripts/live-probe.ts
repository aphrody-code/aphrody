#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
//! Live feature probe — exercises a broad slice of the Photoshop ExtendScript
//! API against a running Photoshop via Remote Connections, and reports each
//! result. Read probes are non-destructive; the single write workflow operates
//! on a throwaway document it creates and closes (never the user's open doc).
//!
//!   PS_HOST=<host> PS_PASSWORD=<pw> bun scripts/live-probe.ts
//!
//! Credentials come from the environment only.

import { PhotoshopRemote } from "../src/client";

const host = process.env.PS_HOST;
const password = process.env.PS_PASSWORD;
if (!host || !password) {
  console.error("set PS_HOST and PS_PASSWORD in the environment");
  process.exit(2);
}

interface Probe {
  name: string;
  jsx: string;
}

// --- Read probes (safe, non-destructive) ----------------------------------
const reads: Probe[] = [
  { name: "app.version", jsx: "app.version" },
  { name: "app.name", jsx: "app.name" },
  { name: "app.path", jsx: "app.path.fsName" },
  { name: "app.documents.length", jsx: "app.documents.length.toString()" },
  { name: "app.fonts.length", jsx: "app.fonts.length.toString()" },
  { name: "app.preferences.rulerUnits", jsx: "app.preferences.rulerUnits.toString()" },
  { name: "app.foregroundColor", jsx: "app.foregroundColor.rgb.hexValue" },
  { name: "extendscript.$.version", jsx: "$.version" },
  { name: "extendscript.$.os", jsx: "$.os" },
  { name: "extendscript.$.locale", jsx: "$.locale" },
  { name: "activeDocument.name", jsx: "app.activeDocument.name" },
  {
    name: "activeDocument.size",
    jsx: "app.activeDocument.width.value + 'x' + app.activeDocument.height.value",
  },
  { name: "activeDocument.resolution", jsx: "app.activeDocument.resolution.toString()" },
  { name: "activeDocument.mode", jsx: "app.activeDocument.mode.toString()" },
  { name: "activeDocument.bitsPerChannel", jsx: "app.activeDocument.bitsPerChannel.toString()" },
  { name: "activeDocument.channels", jsx: "app.activeDocument.channels.length.toString()" },
  { name: "activeDocument.historyStates", jsx: "app.activeDocument.historyStates.length.toString()" },
  {
    name: "activeDocument.layers",
    jsx:
      "(function(){var s=[];var L=app.activeDocument.layers;for(var i=0;i<L.length;i++)" +
      "s.push(L[i].name+':'+L[i].typename);return s.join(', ');})()",
  },
  {
    name: "activeDocument.histogram",
    jsx: "app.activeDocument.histogram.length.toString()",
  },
  {
    name: "actionManager.executeActionGet",
    jsx:
      "(function(){var r=new ActionReference();" +
      "r.putProperty(charIDToTypeID('Prpr'),charIDToTypeID('Ttl '));" +
      "r.putEnumerated(charIDToTypeID('Dcmn'),charIDToTypeID('Ordn'),charIDToTypeID('Trgt'));" +
      "var d=executeActionGet(r);return 'AM title='+d.getString(charIDToTypeID('Ttl '));})()",
  },
  {
    name: "scripting.listFiles",
    jsx: "(function(){return Folder.appData.fsName;})()",
  },
];

// --- Write workflow (atomic, isolated to a throwaway doc) ------------------
const writeWorkflow: Probe = {
  name: "write-workflow(new doc→layer→fill→histogram→flatten→resize→close)",
  jsx: `(function(){
    var created=false, doc=null, log=[];
    try {
      doc = app.documents.add(200,200,72,"aphrody_probe_"+(new Date().getTime()),NewDocumentMode.RGB);
      created=true; log.push("add="+doc.name);
      var lyr = doc.artLayers.add(); lyr.name="probe"; log.push("layers="+doc.layers.length);
      doc.selection.selectAll();
      var c=new SolidColor(); c.rgb.red=255; c.rgb.green=64; c.rgb.blue=0;
      doc.selection.fill(c); doc.selection.deselect(); log.push("fill=ok");
      log.push("hist="+doc.histogram.length);
      doc.flatten(); log.push("flatten=ok");
      doc.resizeImage(UnitValue(100,"px"),UnitValue(100,"px"));
      log.push("resize="+doc.width.value+"x"+doc.height.value);
    } catch(e) { log.push("ERR="+e.toString()); }
    finally {
      if (created && doc) { try { doc.close(SaveOptions.DONOTSAVECHANGES); log.push("closed; docs="+app.documents.length); } catch(e2) { log.push("closeERR="+e2); } }
    }
    return log.join(" | ");
  })()`,
};

const probes = [...reads, writeWorkflow];

const ps = new PhotoshopRemote({ host, password });
await ps.connect();

let pass = 0;
let fail = 0;
for (const p of probes) {
  try {
    const r = await ps.exec(p.jsx, { timeoutMs: 30_000 });
    const ok = !r.isError && !/^ERR=|ERR=/.test(r.text);
    const text = r.text.length > 90 ? r.text.slice(0, 90) + "…" : r.text;
    console.log(`${ok ? "✓" : "✗"} ${p.name.padEnd(48)} → ${text}`);
    ok ? pass++ : fail++;
  } catch (e) {
    console.log(`✗ ${p.name.padEnd(48)} → THREW ${String((e as Error).message ?? e)}`);
    fail++;
  }
}
ps.close();

console.log(`\n${pass}/${probes.length} probes passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
