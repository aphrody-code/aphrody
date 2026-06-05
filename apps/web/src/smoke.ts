// Headless smoke test: boots the real Bun server (which bundles the React app on
// the fly), then asserts the HTML shell, the bundled JS, the mock API, and the SSE
// completion stream. No browser. Exits non-zero on any failure.



const PORT = "3219";
const BASE = `http://localhost:${PORT}`;

const proc = Bun.spawn(["bun", "src/server.ts"], {
  env: { ...process.env, PORT, NODE_ENV: "development" },
  stdout: "pipe",
  stderr: "pipe",
});

const TOKEN = process.env.WEB_APP_TOKEN ?? "m7K2p9Q4x1R8w5Z3";
const headers = { "authorization": `Bearer ${TOKEN}` };

async function waitReady(timeoutMs = 20_000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(`${BASE}/api/config`, { headers });
      if (r.ok) return;
    } catch {
      /* not up yet */
    }
    await Bun.sleep(150);
  }
  throw new Error("server did not become ready");
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(`assertion failed: ${msg}`);
}

let failed = false;
try {
  await waitReady();

  // 1. HTML shell + bundled module script
  const html = await (await fetch(`${BASE}/`)).text();
  assert(html.includes('<div id="root">'), "HTML serves the React root");
  assert(/<script[^>]+type="module"/.test(html), "HTML references a module script");
  console.log("ok  HTML shell + bundle entry");

  // 2. Bundled JS actually builds & serves (find the emitted script src, fetch it)
  const srcMatch = html.match(/<script[^>]+src="([^"]+\.js)"/);
  assert(srcMatch, "bundled JS script src present");
  const js = await fetch(`${BASE}${srcMatch![1]}`);
  assert(js.ok, "bundled JS is served (build succeeded)");
  const jsText = await js.text();
  assert(jsText.length > 1000, "bundled JS is non-trivial");
  console.log("ok  React/TanStack/m3-react bundle compiled");

  // 3. Mock API surface
  const config = (await (await fetch(`${BASE}/api/config`, { headers })).json()) as { name: string };
  assert(config.name, "config endpoint returns a name");
  const chats = (await (await fetch(`${BASE}/api/chats`, { headers })).json()) as unknown[];
  assert(Array.isArray(chats) && chats.length > 0, "chats endpoint returns seeded list");
  const models = (await (await fetch(`${BASE}/api/models`, { headers })).json()) as unknown[];
  assert(Array.isArray(models) && models.length > 0, "models endpoint returns list");
  console.log("ok  mock API (config / chats / models)");

  // 4. SSE chat completion streams deltas
  const res = await fetch(`${BASE}/api/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify({
      model: "shenron",
      messages: [{ role: "user", content: "hi" }],
      stream: true,
    }),
  });
  assert(
    res.headers.get("content-type")?.includes("text/event-stream"),
    "completion is an SSE stream",
  );
  const body = await res.text();
  assert(body.includes("data:") && body.includes("[DONE]"), "stream emits deltas and a terminator");
  console.log("ok  SSE chat completion stream");

  console.log("\nSMOKE PASS");
} catch (err) {
  failed = true;
  console.error("\nSMOKE FAIL:", err instanceof Error ? err.message : err);
} finally {
  proc.kill();
}

process.exit(failed ? 1 : 0);
