// Runs mzlab/web/llm/trasporto.js against stub DOM, registry and fetch: prints the fetches the transport makes for each send.
// usage: node trasporto_node.js <web/llm dir> <provider> <proxy host or ""> <url>...
const fs = require("fs"), vm = require("vm"), path = require("path");
const [dir, provider, proxy, ...urls] = process.argv.slice(2);
let hang = false; // RACE: the IndexedDB of a second init never answers
let failRegistry = Number(process.env.FAIL_REGISTRY || 0);
const els = {}, clicks = {}, handlers = {}, fetched = [];
const el = id => els[id] || (els[id] = { id, value: "", hidden: false, disabled: false, checked: false, textContent: "", setAttribute() {}, addEventListener(t, f) { clicks[id + ":" + t] = f; } });
const files = { "fornitori-1.json": fs.readFileSync(path.join(dir, "../../../crates/mzlab-llm/data/fornitori-1.json"), "utf8"), "proxy-host.js": `export const PROXY_HOST = "${proxy}";\n` };
class H { constructor(a = []) { this.m = new Map(a); } set(k, v) { this.m.set(k.toLowerCase(), v); } }
const parent = { postMessage() {} }, port = { postMessage(m) { port.out.push(m); }, out: [] };
const ctx = { document: { getElementById: el }, window: { addEventListener(t, f) { handlers[t] = f; }, parent, postMessage() {} }, location: { origin: "http://x" }, Headers: H, AbortController,
  indexedDB: { open() { const r = {}; setTimeout(() => { if (!hang && r.onerror) r.onerror(); }, 0); return r; } },
  TextEncoder, TextDecoder, URL, Promise, JSON, console, crypto: {},
  fetch: async (u, o = {}) => {
    const name = String(u);
    if (name === "fornitori-1.json" && failRegistry-- > 0) throw new Error("offline");
    if (files[name] !== undefined) return { json: async () => JSON.parse(files[name]), text: async () => files[name] };
    fetched.push({ url: name, redirect: o.redirect, headers: [...o.headers.m.entries()] });
    return { status: 200, headers: { get: () => null }, body: null };
  } };
ctx.window.parent = parent;
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(path.join(dir, "trasporto.js"), "utf8"), ctx);
(async () => {
  await handlers.message({ origin: "http://x", source: parent, data: { op: "init", provider, local: false, owner: "", labels: {} }, ports: [port] });
  els.key.value = "sk-SECRET"; await clicks["b-use:click"]();
  if (process.env.RACE) { hang = true; handlers.message({ origin: "http://x", source: parent, data: { op: "init", provider: process.env.RACE, local: false, owner: "", labels: {} }, ports: [port] }); }
  const out = [];
  for (const [i, url] of urls.entries()) {
    fetched.length = 0; port.out.length = 0;
    port.onmessage({ data: { t: "send", id: "s" + i, request: { url, headers: [], body: "{}", auth: { header: "x-api-key", prefix: "" } } } });
    for (let k = 0; k < 20; k++) await new Promise(r => setTimeout(r, 5));
    out.push({ url, fetched: fetched.map(f => ({ ...f })), msgs: port.out.map(m => m.t + (m.key ? ":" + m.key + ":" + JSON.stringify(m.params) : "")) });
  }
  console.log(JSON.stringify(out));
})();
