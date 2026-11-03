// The proxy of the public demo for e2e_llm_consenso.py: the real proxy (proxy/assist-proxy.js) with every host service in memory (proxy/adapters/memoria.js)
// and the fake provider of the proxy tests, behind a tiny HTTP server on 127.0.0.1. The test routes https://localhost:<port> here; the page's Origin is
// swapped for the site's (the proxy accepts only that) and given back in the CORS answer. GET /__vars?NAME=VALUE changes PHASE or ASSIST_OFF.
// Usage: node tests_e2e/finto_proxy.mjs PORT (MZLAB_LLM_WASM_DIR = the folder of the llm .wasm of the built site). Prints «ready» once listening.
import http from "node:http";
import { setup, ORIGIN } from "../proxy/test/helpers.mjs";

const port = Number(process.argv[2]);
const vars = { PHASE: "0" };
const proxy = await setup({ vars });
if (!proxy) {
  console.error("the llm .wasm is missing (MZLAB_LLM_WASM_DIR)");
  process.exit(3);
}

http
  .createServer(async (req, res) => {
    const url = new URL(req.url, "http://127.0.0.1");
    if (url.pathname === "/__vars") {
      for (const [k, v] of url.searchParams) vars[k] = v;
      res.end("{}");
      return;
    }
    const chunks = [];
    for await (const c of req) chunks.push(c);
    const page = req.headers.origin || "";
    const headers = new Headers();
    for (const [k, v] of Object.entries(req.headers)) if (typeof v === "string") headers.set(k, v);
    headers.set("origin", ORIGIN);
    const body = req.method === "POST" ? Buffer.concat(chunks) : undefined;
    const r = await proxy.handle(new Request(`https://localhost:${port}${url.pathname}`, { method: req.method, headers, body }), "203.0.113.7");
    const out = {};
    r.headers.forEach((v, k) => {
      out[k] = k === "access-control-allow-origin" ? page : v;
    });
    res.writeHead(r.status, out);
    if (r.body) for await (const c of r.body) res.write(c);
    res.end();
  })
  .listen(port, "127.0.0.1", () => console.log("ready"));
