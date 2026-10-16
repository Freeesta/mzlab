// @ts-strict
// LC gradient (%B) of a file: read from the .dam by the server, or typed / pasted / imported by the student here (a .raw script, a CSV, the course method).
// mzML files carry no gradient; Thermo .raw files keep it as UTF-16 text of the Chromeleon script (the first ~2 MB are read in the browser, the file is neither
// opened nor copied). The model is {source, points: [{t, b, flow?, curve?}], run_time?, solvents?}; the server keeps it per file (api/gradient) and the notebook
// (NB.gradienti), the .mzflow and qqq.gradiente.ultimo remember what the student entered. Classic globals are used as they are (E, J, post, NB, I18N, EH).
const LAST_KEY = "qqq.gradiente.ultimo";
const cache = new Map();                                     // file name -> model | null (what the server answered)
const nameOf = f => f.file.split("#")[0];
const T = (k, p) => I18N.t(k, p);

const lastRead = () => { const o = lsObj(LAST_KEY); return Array.isArray(o.points) ? o : null; };
const lastWrite = m => { try { localStorage.setItem(LAST_KEY, JSON.stringify({ points: m.points, run_time: m.run_time })); } catch (_) { /* not saved */ } };

// The pump program that the file's own method document (metodo/1: .raw, .wiff, .D, Waters) carries, as a gradient model; null when it has fewer than two %B points.
async function fromMethod(f) {
  let lc = null;
  try { lc = (((await J("api/metodo?k=" + f.k)) || {}).metodo || {}).lc; } catch (_) { return null; }
  const pts = ((lc && lc.gradiente) || []).filter(q => q.t_min != null && q.B_pct != null).map(q => ({ t: +q.t_min, b: +q.B_pct, ...(q.flusso_mL_min != null ? { flow: +q.flusso_mL_min } : {}) }));
  return pts.length > 1 ? { source: "file", points: pts, run_time: lc.durata_min != null ? +lc.durata_min : pts[pts.length - 1].t } : null;
}
// The gradient of file f: .dam first (the server), then what was entered; one that the notebook remembers is given back to the server first.
async function get(f) {
  const n = nameOf(f);
  if (cache.has(n)) return cache.get(n);
  let g = null;
  try { g = (await J("api/gradient?k=" + f.k)).gradient; } catch (_) { /* no gradient */ }
  if (!g) g = await fromMethod(f);
  if (!g && NB.gradienti && NB.gradienti[n]) { try { g = (await post("api/gradient", { k: f.k, model: NB.gradienti[n] })).gradient; } catch (_) { /* stays null */ } }
  cache.set(n, g);
  return g;
}
async function set(f, model, all) {
  const ks = all ? tabFiles(f.kind).map(x => x.k) : [f.k];
  const g = (await post("api/gradient", { k: f.k, model, all: ks })).gradient;
  NB.gradienti = NB.gradienti || {};
  ks.forEach(k => { const x = E.files[k]; if (!x) return; const n = nameOf(x); cache.delete(n); if (g) NB.gradienti[n] = g; else delete NB.gradienti[n]; });
  if (g) lastWrite(g);
  nbSave();
  return g;
}
const any = () => [...cache.values()].some(Boolean);
const forget = () => cache.clear();
document.addEventListener("nbreset", () => { forget(); });

// ---- the section in the Method window (files without a gradient of their own)
const SRC = { dam: "grad.src.dam", raw: "grad.src.raw", file: "grad.src.file", manual: "grad.src.manual", csv: "grad.src.csv", course: "grad.src.course" };
let draft = null;                                            // the table being edited: [{t, b, flow}] as text
const row = (r, i) => `<tr><td><input data-c="t" data-i="${i}" inputmode="decimal" aria-label="${T("grad.col.time")}" value="${EH(r.t ?? "")}"></td><td><input data-c="b" data-i="${i}" inputmode="decimal" aria-label="% B" value="${EH(r.b ?? "")}"></td><td><input data-c="flow" data-i="${i}" inputmode="decimal" aria-label="${T("grad.col.flow")}" value="${EH(r.flow ?? "")}"></td><td><button type="button" class="bt" data-del="${i}" aria-label="${T("grad.delRow")}" title="${T("grad.delRow")}">&times;</button></td></tr>`;
const toDraft = g => (g ? g.points.map(p => ({ t: p.t, b: p.b, flow: p.flow ?? "" })) : [{ t: 0, b: "", flow: "" }, { t: "", b: "", flow: "" }]);

