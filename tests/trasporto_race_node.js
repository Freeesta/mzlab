// Races of mzlab/web/llm/trasporto.js against a stub DOM, a controllable IndexedDB (an operation can be held and released) and fetch.
// usage: node trasporto_race_node.js <web/llm dir> <scenario>; prints {fetched, msgs, stored}: what left the document, what the worker heard, what was written to the store.
const fs = require("fs"), vm = require("vm"), path = require("path");
const [dir, scenario] = process.argv.slice(2);
const PROXY = "https://assist.example.org";
const els = {}, clicks = {}, handlers = {}, fetched = [], data = new Map(), hold = new Set(), waiters = {}; let keys = 0;
const el = id => els[id] || (els[id] = { id, value: "", hidden: false, disabled: false, checked: false, textContent: "", setAttribute() {}, addEventListener(t, f) { clicks[id + ":" + t] = f; } });
const tick = (n = 10) => new Promise(r => setTimeout(r, n));
const gate = name => hold.has(name) ? new Promise(r => { (waiters[name] = waiters[name] || []).push(r); }) : Promise.resolve();
const release = name => { hold.delete(name); (waiters[name] || []).forEach(r => r()); waiters[name] = []; };
const seed = (provider, text) => data.set("k:" + provider, { id: "k:" + provider, iv: new Uint8Array(12), data: new Uint8Array([...new TextEncoder().encode(text), 0]) });
const db = { createObjectStore() {}, close() {}, transaction() {
  const tx = {};
  const req = (kind, arg) => {
    const r = {}, id = kind === "put" ? arg.id : arg;
    gate(kind + ":" + id).then(() => { if (kind === "get") r.result = data.get(id); else if (kind === "put") data.set(id, arg); else data.delete(id); setTimeout(() => tx.oncomplete && tx.oncomplete(), 0); });
    return r;
  };
  tx.objectStore = () => ({ get: id => req("get", id), put: v => req("put", v), delete: id => req("delete", id) });
  return tx;
} };
const files = { "fornitori-1.json": fs.readFileSync(path.join(dir, "../../../crates/mzlab-llm/data/fornitori-1.json"), "utf8"), "proxy-host.js": `export const PROXY_HOST = "${PROXY}";\n` };
class H { constructor(a = []) { this.m = new Map(a); } set(k, v) { this.m.set(k.toLowerCase(), v); } }
const parent = { postMessage() {} }, port = { postMessage(m) { port.out.push(m); }, out: [] };
const ctx = { document: { getElementById: el }, window: { addEventListener(t, f) { handlers[t] = f; }, parent, postMessage() {} }, location: { origin: "http://x" }, Headers: H, AbortController,
  indexedDB: { open() { const r = {}; setTimeout(() => { r.result = db; if (r.onupgradeneeded) r.onupgradeneeded(); r.onsuccess(); }, 0); return r; } },
  TextEncoder, TextDecoder, URL, Promise, JSON, console,
  // a ciphertext ends with the id of the wrap key that made it: another key fails to decrypt it, as the real one does
  crypto: { getRandomValues: a => a, subtle: { generateKey: async () => ({ id: ++keys }), encrypt: async (_a, k, d) => new Uint8Array([...d, k.id]),
    decrypt: async (_a, k, d) => { if (d[d.length - 1] !== k.id) throw new Error("bad key"); return d.slice(0, -1); } } },
  fetch: async (u, o = {}) => {
    const name = String(u);
    if (name === "proxy-host.js") await gate("fetch:proxy-host");
    if (files[name] !== undefined) return { json: async () => JSON.parse(files[name]), text: async () => files[name] };
    fetched.push({ url: name, key: o.headers.m.get("x-api-key") || "" });
    return { status: 200, headers: { get: () => null }, body: null };
  } };
ctx.window.parent = parent;
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(path.join(dir, "trasporto.js"), "utf8"), ctx);
const init = provider => handlers.message({ origin: "http://x", source: parent, data: { op: "init", provider, local: false, owner: "", labels: {} }, ports: [port] });
const send = (url, i = 0) => port.onmessage({ data: { t: "send", id: "s" + i, request: { url, headers: [], body: "{}", auth: { header: "x-api-key", prefix: "" } } } });
(async () => {
  data.set("wrap", { id: "wrap", key: { id: 0 } });
  seed("anthropic", "key-of-anthropic"); seed("openai", "key-of-openai"); seed("mzlab-demo", "key-of-demo");
  if (scenario === "init-order") { // the recall of init A answers after init B is done
    hold.add("get:k:anthropic");
    const a = init("anthropic"); await tick();
    await init("openai"); release("get:k:anthropic"); await a; await tick();
    send("https://api.openai.com/v1/chat/completions");
  } else if (scenario === "init-then-recall") { // the recall of init A answers while init B (already ready) has no key stored
    data.delete("k:openai"); hold.add("get:k:anthropic");
    const a = init("anthropic"); await tick();
    await init("openai"); release("get:k:anthropic"); await a; await tick();
    send("https://api.openai.com/v1/chat/completions");
  } else if (scenario === "send-vs-init") { // init B lands while a send of provider A waits for the proxy host
    await init("mzlab-demo"); hold.add("fetch:proxy-host");
    send(PROXY + "/v1/messages"); await tick();
    await init("anthropic"); release("fetch:proxy-host");
  } else if (scenario === "remember-vs-init") { // init B lands while the key of A is being encrypted
    data.delete("wrap"); data.delete("k:anthropic"); data.delete("k:openai"); await init("anthropic");
    hold.add("get:wrap"); els.key.value = "typed-key"; els.remember.checked = true;
    const c = clicks["b-use:click"](); await tick();
    await init("openai"); release("get:wrap"); await c;
  } else if (scenario === "wrap-single-flight") { // two callers of wrapKey with no wrap key stored: the key encrypted by one must survive the other
    data.delete("wrap"); data.delete("k:anthropic"); await init("anthropic");
    hold.add("get:wrap"); els.key.value = "typed-key"; els.remember.checked = true;
    const c = clicks["b-use:click"](); await tick();
    const b = init("openai"); await tick();
    release("get:wrap"); await c; await b;
    await init("anthropic"); send("https://api.anthropic.com/v1/messages");
  }
  await tick(30);
  const stored = {};
  for (const [k, v] of data) if (k.startsWith("k:")) stored[k] = new TextDecoder().decode(v.data.slice(0, -1));
  console.log(JSON.stringify({ fetched, msgs: port.out.map(m => m.t + (m.key ? ":" + m.key + ":" + JSON.stringify(m.params) : "")), stored }));
})();
