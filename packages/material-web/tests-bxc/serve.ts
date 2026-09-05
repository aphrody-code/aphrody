const root: string = process.argv[2] ?? ".";
const srv = Bun.serve({
  port: 8799,
  async fetch(req: Request): Promise<Response> {
    let p = new URL(req.url).pathname;
    if (p === "/") p = "/tests-bxc/index.html";
    const f = Bun.file(root + p);
    if (await f.exists()) return new Response(f);
    return new Response("404", { status: 404 });
  },
});
console.log("serving", root, "on", srv.port);
