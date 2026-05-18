# Bun Native APIs — Complete Reference

## HTTP Server (`Bun.serve()`)
```ts
const server = Bun.serve({
  port: 3000,
  routes: {
    "/": new Response("OK"),                           // static
    "/users/:id": (req) => Response.json(req.params),  // dynamic
    "/api/data": {                                     // per-method
      GET: () => Response.json({ items: [] }),
      POST: async (req) => Response.json(await req.json(), { status: 201 }),
    },
    "/api/*": Response.json({ error: "Not found" }, { status: 404 }), // wildcard
    "/file": Bun.file("./data.json"),                  // serve file
    "/old": Response.redirect("/new"),                 // redirect
  },
  websocket: {
    open(ws) { ws.send("hello"); },
    message(ws, msg) { ws.send(msg); },
    close(ws) {},
  },
  fetch(req, server) {                                 // fallback
    if (server.upgrade(req)) return;                   // upgrade to WS
    return new Response("Not Found", { status: 404 });
  },
  development: { hmr: true, console: true },
});
server.url;        // URL object
server.stop();     // graceful shutdown
server.reload({});  // hot reload routes
```

## File I/O (`Bun.file()` / `Bun.write()`)
```ts
const file = Bun.file("data.txt");
file.size;                    // bytes (no read)
file.type;                    // MIME type
await file.exists();          // boolean
await file.text();            // string
await file.json();            // parsed JSON
await file.arrayBuffer();     // ArrayBuffer
await file.bytes();           // Uint8Array
file.stream();                // ReadableStream
await file.delete();          // delete

// Slicing (lazy, zero-copy)
const slice = file.slice(0, 1000);
await slice.text();

// Write (returns bytes written)
await Bun.write("out.txt", "content");
await Bun.write("copy.txt", Bun.file("src.txt"));          // copy
await Bun.write("page.html", await fetch("https://..."));  // HTTP → file
await Bun.write(Bun.stdout, "to stdout\n");

// Incremental writer (buffered)
const w = Bun.file("log.txt").writer({ highWaterMark: 1024 * 1024 });
w.write("line 1\n"); w.write("line 2\n");
w.flush(); w.end();
```

## Shell (`$` from "bun")
```ts
import { $ } from "bun";

await $`echo hello`;                              // prints to stdout
const text = await $`echo hi`.text();             // capture → string
const json = await $`echo '{"a":1}'`.json();      // capture → JSON
const lines = await $`cat file.txt`.lines();      // async iterator
await $`cmd`.quiet();                             // suppress stdout
const { exitCode } = await $`cmd`.nothrow();      // no throw on error
await $`cmd`.cwd("/tmp");                         // working directory
await $`cmd`.env({ NODE_ENV: "prod" });           // env vars
await $`cmd`.timeout(30000);                      // timeout ms

// Safe interpolation (auto-escaped!)
const userInput = "file.txt; rm -rf /";
await $`ls ${userInput}`;  // safe: treated as single arg

// Env vars in commands
await $`FOO=bar bun -e 'console.log(process.env.FOO)'`;

// Pipes & redirections
await $`cat file.txt | grep pattern | wc -l`;
await $`echo data > output.txt`;
await $`command 2> errors.txt`;

// JavaScript interop
const response = await fetch("https://example.com");
await $`cat < ${response} | wc -c`;

// Global defaults
$.cwd("/app");
$.env({ NODE_ENV: "production" });
```

## Glob (`Bun.Glob`)
```ts
const glob = new Bun.Glob("**/*.ts");

// Scan filesystem
for await (const file of glob.scan({ cwd: ".", dot: false, onlyFiles: true })) {
  console.log(file);
}

// Match test (no I/O)
glob.match("src/index.ts");  // boolean
```

## Subprocess (`Bun.spawn()`)
```ts
// Async
const proc = Bun.spawn(["cmd", "arg"], {
  cwd: "/path",
  env: { ...process.env, KEY: "val" },
  stdout: "pipe",   // "inherit" | "ignore" | number | Bun.file()
  stderr: "pipe",
  stdin: "pipe",     // "inherit" | "ignore" | Blob | Response
  onExit(proc, exitCode, signal, error) {},
});
proc.pid;
await proc.exited;                                    // exit code
const output = await new Response(proc.stdout).text();
proc.kill();                                          // SIGTERM

// Sync
const result = Bun.spawnSync(["cmd"], { stdout: "pipe" });
result.exitCode;
result.stdout.toString();
```