async function html(f) {
  const g = await get(f);
  const last = lastRead();
  let h = `<section class="msec" id="grad-sec"><h4>${T("grad.title")}</h4>`;
  if (g) {
    h += `<div class="sm" style="margin:0 0 6px">${T(SRC[g.source] || "grad.src.manual")}${g.nonlinear ? " · " + T("grad.nonlinear") : ""}</div>`;
    h += typeof lcHtml === "function" ? lcHtml({ gradient: g.points, run_time: g.run_time }, () => "") : "";
    h += `<div class="mchips" style="margin-top:6px"><button type="button" class="go" id="grad-edit">${T("grad.edit")}</button> <button type="button" class="go" id="grad-rm">${T("grad.remove")}</button></div>`;
  } else {
    h += `<div class="sm" style="margin:0 0 8px">${T("grad.none")}</div>`;
  }
  h += `<div class="mchips" style="margin-top:6px"><button type="button" class="go" id="grad-new">${T("grad.insert")}</button> <button type="button" class="go" id="grad-raw">${T("grad.rawImport")}</button> <button type="button" class="go" id="grad-csv">${T("grad.csvImport")}</button> <button type="button" class="go" id="grad-course">${T("grad.course")}</button>${last ? ` <button type="button" class="go" id="grad-last">${T("grad.last")}</button>` : ""}</div>`;
  h += `<div id="grad-ed" hidden></div><div id="grad-msg" class="sm" role="status" aria-live="polite" style="margin-top:6px"></div></section>`;
  return h;
}

// buttons of the section; again() reopens the Method window after a change
function bind(f, again) {
  const sec = Q("#grad-sec"); if (!sec) return;
  const msg = t => { Q("#grad-msg").textContent = t || ""; };
  const err = e => msg(e.key ? T(e.key, e.params) : e.message);
  const save = async (model, all) => { try { await set(f, model, all); again(); } catch (e) { err(e); } };
  const ask = (accept, onText, binary) => {
    const inp = document.createElement("input"); inp.type = "file"; inp.accept = accept;
    inp.onchange = async () => { const fl = /** @type {FileList} */ (inp.files)[0]; if (fl) { try { await onText(fl); } catch (e) { err(e); } } };
    inp.click();
  };
  const parse = async (text, fmt) => (await post("api/gradient_parse", { text, fmt })).gradient;
  const edit = g => {                                       // the editable table, pasting from Excel allowed
    draft = toDraft(g);
    const ed = Q("#grad-ed"); ed.hidden = false;
    const paint = () => {
      ed.innerHTML = `<table class="lct"><tr><th class="num">${T("grad.col.time")}</th><th class="num">% B</th><th class="num">${T("grad.col.flow")}</th><th></th></tr>${draft.map(row).join("")}</table>
        <div class="mchips" style="margin-top:6px"><button type="button" class="go" id="grad-add">${T("grad.addRow")}</button> <button type="button" class="go" id="grad-ok">${T("grad.apply")}</button> <label><input type="checkbox" id="grad-all"> ${T("grad.applyAll")}</label></div><div class="sm muted" style="margin-top:4px">${T("grad.paste")}</div>`;
      ed.querySelectorAll("input[data-c]").forEach(i => { i.oninput = () => { draft[+i.dataset.i][i.dataset.c] = i.value; }; });
      ed.querySelectorAll("button[data-del]").forEach(b => { b.onclick = () => { draft.splice(+b.dataset.del, 1); paint(); }; });
      Q("#grad-add").onclick = () => { draft.push({ t: "", b: "", flow: "" }); paint(); };
      Q("#grad-ok").onclick = () => {
        const pts = draft.filter(r => String(r.t).trim() !== "" || String(r.b).trim() !== "").map(r => ({ t: r.t, b: r.b, flow: r.flow === "" ? null : r.flow }));
        save({ source: "manual", points: pts.map(p => ({ t: String(p.t).replace(",", "."), b: String(p.b).replace(",", "."), flow: p.flow == null ? null : String(p.flow).replace(",", ".") })) }, Q("#grad-all").checked);
      };
      ed.onpaste = async e => {                              // a block copied from Excel: time TAB %B [TAB flow] per line
        const t = (e.clipboardData || /** @type {any} */ (window).clipboardData).getData("text"); if (!/\n|\t/.test(t.trim())) return;
        e.preventDefault();
        try { draft = toDraft(await parse(t, "csv")); paint(); } catch (er) { err(er); }
      };
    };
    paint();
  };
  const on = (id, fn) => { const b = Q(id); if (b) b.onclick = fn; };
  on("#grad-new", () => edit(null));
  on("#grad-edit", async () => edit(await get(f)));
  on("#grad-rm", () => save(null));
  on("#grad-course", async () => { try { await save(await parse("", "course")); } catch (e) { err(e); } });
  on("#grad-last", () => { const l = lastRead(); if (l) save({ source: "manual", points: l.points, run_time: l.run_time }); });
  on("#grad-csv", () => ask(".csv,.txt,.tsv", async fl => edit(await parse(await FILESICURI.readText(fl), "csv"))));
  on("#grad-raw", () => ask(".raw,.RAW", async fl => {
    const buf = await fl.slice(0, 2 * 1024 * 1024).arrayBuffer();
    const dec = o => new TextDecoder("utf-16le").decode(buf.slice(o, o + ((buf.byteLength - o) & ~1)));
    const pick = [dec(0), dec(1)].map(s => s.replace(/[^\x20-\x7e°µ]+/g, "\n")).sort((a, b) => (b.match(/%B/g) || []).length - (a.match(/%B/g) || []).length)[0].slice(0, 400000);
    try { edit(await parse(pick, "chromeleon")); } catch (e) { if (e.key === "gradient.err.noScript") msg(T("grad.rawNone")); else throw e; }
  }));
}


