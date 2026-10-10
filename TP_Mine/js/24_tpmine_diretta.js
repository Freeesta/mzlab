// TPMINE-PRIVATE  mzFinder: the «Dig» live. While the engine works, the right-hand side shows the phases with their state, the candidates that pass the criteria
// (compact rows, best provisional score first) and, in high resolution, the MSn tree growing node by node. 20_tpmine.js feeds it with the events of the worker
// (phase, payload); the final results replace this panel. No animation; the phase list is announced (aria-live) only when the phase changes.
(() => {
  "use strict";
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
.tp .live .sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0)}`;
  const fmt = s => s < 10 ? s.toFixed(1) + " s" : Math.round(s) + " s";
  let cur = null;

  function start(R, hr, h) {
    stop();
    if (!document.getElementById("tp-live-css")) { const st = document.createElement("style"); st.id = "tp-live-css"; st.textContent = CSS; document.head.appendChild(st); }
    const esc = h.esc, list = PH[hr ? "hr" : "lr"];
    R.innerHTML = `<div class="card live" id="tp-live"><h3>Digging...</h3><div class="sr" id="tp-live-sr" role="status" aria-live="polite"></div>
      <ol id="tp-live-ph" aria-label="Phases">${list.map(([k, n]) => `<li class="ph" data-ph="${k}" data-s="wait"><span class="pi" aria-hidden="true">○</span><span>${esc(n)}</span><span class="mut pt">waiting</span></li>`).join("")}</ol>
      <div id="tp-live-f" class="mut"></div>
      <div id="tp-live-c" hidden><div class="mut">Candidates that pass the criteria (provisional score)</div><div class="cands" id="tp-live-cl"></div></div>
      ${hr ? `<div id="tp-live-t" hidden></div>` : ""}</div>`;
    const st = { R, hr, h, list, t0: performance.now(), ph: null, tph: 0, cands: new Map(), nodes: [], files: [], alb: null, timer: null, last: new Map() };
    st.timer = setInterval(() => tick(st), 500);
    cur = st;
  }

  function setPhase(st, k) {
    const now = performance.now();
    if (st.ph) finish(st, st.ph, now);
    st.ph = k; st.tph = now;
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
    if (!st.ph || !st.R.isConnected) return;
    const li = st.R.querySelector(`li[data-ph="${st.ph}"]`);
    if (li && li.dataset.s === "run") li.children[2].textContent = fmt((performance.now() - st.tph) / 1000);
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
    if (Array.isArray(m.payload)) m.payload.forEach(p => addPayload(st, p));
  }
  function stop(ok) {
    const st = cur; if (!st) return;
    clearInterval(st.timer); cur = null;
    if (!st.R.isConnected) return;
    if (st.ph) finish(st, st.ph, performance.now());
    if (!ok) { const t = st.R.querySelector("#tp-live h3"); if (t) t.textContent = "Stopped"; }
    st.R.querySelectorAll('li[data-s="wait"]').forEach(l => mark(l, "skip", "–", "not needed"));
  }

  window.TPLIVE = { start, event, stop, phases: () => cur && cur.ph };
})();
