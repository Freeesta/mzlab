"use strict";
// Performance meter. Off by default and then it costs nothing (the script returns at once).
// On with ?perf in the address, or Ctrl+Alt+P (the page asks to reload with ?perf). A small box in the bottom left shows:
//   - start-up: first paint, engine steps (Pyodide), time to "ready";
//   - every call to api/... (ms; in the browser version also the time spent in Python, header X-Py-Ms: the rest is waiting and copying);
//   - every panel draw (ms, with the panel type) and the commands that move around (tab change, arrow step);
//   - the loading of each file as measured in Python (read of the index, peak table, reading mode) from api/perf;
//   - long tasks (> 50 ms) where the browser reports them (not Safari).
// «Copia» puts everything on the clipboard as text. ES module (runs after every classic script); the hooks wrap existing functions, no change to them.
(() => {
  const url = new URL(location.href);
  const ON = url.searchParams.has("perf");
  document.addEventListener("keydown", e => {
    if (e.ctrlKey && e.altKey && !e.metaKey && (e.key === "p" || e.key === "P" || e.code === "KeyP")) {
      e.preventDefault();
      if (ON) { const b = document.getElementById("perfbox"); if (b) b.hidden = !b.hidden; }
      else { url.searchParams.set("perf", "1"); location.href = url.href; }
    }
  });
  if (!ON) return;

  const rows = [];                       // {t, kind, what, ms, extra}
  const t0 = 0;                          // times are seconds since the page was opened (performance.now())
  const note = (kind, what, ms, extra) => { rows.push({ t: performance.now() - t0, kind, what, ms, extra: extra || "" }); schedule(); };
  const stat = {};                       // kind + what -> {n, sum, max}
  const add = (key, ms) => { const s = stat[key] || (stat[key] = { n: 0, sum: 0, max: 0 }); s.n++; s.sum += ms; s.max = Math.max(s.max, ms); };
  const f1 = x => (x == null || isNaN(x) ? "-" : x >= 100 ? x.toFixed(0) : x.toFixed(1));

  window.PERF = {
    rows,
    // browser.js reports the engine steps here
    mark: (text, at) => { rows.push({ t: at == null ? performance.now() : at, kind: "start", what: String(text), ms: at == null ? performance.now() : at, extra: "" }); schedule(); },
    t: (what, ms, extra) => { note("misura", what, ms, extra); add("misura " + what, ms); },
  };

  // ---- fetch: outermost wrapper, whatever order the scripts are loaded in (browser.js replaces window.fetch too)
  let inner = window.fetch.bind(window);
  const kb = n => (n / 1024 >= 100 ? (n / 1048576).toFixed(2) + " MB" : (n / 1024).toFixed(0) + " KB");
  const wrapped = async (input, init) => {
    const u = typeof input === "string" ? input : (input && input.url) || "";
    const m = /(?:^|\/)api\/([a-z]+)/.exec(u);
    if (!m || m[1] === "perf" || m[1] === "ping" || m[1] === "bye") return inner(input, init);
    const t = performance.now();
    const r = await inner(input, init);
    const ms = performance.now() - t;
    const py = r.headers && r.headers.get && r.headers.get("X-Py-Ms");
    const q = u.split("?")[1] || "";
    note("api", m[1] + (q ? "?" + (q.length > 60 ? q.slice(0, 57) + "..." : q) : ""), ms, py != null ? "python " + f1(+py) + " ms" : "");
    add("api " + m[1], ms);
    pyRows();
    return r;
  };
  Object.defineProperty(window, "fetch", { configurable: true, get: () => wrapped, set: f => { inner = f.bind ? f.bind(window) : f; } });

  // ---- the file loading as seen by Python (index, peak table): asked after the calls, only the new numbers are written
  const seen = new Set();
  let pyT = 0;
  function pyRows() {
    clearTimeout(pyT);
    pyT = setTimeout(async () => {
      try {
        const j = await (await inner("api/perf")).json();
        for (const f of j.files || []) for (const [k, v] of Object.entries(f.timing || {})) {
          const key = f.file + k + v;
          if (seen.has(key)) continue;
          seen.add(key);
          note("python", `${f.file}: ${k}`, v * 1000, `${f.mode}, ${(f.size / 1048576).toFixed(0)} MB`);
          add("python " + k, v * 1000);
        }
      } catch (_) { /* old server without api/perf: nothing to add */ }
    }, 300);
  }

  // ---- panel draws and the commands that move around: wrap the global functions (they are plain function declarations)
  function wrapFn(name, label) {
    const f = window[name];
    if (typeof f !== "function" || f.__perf) return;
    const w = function (...a) {
      const t = performance.now(), p = a[0];
      const done = () => { const ms = performance.now() - t, what = label + (p && p.type ? " " + p.type : ""); note("disegno", what, ms); add("disegno " + what, ms); };
      let r;
      try { r = f.apply(this, a); } catch (e) { done(); throw e; }
      if (r && typeof r.then === "function") return r.then(v => { done(); return v; }, e => { done(); throw e; });
      done();
      return r;
    };
    w.__perf = true;
    window[name] = w;
  }
  function hook() { wrapFn("draw", "draw"); wrapFn("setTab", I18N.t("perf.hook.tab")); wrapFn("stepScan", I18N.t("perf.hook.step")); wrapFn("openXic", I18N.t("perf.hook.xic")); }
  hook();
  window.addEventListener("load", hook);

  const kinds = (window.PerformanceObserver && PerformanceObserver.supportedEntryTypes) || [];      // Firefox and Safari warn in the console if asked for an unknown type
  if (kinds.includes("longtask")) try {
    new PerformanceObserver(l => { for (const e of l.getEntries()) { note("lento", "attività lunga", e.duration, "main thread"); add("lento", e.duration); } }).observe({ entryTypes: ["longtask"] });
  } catch (_) { /* Safari: no long tasks */ }
  if (kinds.includes("paint")) try {
    new PerformanceObserver(l => { for (const e of l.getEntries()) note("start", e.name, e.startTime); }).observe({ type: "paint", buffered: true });
  } catch (_) { /* optional */ }

  // ---- the box
  const css = document.createElement("style");
  css.textContent = `#perfbox{position:fixed;left:8px;bottom:8px;z-index:20000;width:420px;max-width:calc(100vw - 16px);max-height:55vh;display:flex;flex-direction:column;
    background:var(--panel,#fff);color:var(--fg,#222);border:1px solid var(--border,#999);border-radius:8px;box-shadow:0 4px 18px rgba(0,0,0,.25);font:12px/1.35 ui-monospace,Menlo,Consolas,monospace}
    #perfbox header{display:flex;gap:6px;align-items:center;padding:4px 6px;border-bottom:1px solid var(--border,#999)}#perfbox header b{flex:1}
    #perfbox button{font:inherit;padding:1px 7px;cursor:pointer}#perfbox pre{margin:0;padding:6px;overflow:auto;white-space:pre;flex:1}`;
  document.head.appendChild(css);
  const box = document.createElement("div");
  box.id = "perfbox";
  box.innerHTML = `<header><b>${I18N.t("perf.title")}</b><button id="perf-copy" title="${I18N.t("perf.copy.title")}">${I18N.t("perf.copy")}</button><button id="perf-clear" title="${I18N.t("perf.clear.title")}">${I18N.t("perf.clear")}</button><button id="perf-min" title="${I18N.t("perf.min.title")}">–</button></header><pre id="perf-out"></pre>`;
  document.body.appendChild(box);
  const out = box.querySelector("#perf-out");
  let min = false, pending = false;
  function schedule() { if (!pending) { pending = true; setTimeout(() => { pending = false; out.textContent = text(true); out.scrollTop = out.scrollHeight; }, 250); } }

  function env() {
    const n = navigator, c = n.connection || {};
    return [`${n.userAgent}`, I18N.t("perf.env", { sw: screen.width, sh: screen.height, iw: innerWidth, ih: innerHeight, dpr: devicePixelRatio, cores: n.hardwareConcurrency || "?", mem: n.deviceMemory || "?", net: c.effectiveType || "?" }),
      I18N.t("perf.version", { app: window.APP_NAME || "?", title: document.title }), I18N.t("perf.engine", { engine: I18N.t(window.QQQ_BROWSER ? "perf.engine.browser" : "perf.engine.local") })];
  }
  function startLines() {
    const nav = performance.getEntriesByType("navigation")[0], o = [];
    if (nav) o.push(`pagina: DOM pronto ${f1(nav.domContentLoadedEventEnd)} ms, caricata ${f1(nav.loadEventEnd)} ms`);
    return o;
  }
  function text(short) {
    const lines = [];
    if (!short) lines.push(...env(), ...startLines(), "");
    else lines.push(...startLines());
    const list = short ? rows.slice(-18) : rows;
    for (const r of list) lines.push((`${(r.t / 1000).toFixed(2).padStart(7)} s  ${r.kind.padEnd(7)} ${r.what}` + (r.kind === "start" ? "  " + I18N.t("perf.sinceOpen", { ms: f1(r.ms) }) : `  ${f1(r.ms)} ms`) + (r.extra ? `  (${r.extra})` : "")).replace(/\s+$/, ""));
    const keys = Object.keys(stat).sort();
    if (keys.length) {
      lines.push("", I18N.t("perf.summary"));
      for (const k of keys) { const s = stat[k]; lines.push(`${k.padEnd(28)} ${String(s.n).padStart(4)} · ${f1(s.sum / s.n).padStart(7)} · ${f1(s.max).padStart(7)}`); }
    }
    return lines.join("\n");
  }
  box.querySelector("#perf-copy").onclick = async () => {
    const t = text(false);
    try { await navigator.clipboard.writeText(t); }
    catch (_) { const a = document.createElement("textarea"); a.value = t; document.body.appendChild(a); a.select(); try { document.execCommand("copy"); } catch (e) { /* the text stays in the box */ } a.remove(); }
    const b = box.querySelector("#perf-copy"); b.textContent = I18N.t("perf.copied"); setTimeout(() => { b.textContent = I18N.t("perf.copy"); }, 1500);
  };
  box.querySelector("#perf-clear").onclick = () => { rows.length = 0; for (const k of Object.keys(stat)) delete stat[k]; schedule(); };
  box.querySelector("#perf-min").onclick = () => { min = !min; out.hidden = min; };
  for (const [at, text] of window.PERF_EARLY || []) window.PERF.mark(text, at);     // engine steps seen by browser.js before this script loaded
  note("start", "misure attive", performance.now());
})();
