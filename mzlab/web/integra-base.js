// @ts-nocheck
// Baseline of an integration other than the line between the edges: `bm` = "zero" (y = 0) or "line" (`bl` = [t0, y0, t1, y1], a level or two points that may
// cut through the peak). The numbers come from the Rust engine (QUALITA.peakArea, mzlab_core::baseline: signed area, the part under the line apart);
// this file only asks, draws (the part under the line hatched) and moves the two points. Classic script, loaded on demand (carica) by grafici.js.
const IB_MODE = { zero: 1, line: 2 }, IB_C = new WeakMap();           // the engine's answer per record (never saved: sessions and undo keep only bm/bl)
const ibKey = (s, it) => [s.key, s.x.length, s.y[0], s.y[s.y.length - 1], it.a, it.b, it.bm, (it.bl || []).join()].join("|");
// the measure of the record; until the engine answers: the previous one, or nothing (wait), then the panel is drawn again
function ibArea(p, s, it) {
  const k = ibKey(s, it), c = IB_C.get(it);
  if (c && c.k === k) return c.r;
  const r = c ? { ...c.r, wait: true } : { area: NaN, below: NaN, height: 0, rt: (it.a + it.b) / 2, wait: true };
  IB_C.set(it, { k, r });
  if (window.QUALITA) QUALITA.peakArea(s.x, s.y, it.a, it.b, IB_MODE[it.bm], it.bm === "line" ? it.bl : []).then(t => {
    const e = IB_C.get(it); if (!e || e.k !== k) return;
    const v = t ? JSON.parse(t) : null;
    e.r = v ? { area: v.area, below: v.below, height: v.height, rt: v.rt } : { area: NaN, below: NaN, height: 0, rt: r.rt };
    draw(p);
  });
  return r;
}
const ibLine = (it, t) => {
  if (it.bm !== "line") return 0;
  const [t0, y0, t1, y1] = it.bl; return t1 === t0 ? y0 : y0 + (y1 - y0) * (t - t0) / (t1 - t0);
};
// the part under the line: diagonal hatching in the colour of the trace (a pattern, not only a colour)
function ibHatch(g, col) {
  const c = document.createElement("canvas"); c.width = c.height = 8; const h = c.getContext("2d");
  h.strokeStyle = col; h.lineWidth = 1.5; h.beginPath(); h.moveTo(0, 8); h.lineTo(8, 0); h.moveTo(-2, 2); h.lineTo(2, -2); h.moveTo(6, 10); h.lineTo(10, 6); h.stroke();
  return g.createPattern(c, "repeat");
}
// the region between the trace and the line: filled above the line, hatched under it; the line dashed, the two points of a custom line as dots
function ibPaint(g, X, Y, U, s, it, chosen) {
  const i0 = nearIdx(s.x, Math.min(it.a, it.b)), i1 = nearIdx(s.x, Math.max(it.a, it.b)); if (i1 <= i0) return;
  const xa = s.x[i0], xb = s.x[i1], L = t => Y(U(s, ibLine(it, t))), big = 1e5;
  const region = () => {
    g.beginPath(); g.moveTo(X(xa), Y(U(s, s.ys[i0])));
    for (let j = i0 + 1; j <= i1; j++) g.lineTo(X(s.x[j]), Y(U(s, s.ys[j])));
    g.lineTo(X(xb), L(xb)); g.lineTo(X(xa), L(xa)); g.closePath();
  };
  const side = up => { g.beginPath(); g.moveTo(X(xa) - 1, L(xa)); g.lineTo(X(xb) + 1, L(xb)); g.lineTo(X(xb) + 1, up ? -big : big); g.lineTo(X(xa) - 1, up ? -big : big); g.closePath(); g.clip(); };
  g.save(); side(true); region(); g.globalAlpha = chosen ? 0.5 : 0.28; g.fillStyle = s.color; g.fill(); g.restore();
  g.save(); side(false); region(); g.globalAlpha = chosen ? 0.9 : 0.7; g.fillStyle = ibHatch(g, s.color); g.fill(); g.restore();
  if (chosen) { region(); g.strokeStyle = s.color; g.lineWidth = 2.5; g.stroke(); }
  const t0 = it.bm === "line" ? Math.min(xa, it.bl[0], it.bl[2]) : xa, t1 = it.bm === "line" ? Math.max(xb, it.bl[0], it.bl[2]) : xb;
  g.strokeStyle = s.color; g.lineWidth = 1.5; g.setLineDash([5, 3]); g.beginPath(); g.moveTo(X(t0), L(t0)); g.lineTo(X(t1), L(t1)); g.stroke(); g.setLineDash([]);
  if (it.bm === "line") for (const k of [0, 2]) {
    g.beginPath(); g.arc(X(it.bl[k]), L(it.bl[k]), 5, 0, 2 * Math.PI); g.fillStyle = ecss("--panel"); g.fill(); g.lineWidth = 2; g.strokeStyle = s.color; g.stroke();
  }
}
// the two points of a custom line as 24 px buttons over the chart: drag with mouse, finger or pen; ← → one sample (Shift five), ↑ ↓ 1 % of the axis (Shift 10 %)
function ibSync(p) {
  const a = p._a, old = p._bp || []; p._bp = [];
  const list = a && a.sr && !a.cas && a.X && a.yinv && !p._exp ? p.ints.filter(it => it.bm === "line" && it.bl && a.sr.some(q => q.key === it.key)) : [];
  for (const it of list) for (const k of [0, 2]) {
    const s = a.sr.find(q => q.key === it.key), b = old.shift() || ibButton(p);
    b._it = it; b._k = k; b._s = s; b.dataset.k = String(k / 2); p._bp.push(b);
    const px = a.X(it.bl[k]), py = a.Y(a.U(s, it.bl[k + 1])), out = px < M.l - 2 || px > a.W - M.r + 2 || py < M.t - 2 || py > a.H - M.b + 2;
    b.hidden = out; b.style.left = p.cv.offsetLeft + px + "px"; b.style.top = p.cv.offsetTop + py + "px";
    const v = I18N.t("int.base.pt", { n: k / 2 + 1, rt: it.bl[k].toFixed(3), y: I18N.sciE(it.bl[k + 1]) });
    b.setAttribute("aria-label", v); b.title = v; b.setAttribute("aria-valuetext", v); b.setAttribute("aria-valuenow", it.bl[k].toFixed(3));
  }
  old.forEach(n => n.remove());
}
function ibButton(p) {
  const n = document.createElement("button"); n.type = "button"; n.className = "ibp"; n.setAttribute("role", "slider"); p.el.appendChild(n);
  const raw = (s, py) => (p._a.yinv(py) - s.off) * s.sc;          // y on the canvas -> counts of the trace (U is linear: s.off + v / s.sc)
  const put = (t, y) => { const it = n._it, k = n._k; it.bl[k] = t; it.bl[k + 1] = y; if (it.lvl) it.bl[1] = it.bl[3] = y; draw(p); };
  const at = ev => { const r = p.cv.getBoundingClientRect(); return [xOf(p, ev.clientX - r.left), raw(n._s, ev.clientY - r.top)]; };
  n.addEventListener("pointerdown", ev => { ev.preventDefault(); ev.stopPropagation(); try { n.setPointerCapture(ev.pointerId); } catch (_) { /* events still arrive */ } setActive(p); pushIh(p); n.classList.add("on"); });
  n.addEventListener("pointermove", ev => { if (n.classList.contains("on")) put(...at(ev)); });
  const end = ev => { if (!n.classList.contains("on")) return; n.classList.remove("on"); if (ev.type === "pointerup") put(...at(ev)); uiSave(); };
  n.addEventListener("pointerup", end); n.addEventListener("pointercancel", end);
  n.addEventListener("keydown", ev => {
    const h = ev.key === "ArrowLeft" ? -1 : ev.key === "ArrowRight" ? 1 : 0, v = ev.key === "ArrowDown" ? -1 : ev.key === "ArrowUp" ? 1 : 0;
    if ((!h && !v) || ev.ctrlKey || ev.metaKey || ev.altKey) return;
    ev.preventDefault(); ev.stopPropagation(); setActive(p); pushIh(p);
    const it = n._it, k = n._k, sx = n._s.x, f = ev.shiftKey ? 5 : 1, span = (p._a.ymax - p._a.ymin) * n._s.sc;
    const t = h ? sx[Math.max(0, Math.min(sx.length - 1, nearIdx(sx, it.bl[k]) + h * f))] : it.bl[k];
    put(t, it.bl[k + 1] + v * f * (ev.shiftKey ? 0.02 : 0.01) * span); uiSave();
  });
  return n;
}
// the dialog of one integration (key B or «Baseline…» in the menu): the mode, the two points by number, a level
function ibOpen(p, it) {
  const s = p._a && p._a.sr && p._a.sr.find(q => q.key === it.key); if (!s) return;
  const bl = it.bl || edgeLine(s, it.a, it.b, it.base), m = it.bm || "edges";
  const opt = k => `<label class="ib-opt"><input type="radio" name="ib-m" value="${k}"${m === k ? " checked" : ""}> ${I18N.t("int.base." + k)}</label>`;
  const num = (k, i) => `<label class="pk-row"><span>${I18N.t("int.base." + k)}</span><input data-ib="${i}" value="${+bl[i].toPrecision(6)}" inputmode="decimal"></label>`;
  big(I18N.t("int.base.title"), `<div class="ib-dlg"><div class="muted sm pk-intro">${I18N.t("int.base.intro")}</div>
    <fieldset class="pk-sec"><legend>${I18N.t("int.base.mode")}</legend>${["edges", "zero", "line"].map(opt).join("")}</fieldset>
    <fieldset class="pk-sec" id="ib-pts"><legend>${I18N.t("int.base.line")}</legend>${num("t0", 0)}${num("y0", 1)}${num("t1", 2)}${num("y1", 3)}
    <label class="ib-opt"><input type="checkbox" id="ib-lvl"${it.lvl ? " checked" : ""}> ${I18N.t("int.base.level")}</label></fieldset>
    <div id="ib-msg" class="fail sm" role="status" aria-live="polite"></div><div class="pk-act"><button id="ib-ok" type="button" class="go">${I18N.t("int.base.apply")}</button></div></div>`, () => {
    const pts = Q("#ib-pts"), sel = () => Q("#bigbody input[name=ib-m]:checked").value;
    const sync1 = () => { pts.disabled = sel() !== "line"; };
    Q("#bigbody").querySelectorAll("input[name=ib-m]").forEach(r => { r.onchange = sync1; }); sync1();
    Q("#ib-ok").onclick = () => {
      const v = [...Q("#bigbody").querySelectorAll("[data-ib]")].map(i => numIt(i.value)), k = sel(), lvl = Q("#ib-lvl").checked;
      if (k === "line" && !v.every(Number.isFinite)) { Q("#ib-msg").textContent = I18N.t("int.base.bad"); return; }   // in the modal: a toast would sit behind it
      pushIh(p); delete it.zero;
      if (k === "edges") { delete it.bm; delete it.bl; delete it.lvl; }
      else if (k === "zero") { it.bm = "zero"; delete it.bl; delete it.lvl; }
      else { it.bm = "line"; it.bl = lvl ? [v[0], v[1], v[2], v[1]] : v; if (lvl) it.lvl = true; else delete it.lvl; }
      Q("#bigdlg").close(); uiSave(); draw(p);
    };
  });
}
{                                                                  // the look of the two point buttons (colours from the CSS variables)
  const st = document.createElement("style");
  st.textContent = ".ibp{position:absolute;z-index:3;width:24px;height:24px;margin:-12px 0 0 -12px;padding:0;border:0;border-radius:50%;background:transparent;touch-action:none;cursor:move}" +
    ".ibp:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.ibp.on{background:color-mix(in srgb,var(--accent) 25%,transparent)}.ib-opt{display:flex;align-items:center;gap:2px;min-height:24px;margin:.2em 0;width:fit-content}.ib-dlg .pk-sec+.pk-sec{margin-top:8px}#bigdlg:has(.ib-dlg){width:min(38rem,95vw)}.ib-dlg .pk-row input{grid-row:1;width:9ch;min-width:100%}@supports (grid-template-columns:subgrid){.ib-dlg .pk-sec:has(>.pk-row){grid-template-columns:fit-content(18rem) 11ch}}" +
    "@media (forced-colors:active){.ibp:focus-visible{outline-color:Highlight}.ibp.on{background:Highlight}}";
  document.head.appendChild(st);
}
const INTBASE = { area: ibArea, paint: ibPaint, sync: ibSync, open: ibOpen, lineAt: ibLine };