// ---- the gradient drawn over a chromatogram: right axis 0-100 %B, ramps, area at 10 %, dashed (never colour alone)
const pending = new Set();
const filesOf = p => (typeof shownFor === "function" ? shownFor(p) : []);
const key = m => JSON.stringify(m.points);
// the gradients of the files of a panel are fetched once; when one arrives the panels are drawn again
function ensure(p) {
  if (p.overlay === "press") pressEnsure(p);
  filesOf(p).forEach(f => {
    const n = nameOf(f); if (cache.has(n) || pending.has(n)) return;
    pending.add(n);
    get(f).then(g => { pending.delete(n); if (g && typeof redrawSoon === "function") redrawSoon(); }, () => pending.delete(n));
  });
}
const have = p => filesOf(p).some(f => cache.get(nameOf(f)));
// one curve if the gradients of the shown files are equal, else the one of the current file, with a note
function curve(p) {
  const fs = filesOf(p), ms = fs.map(f => ({ f, m: cache.get(nameOf(f)) })).filter(x => x.m);
  if (!ms.length) return null;
  const same = ms.every(x => key(x.m) === key(ms[0].m));
  if (same && ms.length === fs.length) return { m: ms[0].m, note: null };
  const cur = ms.find(x => x.f === E.files[E.cur]) || ms[0];
  return { m: cur.m, note: fs.length > 1 ? T("grad.legend.one", { file: cur.f.label }) : null };
}
function at(m, t) {                                          // {b, flow?} on the straight ramps; null outside the programmed time
  const pts = m.points; if (!pts.length || t < pts[0].t || t > pts[pts.length - 1].t) return null;
  /** @type {{ b: number, flow?: number } | null} */ let r = null;
  for (let i = 0; i < pts.length - 1; i++) {
    const a = pts[i], b = pts[i + 1]; if (t < a.t || t > b.t) continue;
    const w = b.t === a.t ? 1 : (t - a.t) / (b.t - a.t);
    r = { b: a.b + w * (b.b - a.b) }; if (a.flow != null && b.flow != null) r.flow = a.flow + w * (b.flow - a.flow);
  }
  return r;
}
const delayOf = p => p.gdelay || 0;
function reading(p, t) {                                     // the values at time t (programmed and at the column)
  const c = curve(p); if (!c || p.overlay !== "grad") return null;
  const d = delayOf(p), prog = at(c.m, t), col = d ? at(c.m, t - d) : null;
  return prog || col ? { b: prog ? prog.b : null, flow: prog ? prog.flow : null, bcol: col ? col.b : null } : null;
}
function colorOf(p) {
  const used = new Set(filesOf(p).map(f => String(f.color || "").toLowerCase()));
  const dark = document.documentElement.dataset.theme === "dark" || (document.documentElement.dataset.theme !== "light" && matchMedia("(prefers-color-scheme: dark)").matches);
  const first = css("--grad").toLowerCase();
  if (!used.has(first)) return css("--grad");
  const c = (dark ? OKABE_DARK : OKABE).find(x => !used.has(x.toLowerCase()));
  return c || css("--muted");
}
function needR(p) { return (p.overlay === "grad" && have(p)) || (p.overlay === "press" && pressHave(p)) ? Math.round(46 * (typeof fz === "function" ? fz() : 1)) : 0; }

