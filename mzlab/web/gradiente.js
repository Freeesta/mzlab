// LC gradient (%B) of a file: read from the .dam by the server, or typed / pasted / imported by the student here (a .raw script, a CSV, the course method).
// mzML files carry no gradient; Thermo .raw files keep it as UTF-16 text of the Chromeleon script (the first ~2 MB are read in the browser, the file is neither
// opened nor copied). The model is {source, points: [{t, b, flow?, curve?}], run_time?, solvents?}; the server keeps it per file (api/gradient) and the notebook
// (NB.gradienti), the .mzflow and qqq.gradiente.ultimo remember what the student entered. Classic globals are used as they are (E, J, post, NB, I18N, EH).
const LAST_KEY = "qqq.gradiente.ultimo";
const cache = new Map();                                     // file name -> model | null (what the server answered)
const nameOf = f => f.file.split("#")[0];
const T = (k, p) => I18N.t(k, p);

const lastRead = () => { try { return JSON.parse(localStorage.getItem(LAST_KEY) || "null"); } catch (_) { return null; } };
const lastWrite = m => { try { localStorage.setItem(LAST_KEY, JSON.stringify({ points: m.points, run_time: m.run_time })); } catch (_) { /* not saved */ } };

// The gradient of file f: .dam first (the server), then what was entered; one that the notebook remembers is given back to the server first.
async function get(f) {
  const n = nameOf(f);
  if (cache.has(n)) return cache.get(n);
  let g = null;
  try { g = (await J("api/gradient?k=" + f.k)).gradient; } catch (_) { /* no gradient */ }
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
const SRC = { dam: "grad.src.dam", raw: "grad.src.raw", manual: "grad.src.manual", csv: "grad.src.csv", course: "grad.src.course" };
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
    inp.onchange = async () => { const fl = inp.files[0]; if (fl) { try { await onText(fl); } catch (e) { err(e); } } };
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
        const t = (e.clipboardData || window.clipboardData).getData("text"); if (!/\n|\t/.test(t.trim())) return;
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
  on("#grad-csv", () => ask(".csv,.txt,.tsv", async fl => edit(await parse(await fl.text(), "csv"))));
  on("#grad-raw", () => ask(".raw,.RAW", async fl => {
    const buf = await fl.slice(0, 2 * 1024 * 1024).arrayBuffer();
    const dec = o => new TextDecoder("utf-16le").decode(buf.slice(o, o + ((buf.byteLength - o) & ~1)));
    const pick = [dec(0), dec(1)].map(s => s.replace(/[^\x20-\x7e°µ]+/g, "\n")).sort((a, b) => (b.match(/%B/g) || []).length - (a.match(/%B/g) || []).length)[0].slice(0, 400000);
    try { edit(await parse(pick, "chromeleon")); } catch (e) { if (e.key === "gradient.err.noScript") msg(T("grad.rawNone")); else throw e; }
  }));
}

const exportState = () => (NB.gradienti && Object.keys(NB.gradienti).length ? NB.gradienti : null);
function importState(o) { if (o && typeof o === "object") { NB.gradienti = { ...(NB.gradienti || {}), ...o }; forget(); nbSave(); } }

window.GRADIENTE = { get, set, html, bind, any, forget, exportState, importState };
