// @ts-strict
// TPMINE-PRIVATE  mzFinder: the «Dig» live. While the engine works, the right-hand side shows the phases with their state, the candidates that pass the criteria
// (compact rows, best provisional score first) and, in high resolution, the MSn tree growing node by node. 20_tpmine.js feeds it with the events of the worker
// (phase, payload); the final results replace this panel. No animation; the phase list is announced (aria-live) only when the phase changes.
// The head (elapsed time, time left, «Still working...», Cancel, the summary of a failed Dig with «Copy diagnosis») is the common progress panel of the page (avanzamento.js),
// with the English texts below: Tabui is in English whatever the language of the page.
(() => {
  "use strict";
  /** @type {Promise<any>} */
  const AVP = import(new URL("../avanzamento.js", /** @type {HTMLScriptElement} */ (document.currentScript).src).href);      // static/avanzamento.js, next to static/mzfinder/
  AVP.catch(() => {});
  const EN = {
    "av.cancel": "Cancel", "av.cancel.title": "Cancel the Dig (Escape)", "av.item": "{k} of {n}: {name}", "av.elapsed": "{t} elapsed", "av.left": "about {t} left", "av.beat": "Still working...",
    "av.stall": "No news for over 30 seconds: keep this tab open; if it does not restart you can cancel and try again.", "av.sum": "{done} of {n} done", "conv.cancelling": "Cancelling...",
    "conv.sum.fail": "{n} failed", "conv.sum.cancel": "{n} cancelled", "common.close": "Close", "diag.copy": "Copy diagnosis", "diag.copied": "Diagnosis copied", "diag.copyFail": "Copy failed: select the text by hand",
  };
  const T = (k, p) => String(EN[k] || k).replace(/\{(\w+)\}/g, (_, n) => p && p[n] != null ? String(p[n]) : "{" + n + "}");
  const PH = {
    lr: [["calibration", "Calibration"], ["candidates", "Candidates"], ["xic", "XICs and peaks"], ["unexpected", "Unexpected ions"], ["ms2", "MS2"], ["tables", "Tables"]],
    hr: [["tree", "MSn tree"], ["features", "Features (per file)"], ["align", "Alignment and filters"], ["candidates", "Candidates"], ["families", "Families"], ["network", "Network"]],
  };
  const MAX_ROWS = 60;
  const CSS = `
.tp .live ol{list-style:none;margin:0 0 8px;padding:0}.tp .live li.ph{display:grid;grid-template-columns:18px 1fr auto;gap:6px;padding:2px 0;font-size:13px}
.tp .live li.ph[data-s=run]{font-weight:600}.tp .live li.ph[data-s=wait],.tp .live li.ph[data-s=skip]{color:var(--muted)}.tp .live li.ph[data-s=done] .pi{color:var(--ok)}
.tp .live .cand{display:grid;grid-template-columns:34px 1fr auto auto;gap:8px;align-items:baseline;font-size:12px;padding:1px 4px;border-bottom:1px solid var(--line)}
.tp .live .cand b{text-align:right}.tp .live .cands{max-height:30vh;overflow:auto;border:1px solid var(--line);border-radius:6px;margin:6px 0}
.tp .live progress{display:block;width:100%;height:6px;margin-top:2px;accent-color:var(--acc,#2b5c8a)}
.tp .live button.lcancel{min-height:32px;min-width:80px}
.tp .live .sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0)}`;
  const fmt = s => s < 10 ? s.toFixed(1) + " s" : Math.round(s) + " s";
  let cur = null;

  function start(R, hr, h) {
    stop();
    if (!document.getElementById("tp-live-css")) { const st = document.createElement("style"); st.id = "tp-live-css"; st.textContent = CSS; document.head.appendChild(st); }
    const esc = h.esc, list = PH[hr ? "hr" : "lr"];
    R.innerHTML = `<div class="card live" id="tp-live" tabindex="-1"><h3>Digging...</h3>
      <div id="tp-live-av"></div><div class="sr" id="tp-live-sr" role="status" aria-live="polite"></div>
      <ol id="tp-live-ph" aria-label="Phases">${list.map(([k, n]) => `<li class="ph" data-ph="${k}" data-s="wait"><span class="pi" aria-hidden="true">○</span><span>${esc(n)}</span><span class="mut pt">waiting</span></li>`).join("")}</ol>
      <div id="tp-live-f" class="mut"></div>
      <div id="tp-live-c" hidden><div class="mut">Candidates that pass the criteria (provisional score)</div><div class="cands" id="tp-live-cl"></div></div>
      ${hr ? `<div id="tp-live-t" hidden></div>` : ""}</div>`;
    const st = { R, hr, h, list, t0: performance.now(), ph: null, tph: 0, cands: new Map(), nodes: [], files: [], alb: null, timer: /** @type {any} */ (null), last: new Map(), A: null, text: "" };
    st.timer = setInterval(() => tick(st), 500);
    cur = st; st.frac = 0;
    const live = R.querySelector("#tp-live"), btn = () => /** @type {HTMLButtonElement | null} */ (R.querySelector("#tp-live-cancel"));
    if (live) live.addEventListener("keydown", e => { const cb = btn(); if (e.key === "Escape" && cb && !cb.disabled) { e.preventDefault(); cb.click(); } });
    AVP.then(m => {
      const box = R.querySelector("#tp-live-av"); if (cur !== st || !box) return;
      const ctl = new AbortController();
      if (h.cancel) ctl.signal.addEventListener("abort", () => h.cancel());
      // the weights of the phases are a rough model: the time left only from a fifth of the work
      const A = st.A = m.avanza(/** @type {HTMLElement} */ (box), { title: "Dig", n: 1, cancel: !!h.cancel, signal: ctl, t: T, eta: 0.2 });
      const cb = /** @type {HTMLButtonElement | null} */ (box.querySelector("[data-r=cancel]"));
      if (cb && h.cancel) { cb.id = "tp-live-cancel"; cb.classList.add("lcancel"); cb.focus({ preventScroll: true }); }
      A.set(st.frac, st.text);
    });
  }

  function setPhase(st, k) {
    const now = performance.now();
    if (st.ph) finish(st, st.ph, now);
    st.ph = k; st.tph = now;
    if (typeof DIAG !== "undefined") DIAG.LONG.marca(k);      // the long tasks of the page are told apart by phase (diagnosi.js)
    const li = st.R.querySelector(`li[data-ph="${k}"]`);
    if (!li) return;
    // phases skipped before this one (not announced by the engine) are shown as such
    for (const [p] of st.list) { if (p === k) break; const l = st.R.querySelector(`li[data-ph="${p}"]`); if (l && l.dataset.s === "wait") mark(l, "skip", "–", "not needed"); }
    mark(li, "run", "●", "0.0 s");
    const sr = st.R.querySelector("#tp-live-sr"); if (sr) sr.textContent = `${li.children[1].textContent}`;
  }
  function mark(li, s, icon, text) { li.dataset.s = s; li.children[0].textContent = icon; li.children[2].textContent = text; }
  function finish(st, k, now) {
    const li = st.R.querySelector(`li[data-ph="${k}"]`);
    if (li && li.dataset.s === "run") { const d = (now - st.tph) / 1000; st.last.set(k, d); mark(li, "done", "✓", fmt(d)); }
  }
  function tick(st) {
    if (!st.R.isConnected || !st.ph) return;
    const li = st.R.querySelector(`li[data-ph="${st.ph}"]`);
    if (li && li.dataset.s === "run") li.children[2].textContent = fmt((performance.now() - st.tph) / 1000);
  }

  /** «k of N» and a bar in the row of a phase that goes file by file (the engine says how many are done) */
  function count(st, k, done, total) {
    const li = st.R.querySelector(`li[data-ph="${k}"]`); if (!li) return;
    let pr = /** @type {HTMLProgressElement | null} */ (li.querySelector("progress"));
    if (!pr) { pr = document.createElement("progress"); pr.setAttribute("aria-label", `${li.children[1].textContent}: files done`); li.children[1].appendChild(pr); }
    pr.max = total; pr.value = done;
    const c = li.querySelector(".pc") || (() => { const x = document.createElement("span"); x.className = "pc mut"; x.style.marginLeft = "6px"; li.children[1].insertBefore(x, pr); return x; })();
    c.textContent = ` ${done} of ${total}`;
  }
  function addPayload(st, p) {
    if (p.kind === "candidate") {
      st.cands.set(p.id, p); drawCands(st);
    } else if (p.kind === "features") {
      st.files.push(p);
      st.R.querySelector("#tp-live-f").textContent = st.files.map(f => `${f.file}: ${f.n} features`).join(" · ");
    } else if (p.kind === "node") {
      st.nodes.push({ id: p.id, parent: p.parent, level: p.level, kind: "ion", mz: p.mz, formula: p.formula, rel: null, ghost: p.ghost, smiles: [], alternatives: [] });
      drawTree(st);
    }
  }
  function drawCands(st) {
    const box = st.R.querySelector("#tp-live-c"); if (!box) return;
    box.hidden = false;
    const rows = [...st.cands.values()].sort((a, b) => b.score - a.score).slice(0, MAX_ROWS), esc = st.h.esc;
    st.R.querySelector("#tp-live-cl").innerHTML = rows.map(c => `<div class="cand"><b>${Math.round(c.score)}</b><span>${esc(c.name)}</span><span class="mut"><i>m/z</i> ${c.mz}</span><span class="mut">${c.rt != null ? "RT " + c.rt : ""}</span></div>`).join("");
  }
  function drawTree(st) {
    const box = st.R.querySelector("#tp-live-t"); if (!box || !window.TPALBERO) return;
    box.hidden = false;
    if (!st.alb) st.alb = window.TPALBERO.mount(box, { esc: st.h.esc, call: st.h.call }, { live: true });
    st.alb.live({ nodes: st.nodes.slice(), supposed: false });
  }

  // one event of the worker: {phase, payload (list of dicts or null)}
  function event(m) {
    const st = cur; if (!st || !st.R.isConnected) return;
    if (m.phase && m.phase !== st.ph) setPhase(st, m.phase);
    if (typeof m.frac === "number" && m.frac >= st.frac) st.frac = m.frac;      // never goes back
    if (m.text) st.text = String(m.text);
    if (st.A && (typeof m.frac === "number" || m.text)) st.A.set(st.frac, st.text);
    if (m.total > 1 && m.phase) count(st, m.phase, m.done, m.total);
    if (Array.isArray(m.payload)) m.payload.forEach(p => addPayload(st, p));
  }
  /** the end of the Dig: `ok` = results (they replace the panel); otherwise cancelled (no `err`) or failed: the panel stays with the error, the phase it stopped in and «Copy diagnosis»
   * @param {boolean} [ok] @param {string | null} [err] */
  function stop(ok, err) {
    const st = cur; if (!st) return;
    clearInterval(st.timer); cur = null;
    if (!st.R.isConnected) { if (st.A) st.A.remove(); return; }
    const ph = st.ph, name = ph ? (st.list.find(x => x[0] === ph) || [ph, ph])[1] : "";
    if (ph) finish(st, ph, performance.now());
    if (!ok) { const t = st.R.querySelector("#tp-live h3"); if (t) t.textContent = "Stopped"; }
    st.R.querySelectorAll('li[data-s="wait"]').forEach(l => mark(l, "skip", "–", "not needed"));
    if (!st.A) return;
    if (ok || !err) { st.A.remove(); return; }
    st.A.fail(name, err, diagText(st, ph, err));
    st.A.end({ head: name ? `stopped during ${name}` : "stopped" });
  }
  // what «Copy diagnosis» copies for a failed Dig: version, cores and long tasks of the page (DIAG.testoLento), mode, phase and its time, the error without paths
  function diagText(st, ph, err) {
    const D = typeof DIAG !== "undefined" ? DIAG : null, ver = window.MZLAB_VERSION || "?";
    const head = D ? D.testoLento(ver, st.t0) : `mzLab ${ver}`, times = [...st.last].map(([k, d]) => `${k} ${fmt(d)}`).join(", ");
    return [head, `dig: ${st.hr ? "HR" : "LR"}`, `phase: ${ph || "?"}`, `phases done: ${times || "none"}`, `error: ${D ? D.pulisci(err) : err}`].join("\n");
  }

  window.TPLIVE = { start, event, stop, phases: () => cur && cur.ph };
})();
