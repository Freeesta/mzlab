// Reference of the session (a blank or the t0): one gesture, the same difference on traces, spectrum and map. Module; the page uses window.SUBREF.
// The state is S.ref = { on: { [tab]: bool }, k: "auto" | file number, factor, norm: "none" | "tic" | "istd", istd: m/z, rttol: min, clip }.
// The program only shows the difference (sample minus reference): it names nothing (principle 1). The calculation is in mzlab/differenza.py.
const T = (k, p) => I18N.t(k, p);
const KINDS = ["chrom", "xic", "mrm", "spec", "map"];
const R = () => S.ref;
const num = v => (+v).toLocaleString(typeof LOCALE === "string" ? LOCALE : undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const fileOf = k => (k != null && k !== "" && E.files[+k] && !E.files[+k].gone ? E.files[+k] : null);

// the file chosen by "auto" in a tab: the only blank of the tab, otherwise the first by time (files without a time come after those with one)
function autoFile(tab) {
  const fs = tabFiles(tab); if (!fs.length) return null;
  const bl = fs.filter(f => f.type === "blank"); if (bl.length === 1) return bl[0];
  const timed = fs.filter(f => f.time != null && isFinite(f.time));
  return timed.length ? timed.reduce((a, b) => (b.time < a.time ? b : a)) : fs[0];
}
// the reference that applies to a panel: its own choice (menu «Correz.», «fondo», «differenza») wins, otherwise the one of the session when it is on for the tab
function eff(p) {
  if (!p || !KINDS.includes(p.type)) return null;
  const tab = p.tab || E.tab, r = R();
  if (p.type === "chrom" && (tab === "ms2" || (p.ranges && p.ranges.length))) return null;       // precursor chromatograms and the stacked graphs of the bench have their own logic
  if (p.type === "spec" && p.bg === "x") return null;                            // «none» chosen in the panel while the session has a reference
  const own = p.type === "spec" ? (p.bg !== "w" ? p.bg : null) : p.type === "map" ? p.ref : p.type === "mrm" ? null : p.bk;
  let f = fileOf(own), auto = false;
  if (!f && r.on[tab]) { f = r.k === "auto" ? autoFile(tab) : fileOf(r.k) || autoFile(tab); auto = r.k === "auto"; }
  if (!f) return null;
  return { k: f.k, file: f, auto, factor: r.factor, norm: r.norm, istd: r.istd, rttol: r.rttol, clip: r.clip };
}
const qs = o => `&ref=${o.k}&factor=${o.factor}&norm=${o.norm}${o.norm === "istd" ? `&istd=${o.istd}` : ""}&rttol=${o.rttol}&clip=${o.clip ? 1 : 0}`;
const normText = o => (o.norm === "tic" ? T("sub.norm.tic") : o.norm === "istd" ? T("sub.norm.istd.short", { mz: o.istd }) : T("sub.norm.none"));
// «area on the trace minus {file} × {f}»: the note of the integration table and of the Excel sheets
const text = p => { const o = eff(p); return o ? T("sub.note", { file: o.file.label, f: num(o.factor), norm: normText(o) }) : ""; };

let live = null;
function say(t) {                                                             // announced politely: the chip shows the state, no modal dialog
  if (!live) { live = document.createElement("div"); live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); live.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap"; document.body.appendChild(live); }
  live.textContent = ""; setTimeout(() => { live.textContent = t; }, 30);
}
let timer = 0;
function refresh(tab, now) {                                                 // the preview follows the control after 150 ms
  clearTimeout(timer);
  const run = () => { E.panels.filter(p => (p.tab || E.tab) === tab && KINDS.includes(p.type)).forEach(p => { ctl(p); draw(p); }); uiSave(); };
  if (now) run(); else timer = setTimeout(run, 150);
}
function clearOwn(tab) {
  E.panels.filter(p => (p.tab || E.tab) === tab).forEach(p => {
    if (p.type === "spec") { if (p.bg !== "w") p.bg = ""; } else if (p.type === "map") p.ref = ""; else if (p.type !== "mrm") p.bk = "";
  });
}
function set(tab, on) {
  const r = R(); r.on[tab] = !!on;
  if (!on) clearOwn(tab);
  refresh(tab, true);
  const o = on ? eff(E.panels.find(p => (p.tab || E.tab) === tab && KINDS.includes(p.type))) : null;
  if (on && !o) { r.on[tab] = false; info(T("sub.none")); }
  else if (on) say(T("sub.on", { file: o.file.label, f: num(o.factor) }));
  else say(T("sub.off"));
  if (window.BARRA && BARRA.refresh) BARRA.refresh();
}
const isOn = (tab = E.tab) => !!R().on[tab] || E.panels.some(p => (p.tab || E.tab) === tab && eff(p));
const toggle = (tab = E.tab) => set(tab, !isOn(tab));

// ◀ ▶: the file shown goes through the files of the tab (t1 … tn) while the reference stays; the reference itself is skipped
function step(p, d) {
  const o = eff(p), tab = p.tab || E.tab, fs = tabFiles(tab).filter(f => !o || f.k !== o.k); if (fs.length < 1) return;
  E.browse = true;
  const i = fs.findIndex(f => f.k === E.cur);
  E.cur = fs[((i < 0 ? (d > 0 ? -1 : 0) : i) + d + fs.length) % fs.length].k;
  renderNav(); renderFileList(); redrawAll(); uiSave();
}

// ---- the chip in the legend of a panel and its popover (one popover for the page, positioned under the chip)
let pop = null, popP = null;
function popClose() { if (pop) pop.hidden = true; popP = null; }
function popOpen(p, anchor) {
  if (!pop) {
    pop = document.createElement("div"); pop.id = "subpop"; pop.className = "subpop"; pop.setAttribute("role", "dialog"); pop.hidden = true; document.body.appendChild(pop);
    document.addEventListener("keydown", e => { if (e.key === "Escape" && !pop.hidden) { const a = pop._anchor; popClose(); if (a && a.isConnected) a.focus(); } });
    document.addEventListener("mousedown", e => { if (!pop.hidden && !e.target.closest("#subpop") && !e.target.closest(".subchip")) popClose(); });
  }
  popP = p; pop._anchor = anchor; popBuild(p);
  pop.hidden = false;
  const b = anchor.getBoundingClientRect();
  pop.style.left = Math.max(4, Math.min(b.left, innerWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = Math.min(b.bottom + 4, Math.max(4, innerHeight - pop.offsetHeight - 6)) + "px";
  (pop.querySelector("select,input") || pop).focus();
}
function popBuild(p) {
  const r = R(), tab = p.tab || E.tab, o = eff(p), fs = tabFiles(tab), au = autoFile(tab);
  const opt = (v, cur, lab) => `<option value="${EH(v)}"${String(cur) === String(v) ? " selected" : ""}>${EH(lab)}</option>`;
  const curK = o && !o.auto ? o.k : r.k;
  pop.setAttribute("aria-label", T("sub.pop.title"));
  pop.innerHTML = `<div class="spr"><label for="sp-file">${T("sub.file")}</label><select id="sp-file">${opt("auto", o && o.auto ? "auto" : curK === "auto" ? "auto" : "", T("sub.file.auto", { file: au ? au.label : "—" }))}${fs.map(f => opt(f.k, curK, f.label)).join("")}</select></div>
    <div class="spr"><label for="sp-fr">${T("sub.factor")}</label><input id="sp-fr" type="range" min="0" max="2" step="0.05" value="${r.factor}" aria-valuetext="${EH(T("sub.factor.vt", { f: num(r.factor) }))}"><input id="sp-fn" type="number" min="0" max="2" step="0.05" value="${r.factor}" aria-label="${EH(T("sub.factor"))}"></div>
    <div class="spr"><label for="sp-norm">${T("sub.norm")}</label><select id="sp-norm">${opt("none", r.norm, T("sub.norm.none"))}${opt("tic", r.norm, T("sub.norm.tic.long"))}${opt("istd", r.norm, T("sub.norm.istd"))}</select>
      <input id="sp-istd" class="mzf" inputmode="decimal" autocomplete="off" value="${EH(r.istd)}" placeholder="m/z" aria-label="${EH(T("sub.istd"))}"${r.norm === "istd" ? "" : " hidden"}></div>
    <div class="spr"><label for="sp-rt">${T("sub.rttol")}</label><input id="sp-rt" type="number" min="0" max="2" step="0.01" value="${r.rttol}"></div>
    <div class="spr"><label><input id="sp-clip" type="checkbox"${r.clip ? " checked" : ""}> ${T("sub.clip")}</label></div>
    <div class="spr"><button type="button" id="sp-ok">${T("sub.close")}</button></div>`;
  const $ = id => pop.querySelector("#" + id), upd = () => refresh(tab);
  $("sp-file").onchange = e => { r.k = e.target.value === "auto" ? "auto" : +e.target.value; r.on[tab] = true; clearOwn(tab); upd(); };
  const setF = v => { v = Math.max(0, Math.min(2, Math.round((+v || 0) * 100) / 100)); r.factor = v; $("sp-fr").value = v; $("sp-fn").value = v; $("sp-fr").setAttribute("aria-valuetext", T("sub.factor.vt", { f: num(v) })); r.on[tab] = true; upd(); };
  $("sp-fr").oninput = e => setF(e.target.value);
  $("sp-fn").onchange = e => setF(e.target.value);
  $("sp-norm").onchange = e => { r.norm = e.target.value; $("sp-istd").hidden = r.norm !== "istd"; if (r.norm === "istd" && !r.istd) { $("sp-istd").focus(); return; } r.on[tab] = true; upd(); };
  $("sp-istd").onchange = e => { r.istd = e.target.value.trim().replace(",", "."); if (r.norm === "istd" && !(+r.istd > 0)) { r.norm = "none"; $("sp-norm").value = "none"; $("sp-istd").hidden = true; } r.on[tab] = true; upd(); };
  $("sp-rt").onchange = e => { r.rttol = Math.max(0, Math.min(2, +e.target.value || 0)); r.on[tab] = true; upd(); };
  $("sp-clip").onchange = e => { r.clip = e.target.checked; r.on[tab] = true; upd(); };
  $("sp-ok").onclick = () => { const a = pop._anchor; popClose(); if (a && a.isConnected) a.focus(); };
}

function chip(p) {
  const o = eff(p); if (!o || !p.leg) return;
  const tab = p.tab || E.tab, many = tabFiles(tab).length > 2;
  const c = document.createElement("span"); c.className = "subchip"; c.setAttribute("role", "group"); c.setAttribute("aria-label", T("sub.chip.aria"));
  const lab = T("sub.chip", { file: o.file.label + (o.auto ? T("sub.auto") : ""), f: num(o.factor), norm: normText(o) });
  c.innerHTML = `${many ? `<button type="button" data-s="prev" aria-label="${EH(T("sub.prev", { file: o.file.label }))}" title="${EH(T("sub.prev", { file: o.file.label }))}">&#9664;</button>` : ""}<button type="button" data-s="pop" aria-haspopup="dialog" title="${EH(T("sub.chip.title"))}">${EH(lab)} &#9662;</button>${many ? `<button type="button" data-s="next" aria-label="${EH(T("sub.next", { file: o.file.label }))}" title="${EH(T("sub.next", { file: o.file.label }))}">&#9654;</button>` : ""}<button type="button" data-s="off" aria-label="${EH(T("sub.off.aria"))}" title="${EH(T("sub.off.aria"))}">&times;</button>`;
  c.querySelector("[data-s=pop]").onclick = e => { if (pop && !pop.hidden && popP === p) popClose(); else popOpen(p, e.currentTarget); };
  const pv = c.querySelector("[data-s=prev]"), nx = c.querySelector("[data-s=next]");
  if (pv) pv.onclick = () => step(p, -1);
  if (nx) nx.onclick = () => step(p, 1);
  c.querySelector("[data-s=off]").onclick = () => { popClose(); set(tab, false); };
  p.leg.appendChild(c);
}

window.SUBREF = { eff, qs, text, chip, toggle, set, isOn, autoFile, step, normText, num, refresh };
