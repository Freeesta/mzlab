// @ts-strict
// Transport of the assistant (the document inside the iframe `llm/trasporto.html`, which has a CSP of its own): the only place that holds the API key and the only code that
// fetches a model. No logic: the worker (`llm-worker.js`) builds each request and decodes the bytes; here the key goes into the header named by `auth`, the bytes come back unread.
// Messages: the page sends `init` (labels, provider, port to the worker) with window.postMessage; the worker talks over that port (`send`, `abort` / `head`, `chunk`, `end`, `fail`).
(() => {
  "use strict";
  const DB = "mzlab_llm", STORE = "llm_chiavi";
  const $ = (/** @type {string} */ id) => /** @type {any} */ (document.getElementById(id));
  /** @type {{provider: string, local: boolean, labels: Record<string, string>, port: MessagePort | null}} */
  const st = { provider: "", local: false, labels: {}, port: null };
  /** the key lives here and nowhere else; it is never posted out of this document @type {string} */
  let key = "";
  /** @type {Map<string, AbortController>} */
  const live = new Map();

  const say = (/** @type {string} */ k) => { $("state").textContent = st.labels[k] || ""; };
  const idb = () => new Promise((/** @type {(db: IDBDatabase) => void} */ ok, no) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: "id" });
    r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error);
  });
  /** @param {string} id @param {any} [value] a missing value deletes the entry @returns {Promise<any>} */
  const store = async (id, value) => {
    const db = await idb();
    return new Promise((ok, no) => {
      const tx = db.transaction(STORE, "readwrite"), os = tx.objectStore(STORE);
      const r = value === undefined ? os.get(id) : value === null ? os.delete(id) : os.put({ id, ...value });
      tx.oncomplete = () => { db.close(); ok(r.result); }; tx.onerror = tx.onabort = () => { db.close(); no(tx.error); };
    });
  };
  // the stored key is encrypted with a CryptoKey that cannot be exported: a copy of the disk does not give the key (a script of this origin could still ask this document to decrypt)
  const wrapKey = async () => {
    const old = await store("wrap");
    if (old && old.key) return /** @type {CryptoKey} */ (old.key);
    const k = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
    await store("wrap", { key: k });
    return k;
  };
  const remember = async (/** @type {string} */ v) => {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const data = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, await wrapKey(), new TextEncoder().encode(v));
    await store("k:" + st.provider, { iv, data });
  };
  const recall = async () => {
    try {
      const e = await store("k:" + st.provider);
      if (!e) return "";
      return new TextDecoder().decode(await crypto.subtle.decrypt({ name: "AES-GCM", iv: e.iv }, await wrapKey(), e.data));
    } catch (_) { return ""; }
  };
  const paint = () => {
    $("f").hidden = false;
    $("key").disabled = $("b-use").disabled = $("remember").disabled = st.local;
    $("b-forget").disabled = st.local && !key;
    $("hint").textContent = st.labels[st.local ? "local" : "hint"] || "";
    say(st.local ? "local" : key ? "set" : "none");
  };
  const post = (/** @type {any} */ m) => { if (st.port) st.port.postMessage(m); };

  /** @param {{id: string, request: {url: string, headers: string[][], body: string, auth: {header: string, prefix: string}}}} m */
  const send = async m => {
    const { id, request: r } = m, ctl = new AbortController();
    live.set(id, ctl);
    try {
      if (!/^https?:\/\//.test(r.url)) throw { key: "err.llm.network", params: { reason: "url" } };
      const headers = new Headers(/** @type {[string, string][]} */ (r.headers));
      if (r.auth.header) {
        if (!key) throw { key: "err.llm.auth", params: { reason: "nokey" } };
        headers.set(r.auth.header, r.auth.prefix + key);
      }
      const res = await fetch(r.url, { method: "POST", headers, body: r.body, signal: ctl.signal, credentials: "omit", referrerPolicy: "no-referrer" });
      post({ t: "head", id, status: res.status });
      const rd = res.body && res.body.getReader();
      if (rd) for (;;) { const { done, value } = await rd.read(); if (done) break; const b = value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength); st.port && st.port.postMessage({ t: "chunk", id, bytes: b }, [b]); }
      post({ t: "end", id });
    } catch (e) {
      const x = /** @type {any} */ (e);
      if (ctl.signal.aborted) post({ t: "fail", id, key: "err.llm.aborted", params: {} });
      else post({ t: "fail", id, key: x && x.key ? x.key : "err.llm.network", params: x && x.params ? x.params : { reason: "fetch" } });
    } finally { live.delete(id); }
  };

  /** @param {MessageEvent} ev */
  const onPort = ev => {
    const m = ev.data || {};
    if (m.t === "send") send(m);
    else if (m.t === "abort") { const c = live.get(m.id); if (c) c.abort(); }
  };

  window.addEventListener("message", async ev => {
    if (ev.origin !== location.origin || ev.source !== window.parent) return;
    const m = ev.data || {};
    if (m.op === "init") {
      st.provider = String(m.provider || ""); st.local = !!m.local; st.labels = m.labels || {};
      $("t-key").textContent = st.labels.key || ""; $("b-use").textContent = st.labels.use || ""; $("b-forget").textContent = st.labels.forget || ""; $("t-remember").textContent = st.labels.remember || "";
      $("key").setAttribute("aria-label", st.labels.key || "");
      if (ev.ports[0]) { st.port = ev.ports[0]; st.port.onmessage = onPort; }
      key = st.local ? "" : await recall();
      $("remember").checked = !!key;
      paint();
      window.parent.postMessage({ op: "ready", hasKey: !!key }, location.origin);
    }
  });
  $("b-use").addEventListener("click", async () => {
    key = $("key").value.trim(); $("key").value = "";
    try { if ($("remember").checked && key) await remember(key); else await store("k:" + st.provider, null); } catch (_) { /* storage blocked: the key stays for this session */ }
    paint(); window.parent.postMessage({ op: "key", hasKey: !!key }, location.origin);
  });
  $("b-forget").addEventListener("click", async () => {
    key = ""; $("key").value = ""; $("remember").checked = false;
    try { await store("k:" + st.provider, null); } catch (_) { /* nothing stored */ }
    paint(); window.parent.postMessage({ op: "key", hasKey: false }, location.origin);
  });
  $("f").hidden = true;
})();