// ---- the pump pressure over a chromatogram (same right axis as the gradient, one or the other); in bar, one curve per file in its colour
const PRESS = new Map();                                     // "k|i" -> the answer of api/analog
const pressTrace = f => (f.analog || []).find(a => a.kind === "pressure");
const pressHave = p => filesOf(p).some(f => pressTrace(f));
function pressEnsure(p) {
  filesOf(p).forEach(f => {
    const a = pressTrace(f); if (!a) return; const k = f.k + "|" + a.i; if (PRESS.has(k)) return;
    PRESS.set(k, null);
    J(`api/analog?k=${f.k}&i=${a.i}&n=2000`).then(d => { PRESS.set(k, d); if (typeof redrawSoon === "function") redrawSoon(); }, () => {});
  });
}
const pressCurves = p => filesOf(p).map(f => ({ f, d: PRESS.get(f.k + "|" + (pressTrace(f) || {}).i) })).filter(x => x.d && x.d.t.length);
const pressMax = cs => { let m = 0; cs.forEach(c => c.d.y.forEach(v => { if (v > m) m = v; })); return m > 0 ? m * 1.08 : 1; };
function drawPress(p, g, a) {
  const cs = pressCurves(p); if (!cs.length) return;
  const { W, H, X } = a, ph = H - M.t - M.b, top = pressMax(cs), Y = v => H - M.b - v / top * ph, z = typeof fz === "function" ? fz() : 1, ink = css("--muted");
  g.save(); g.strokeStyle = ink; g.fillStyle = ink; g.lineWidth = 1; g.font = fpx(12); g.textAlign = "left";
  g.beginPath(); g.moveTo(W - M.r + .5, M.t); g.lineTo(W - M.r + .5, H - M.b); g.stroke();
  for (const v of nice(0, top, 5)) { g.beginPath(); g.moveTo(W - M.r, Math.round(Y(v)) + .5); g.lineTo(W - M.r + 4, Math.round(Y(v)) + .5); g.stroke(); g.fillText(String(+v.toFixed(1)), W - M.r + 7, Y(v) + 3.5 * z); }
  g.save(); g.translate(W - 4, (M.t + H - M.b) / 2); g.rotate(Math.PI / 2); g.textAlign = "center"; g.fillText("bar", 0, 0); g.restore();
  g.beginPath(); g.rect(M.l, M.t, W - M.l - M.r, ph); g.clip();
  cs.forEach(({ f, d }) => { g.strokeStyle = f.color; g.lineWidth = 1.6; g.setLineDash([6, 3]); g.beginPath(); d.t.forEach((t, i) => (i ? g.lineTo(X(t), Y(d.y[i])) : g.moveTo(X(t), Y(d.y[i])))); g.stroke(); });
  g.restore();
  const note = cs.find(c => c.d.unit_note); if (note) { g.save(); g.font = fpx(11); g.fillStyle = css("--muted"); g.textAlign = "right"; g.fillText(T(note.d.unit_note.key, note.d.unit_note.params), W - M.r - 6, M.t + 14 * z); g.restore(); }
  a.press = true;
}
function pressAt(d, t) { if (!d.t.length || t < d.t[0] || t > d.t[d.t.length - 1]) return null; let lo = 0, hi = d.t.length - 1; while (hi - lo > 1) { const m = (lo + hi) >> 1; d.t[m] <= t ? lo = m : hi = m; } return d.y[Math.abs(d.t[lo] - t) <= Math.abs(d.t[hi] - t) ? lo : hi]; }
function hoverPress(p, t, py) {
  const a = p._a, cs = pressCurves(p); if (!a || !cs.length) return "";
  const ph = a.H - M.t - M.b, top = pressMax(cs);
  return cs.map(({ f, d }) => { const v = pressAt(d, t); return v != null && Math.abs(a.H - M.b - v / top * ph - py) <= 8 ? `<div><i style="background:${f.color}"></i>${T("press.read", { t: t.toFixed(2), p: v.toFixed(0) })}</div>` : ""; }).join("");
}
function draw(p, g, a) {
  if (p.overlay === "press") return drawPress(p, g, a);
  const c = curve(p); if (!c) return;
  const { W, H, X, x0, x1 } = a, ph = H - M.t - M.b, Y = v => H - M.b - v / 100 * ph, col = colorOf(p), d = delayOf(p), z = typeof fz === "function" ? fz() : 1;
  g.save();
  g.strokeStyle = col; g.fillStyle = col; g.lineWidth = 1; g.font = fpx(12);
  g.beginPath(); g.moveTo(W - M.r + .5, M.t); g.lineTo(W - M.r + .5, H - M.b); g.stroke();                       // the right axis
  g.textAlign = "left";
  for (const v of [0, 25, 50, 75, 100]) { g.beginPath(); g.moveTo(W - M.r, Math.round(Y(v)) + .5); g.lineTo(W - M.r + 4, Math.round(Y(v)) + .5); g.stroke(); g.fillText(String(v), W - M.r + 7, Y(v) + 3.5 * z); }
  g.save(); g.translate(W - 4, (M.t + H - M.b) / 2); g.rotate(Math.PI / 2); g.textAlign = "center"; g.fillText("%B", 0, 0); g.restore();
  g.beginPath(); g.rect(M.l, M.t, W - M.l - M.r, ph); g.clip();
  const trace = (shift, w, dash, area) => {
    const pts = c.m.points.map(q => [X(q.t + shift), Y(q.b)]);
    g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
    g.setLineDash(dash); g.lineWidth = w; g.stroke(); g.setLineDash([]);
    if (area) { g.lineTo(pts[pts.length - 1][0], Y(0)); g.lineTo(pts[0][0], Y(0)); g.closePath(); g.globalAlpha = 0.1; g.fill(); g.globalAlpha = 1; }
  };
  if (d) { g.globalAlpha = 0.75; trace(0, 1, [2, 3], false); g.globalAlpha = 1; trace(d, 2, [], true); }       // programmed: thin dots; at the column: full
  else trace(0, 2, [6, 3], true);
  g.restore();
  if (c.note) { g.save(); g.font = fpx(11); g.fillStyle = css("--muted"); g.textAlign = "right"; g.fillText(c.note, W - M.r - 6, M.t + 14 * z); g.restore(); }
  a.grad = true;
}
// the box that follows the mouse near the curve (8 px)
function hover(p, t, py) {
  if (p.overlay === "press") return hoverPress(p, t, py);
  const r = /** @type {any} */ (reading(p, t)), a = p._a; if (!r || !a) return "";
  const ph = a.H - M.t - M.b, Y = v => a.H - M.b - v / 100 * ph, vals = [r.b, r.bcol].filter(v => v != null);
  if (!vals.some(v => Math.abs(Y(v) - py) <= 8)) return "";
  return `<div><i style="background:${colorOf(p)}"></i>${T("grad.read", { t: t.toFixed(2), b: vals.length === 2 ? `${r.b.toFixed(1)} / ${r.bcol.toFixed(1)}` : vals[0].toFixed(1) })}${r.flow != null ? " · " + r.flow.toFixed(2) + " mL/min" : ""}</div>`;
}
const cursorText = p => { if (p.overlay === "press") { const c = pressCurves(p)[0], v = c && p.cur != null ? pressAt(c.d, p.cur) : null; return v != null ? " · " + T("press.read.cursor", { p: v.toFixed(0) }) : ""; } const r = p.cur != null ? reading(p, p.cur) : null; return r ? " · " + T("grad.read.cursor", { b: /** @type {number} */ (r.bcol != null ? r.bcol : r.b).toFixed(1) }) : ""; };