## SQLite (`bun:sqlite`)
```ts
import { Database } from "bun:sqlite";

const db = new Database("app.db");       // or ":memory:"
db.run("PRAGMA journal_mode = WAL;");    // performance

// Create table
db.run(`CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  email TEXT UNIQUE
)`);

// Prepared statements (cached)
const insert = db.query("INSERT INTO users (name, email) VALUES ($name, $email)");
const result = insert.run({ $name: "Alice", $email: "alice@ex.com" });
result.lastInsertRowid;  // 1
result.changes;          // 1

// Query
db.query("SELECT * FROM users").all();                    // [{...}, ...]
db.query("SELECT * FROM users WHERE id = ?").get(1);      // {...} or null
db.query("SELECT name FROM users").values();              // [["Alice"], ...]

// Iterate (memory-efficient)
for (const row of db.query("SELECT * FROM users").iterate()) { ... }

// Map to class
class User { id: number; name: string; get display() { return `User: ${this.name}`; } }
db.query("SELECT * FROM users").as(User).all();

// Transactions (atomic)
const insertMany = db.transaction((users) => {
  for (const u of users) insert.run(u);
  return users.length;
});
insertMany([{ $name: "Bob", $email: "bob@ex.com" }]);
insertMany.immediate(users);  // BEGIN IMMEDIATE

db.close();
```

## Test Runner (`bun:test`)
```ts
import { test, expect, describe, mock, spyOn, beforeEach, afterEach } from "bun:test";

describe("math", () => {
  test("addition", () => { expect(1 + 1).toBe(2); });
  test("async", async () => { expect(await fetch("...")).toBeDefined(); });
  test.skip("todo", () => {});
  test.todo("implement later");
});

// Mocks
const fn = mock(() => 42);
fn(); fn();
expect(fn).toHaveBeenCalledTimes(2);
expect(fn.mock.results[0].value).toBe(42);

// Spy
const obj = { greet: (name: string) => `Hi ${name}` };
const spy = spyOn(obj, "greet");
obj.greet("Alice");
expect(spy).toHaveBeenCalledWith("Alice");

// Snapshots
expect({ a: 1 }).toMatchSnapshot();

// Run: bun test
// Coverage: bun test --coverage
```

## Bundler (`Bun.build()`)
```ts
const result = await Bun.build({
  entrypoints: ["./src/index.ts"],
  outdir: "./dist",
  target: "bun",         // "bun" | "browser" | "node"
  format: "esm",
  minify: true,
  sourcemap: "linked",   // "linked" | "inline" | "external" | "none"
  splitting: true,        // code splitting
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  plugins: [myPlugin],
  external: ["fsevents"],
});
if (!result.success) console.error(result.logs);

// Plugin API
const myPlugin: BunPlugin = {
  name: "my-plugin",
  setup(build) {
    build.onResolve({ filter: /\.css$/ }, (args) => ({ path: args.path }));
    build.onLoad({ filter: /\.css$/ }, async (args) => ({
      contents: await Bun.file(args.path).text(),
      loader: "css",
    }));
  },
};

// Compile to executable
await Bun.build({
  entrypoints: ["./cli.ts"],
  compile: { target: "bun-linux-x64", outfile: "./dist/mycli" },
});
```

## Hashing & Passwords
```ts
// Fast non-crypto (Wyhash)
Bun.hash("data");                // number
Bun.hash.wyhash("data");
Bun.hash.crc32("data");
Bun.hash.adler32("data");

// Crypto hash
const hasher = new Bun.CryptoHasher("sha256");
hasher.update("data");
hasher.digest("hex");            // string

// Passwords (bcrypt/argon2)
const hash = await Bun.password.hash("password", { algorithm: "bcrypt", cost: 12 });
const valid = await Bun.password.verify("password", hash);  // boolean
```

## Semver
```ts
Bun.semver.satisfies("1.2.3", "^1.0.0");  // true
Bun.semver.order("1.0.0", "2.0.0");       // -1, 0, or 1
```

## Color
```ts
Bun.color("red", "css");     // "red"
Bun.color("#ff0000", "ansi"); // ANSI escape
Bun.color("hsl(0, 100%, 50%)", "hex"); // "#ff0000"
```

## Sleep
```ts
await Bun.sleep(1000); // ms
```

## Misc
```ts
Bun.version;          // "1.3.12"
Bun.revision;         // git sha
Bun.main;             // entrypoint path
Bun.env.NODE_ENV;     // env access
Bun.which("node");    // resolve binary path
Bun.peek(promise);    // sync peek at promise state
Bun.deepEquals(a, b); // deep equality
Bun.inspect(obj);     // pretty print
```