const exportState = () => (NB.gradienti && Object.keys(NB.gradienti).length ? NB.gradienti : null);
function importState(o) { if (o && typeof o === "object") { NB.gradienti = { ...(NB.gradienti || {}), ...o }; forget(); nbSave(); } }

// the overlays of the chromatogram menu (registry azioni/1): the pressure trace and the gradient over the graph
if (typeof AZIONI !== "undefined") {
  /** @param {string} id @param {string} nome @param {string} aria @param {(c: any) => boolean} have */
  const over = (id, nome, aria, have) => AZIONI.registra({ id: "chrom." + id, gruppo: "view", viste: ["chrom", "xic", "mrm"], visibile: have, attiva: c => c.p.overlay === id,
    nome: () => T(nome), aria: () => T(aria), esegui: c => { c.p.overlay = c.p.overlay === id ? null : id; drawSoon(c.p); uiSave(); } });
  over("press", "ctx.chrom.press", "ctx.chrom.press.aria", c => pressHave(c.p) && !isAnalog(c.p.kind));
  over("grad", "ctx.chrom.grad", "ctx.chrom.grad.aria", c => have(c.p));
}
window.GRADIENTE = { pressHave, get, set, html, bind, any, forget, exportState, importState, ensure, have, curve, at, reading, draw, hover, needR, cursorText, colorOf };
