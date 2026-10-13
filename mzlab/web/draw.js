// "Disegno": Ketcher (structures, fragments, arrows, text). Bundled in web/vendor: no internet needed.
// The page only draws and labels: the student does the chemistry (no formula/adduct/fragment tables here, on purpose).
// OpenChemLib (also bundled) only turns what is drawn into SMILES and estimates properties of it (logP ...).
// Module script; helpers come from explore.js (window).
// OpenChemLib (1.1 MB) is fetched only when the first properties are computed (Disegno open, something drawn), not at page load
import { applyFont, mountFont } from "./disegno-font.js";
import { mountBar } from "./disegno-barra.js";
import { fragmentItems } from "./disegno-frammenti.js";
let OCL = null;
const loadOCL = async () => OCL || (OCL = await import("./vendor/openchemlib.js"));

const Q = s => document.querySelector(s);
let K = null, starting = null, restored = false, timer = null, startTimeout = null;

let kldTimer = null, kldN = 0;
function start() {
  if (starting) return starting;
  const fr = Q("#kframe");
  starting = new Promise((resolve, reject) => {
    const on = e => { if (e.source === fr.contentWindow && e.data && e.data.type === "ketcher-ready") { removeEventListener("message", on); clearTimeout(startTimeout); K = fr.contentWindow.ketcher; resolve(K); } };
    addEventListener("message", on);
    Q("#kload").hidden = false;
    const dots = Q("#kld-dots");
    if (dots) {
      kldN = 0; [...dots.children].forEach(d => { d.style.visibility = "hidden"; });
      clearInterval(kldTimer);
      kldTimer = setInterval(() => {
        kldN = (kldN + 1) % 4;
        [...dots.children].forEach((d, i) => { d.style.visibility = i < kldN ? "visible" : "hidden"; });
      }, 400);
    }
    fr.src = "static/vendor/ketcher/index.html";
    // a slow first download must not break the editor for good: after the wait the attempt is dropped and the next call (Quick draw, export, the tab) starts it again
    clearTimeout(startTimeout);
    startTimeout = setTimeout(() => { removeEventListener("message", on); starting = null; reject(new Error(I18N.t("draw.error.ketcherNotStarted"))); }, 60000);
  });
  starting.finally(() => { clearInterval(kldTimer); Q("#kload").hidden = true; }).catch(() => {});
  starting = starting.then(async k => { await window.nbEnsure(); await restore(); k.editor.subscribe("change", () => { clearTimeout(timer); timer = setTimeout(changed, 500); FRAME.next(drawLabels); });
    k.editor.subscribe("selectionChange", () => FRAME.next(showInfo));
    hideMacro(fr); applyFont(k); fitZoom(k, fr); changed(); drawLabels(); frameExtras(fr); dnote("");
    [150, 700].forEach(ms => setTimeout(drawLabels, ms));                 // Ketcher redraws its canvas after the first fit: the labels (formula, mass) of the saved options are put back on
    
    const doc = fr.contentDocument;
    if (doc && doc.body) {
      doc.body.addEventListener("dragover", e => { e.preventDefault(); e.stopPropagation(); });
      doc.body.addEventListener("drop", async e => {
        e.preventDefault(); e.stopPropagation();
        const file = e.dataTransfer.files[0];
        let txt = "";
        if (file) txt = await file.text();
        else txt = e.dataTransfer.getData("text/plain");
        if (txt) {
          try {
            const before = await k.getKet();
            await k.addFragment(txt);
            if (await k.getKet() === before) dnote(I18N.t("draw.import.unsupported"));
            else { fitZoom(k, fr); if (window.toast) toast(I18N.t("draw.import.done")); dnote(""); }
          } catch (err) { dnote(I18N.t("draw.import.failed", { message: err.message })); }
        }
      });
    }
    
    return k; });
  starting.catch(e => dnote(e.message));
  return starting;
}
// Ketcher's macromolecule mode (peptides, RNA, DNA) is not needed here and confuses: its switch is hidden
let kBar = null;                                                           // drop-down toolbar (disegno-barra.js), null = Ketcher's own bars
let kFit = () => {};                                                       // re-fits the side toolbars of Ketcher (set by hideMacro)
function hideMacro(fr) {
  try {
    const d = fr.contentDocument, st = d.createElement("style");
    // the buttons of Ketcher are small (32 px): the four toolbars are enlarged (CSS zoom, so the menus that open from them grow too). The size follows the room:
    // the side toolbars need ~655 px of height at normal size, the top one ~910 px of width (its help / about buttons are hidden: the program has its own help)
    st.textContent = '[data-testid="help-button"],[data-testid="about-button"]{display:none!important}' +
      '[class*="App-module_top"]{zoom:var(--ktz,1.1);flex-wrap:nowrap!important}' +
      '[class*="App-module_top"] kbd{display:none!important}' +
      '[data-testid="fullscreen-mode-button"],[data-testid="center-struct-button"]{display:inline-flex!important;align-items:center;justify-content:center}' +
      '[data-testid="fullscreen-mode-button"] svg,[data-testid="center-struct-button"] svg{margin:0!important;display:block}' +
      '[class*="LeftToolbar-module_root"],[class*="RightToolbar-module_root"],[class*="BottomToolbar-module_root"]{zoom:var(--ksz,1.3)}' +
      // the quick rings (Ketcher's bottom bar) become a column on the far left: the bottom row disappears and the canvas gets the whole height
      '[class*="App-module_app"]{grid-template-columns:auto auto minmax(0,1fr) auto!important;grid-template-rows:auto minmax(0,1fr)!important;' +
      'grid-template-areas:"toolbar-top toolbar-top toolbar-top toolbar-top" "toolbar-bottom toolbar-left canvas toolbar-right"!important}' +
      '[class*="App-module_app"] [class*="BottomToolbar-module_root"]{flex-direction:column!important;flex-wrap:nowrap!important;align-self:start;height:auto!important;width:auto!important;padding:8px 0 8px 8px!important;margin:0!important}' +
      '[class*="App-module_app"] [class*="BottomToolbar-module_group"]{flex-direction:column!important;height:auto!important;width:auto!important}' +
      '[class*="App-module_top"] [data-testid="select-drop-down-button"] { width: 28px !important; min-width: 28px !important; max-width: 28px !important; height: 28px !important; min-height: 28px !important; max-height: 28px !important; overflow: hidden !important; border-radius: 4px; align-self: center; padding: 0 !important; margin: 0 !important; display: block !important; box-sizing: border-box !important; flex-shrink: 0 !important; } ' +
      '[class*="App-module_top"] [data-testid="select-drop-down-button"] > button { width: 28px !important; min-width: 28px !important; max-width: 28px !important; height: 28px !important; min-height: 28px !important; max-height: 28px !important; padding: 0 !important; margin: 0 !important; border-radius: 4px; display: flex !important; align-items: center !important; justify-content: center !important; box-sizing: border-box !important; border: none !important; flex-shrink: 0 !important; } ' +
      '[class*="App-module_top"] [data-testid="select-drop-down-button"] > button svg { width: 16px !important; min-width: 16px !important; height: 16px !important; min-height: 16px !important; margin: 0 !important; padding: 0 !important; flex-shrink: 0 !important; display: block !important; position: static !important; } ' +
      '[class*="App-module_top"] [data-testid="select-drop-down-button"] > svg { display: none !important; width: 0 !important; height: 0 !important; opacity: 0 !important; } ' +
      '[class*="App-module_top"] [data-testid="hand"], [class*="App-module_top"] [data-testid="erase"], [class*="App-module_top"] [data-testid="text"] { width: 28px !important; min-width: 28px !important; max-width: 28px !important; height: 28px !important; min-height: 28px !important; max-height: 28px !important; display: flex !important; align-items: center !important; justify-content: center !important; align-self: center !important; border-radius: 4px !important; padding: 0 !important; margin: 0 !important; box-sizing: border-box !important; flex-shrink: 0 !important; } ';
      

    // Move Hand, Select, Erase, Text to top toolbar right after Cut
    const cutBtn = d.querySelector('[class*="App-module_top"] [data-testid="cut-button"]');
    const hrCut = cutBtn ? cutBtn.nextElementSibling : null;
    const hand = d.querySelector('[data-testid="left-toolbar-buttons"] [data-testid="hand"]');
    const selectDrop = d.querySelector('[data-testid="left-toolbar-buttons"] [data-testid="select-drop-down-button"]');
    const erase = d.querySelector('[data-testid="left-toolbar-buttons"] [data-testid="erase"]');
    const text = d.querySelector('[data-testid="left-toolbar-buttons"] [data-testid="text"]');

    if (hrCut && hand && selectDrop && erase && text) {
      const hrNew = d.createElement("hr"); hrNew.className = hrCut.className;
      hrCut.after(hrNew);
      hrCut.after(text);
      hrCut.after(erase);
      hrCut.after(selectDrop);
      hrCut.after(hand);
    }
    const left = d.querySelector('[class*="LeftToolbar-module_buttons"]');
    if (left) {
      [...left.children].forEach(g => { if (!g.querySelector("[data-testid]")) g.remove(); });
    }

    // Add Center Structure button next to fullscreen button in top bar
    const fullBtn = d.querySelector('[class*="App-module_top"] [data-testid="fullscreen-mode-button"]');
    if (fullBtn && !d.querySelector('[data-testid="center-struct-button"]')) {
      const btn = d.createElement("button");
      btn.setAttribute("data-testid", "center-struct-button");
      btn.setAttribute("title", I18N.t("draw.center.title"));
      btn.className = fullBtn.className;
      btn.innerHTML = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="7"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/><circle cx="12" cy="12" r="1.5" fill="currentColor"/></svg>';
      btn.onclick = () => {
        const st = K && K.editor && K.editor.struct();
        if (st && !st.isBlank() && typeof K.editor.centerStruct === "function") {
          K.editor.centerStruct();
          drawLabels();
        }
      };
      fullBtn.after(btn);
    }

    // Context menu on canvas / molecule
    d.addEventListener("contextmenu", async ev => {
      const st = K && K.editor && K.editor.struct();
      if (!st || st.isBlank()) return;
      const hl = labelHit(ev); if (hl) { ev.preventDefault(); ev.stopPropagation(); labelMenu(ev, hl, fr); return; }      // right click on the label under a molecule
      ev.preventDefault();
      ev.stopPropagation();
      const sel = K.editor.selection() || {};
      const set = new Set(sel.atoms || []);
      (sel.bonds || []).forEach(id => { const b = st.bonds.get(id); if (b) { set.add(b.begin); set.add(b.end); } });
      let smi = "";
      if (set.size) { try { smi = await pieceSmiles([...set]); } catch (_) {} }
      if (!smi) { try { smi = await K.getSmiles(); } catch (_) {} }
      const frRect = fr.getBoundingClientRect();
      const mx = frRect.left + ev.clientX, my = frRect.top + ev.clientY;
      const m = Q("#ctx"); if (!m) return;
      m.innerHTML = "";
      const cpItem = document.createElement("div");
      const shortSmi = smi.length > 24 ? smi.slice(0, 21) + "…" : smi;
      cpItem.textContent = smi ? I18N.t("draw.ctx.copySmilesOf", { smiles: shortSmi }) : I18N.t("draw.ctx.copySmiles");
      cpItem.onclick = () => {
        if (smi) navigator.clipboard.writeText(smi).catch(() => {});
        m.hidden = true;
      };
      m.appendChild(cpItem);
      const ctrItem = document.createElement("div");
      ctrItem.textContent = I18N.t("draw.ctx.center");
      ctrItem.onclick = () => {
        if (typeof K.editor.centerStruct === "function") { K.editor.centerStruct(); drawLabels(); }
        m.hidden = true;
      };
      m.appendChild(ctrItem);
      if ((sel.bonds || []).length) {                                   // «Spezza il legame»: the selected bond(s) are deleted (the atoms stay), the two pieces and their formulas are worked out again
        const brk = document.createElement("div"), nb = sel.bonds.length;
        brk.textContent = I18N.t("draw.ctx.breakBonds", { n: nb }); brk.title = I18N.t("draw.ctx.breakBonds.title");
        const bids = [...sel.bonds]; brk.onclick = () => { m.hidden = true; setTimeout(() => breakBonds(bids), 60); };
        m.appendChild(brk);
      }
      for (const it of await fragmentItems(K, ev, fragHelpers).catch(() => [])) {                  // the two pieces of the bond under the pointer, and the table of all cuts
        const row = document.createElement("div"); row.textContent = it.text; if (it.title) row.title = it.title;
        row.onclick = () => { m.hidden = true; if (it.run) it.run(); }; m.appendChild(row);
      }
      m.hidden = false;
      m.style.left = Math.min(mx, innerWidth - 240) + "px";
      m.style.top = Math.max(4, Math.min(my, innerHeight - m.offsetHeight - 8)) + "px";
    }, true);

    const fit = () => {
      const w = d.defaultView.innerWidth, h = d.defaultView.innerHeight, coarse = d.defaultView.matchMedia("(pointer:coarse)").matches, root = d.documentElement;
      root.style.setProperty("--ktz", String(Math.max(1, Math.min(coarse ? 1.3 : 1.05, (w - 24) / 1020)).toFixed(3)));
      // the side columns (rings, tools, atoms) grow as much as the height allows: measured at zoom 1, because the visible buttons depend on the choice of tools
      root.style.setProperty("--ksz", "1");
      const need = Math.max(300, ...['[class*="LeftToolbar-module_buttons"]', '[class*="RightToolbar-module_buttons"]', '[class*="BottomToolbar-module_group"]']
        .map(q => { const e = d.querySelector(q); return e ? e.scrollHeight + 16 + (q.includes("Left") ? (d.getElementById("tp-bar")?.offsetHeight || 0) : 0) : 0; }));
      const top = d.querySelector('[class*="App-module_top"]'), th = top ? top.getBoundingClientRect().height : 46;
      root.style.setProperty("--ksz", String(Math.max(1, Math.min(coarse ? 1.55 : 1.2, (h - th - 24) / need)).toFixed(3)));
    };
    kFit = fit;
    fit(); d.defaultView.addEventListener("resize", fit);
    d.head.appendChild(st);
    kBar = mountBar(d, k => I18N.t(k));
    applyTools();
  } catch (_) { /* not critical */ }
}
// ------------------------------------------------------------------ which tools of Ketcher are shown (choice of the student, kept in this browser)
// Each group lists data-testid of Ketcher's buttons. Default: what is needed to draw structures and transformation pathways;
// the rest (stereochemistry, S/R groups, biology, mapping ...) is one click away in the «Strumenti dell'editor» card.
// Note: Hand, Selection, Eraser, Text are in the top toolbar (always on). Shapes and images are always on.
const TOOLS = [
  ["rings", true, ["bottom-toolbar"]],
  ["react", true, ["reaction-plus", "arrows-drop-down-button"]],
  ["chain", true, ["chain"]],
  ["tidy", true, ["Layout button", "Clean Up button"]],
  ["file", true, ["open-file-button", "save-file-button"]],
  ["arom", false, ["Aromatize button", "Dearomatize button", "Add/Remove explicit hydrogens button"]],
  ["stereo", false, ["enhanced-stereo", "Calculate CIP button"]],
  ["groups", false, ["sgroup", "rgroup-drop-down-button"]],
  ["calc", false, ["Check Structure button", "Calculated Values button", "3D Viewer button"]],
  ["map", false, ["reaction-mapping-tools-drop-down-button"]],
  ["atoms", false, ["any-atom", "extended-table"]],
  ["bio", false, ["create-monomer", "polymer-toggler"]],
];
const TKEY = "qqq.disegno.strumenti"; // kept from the old name: renaming it would lose the users' data
function toolChoice() {
  const saved = lsObj(TKEY);
  return Object.fromEntries(TOOLS.map(([k, on]) => [k, k in saved ? !!saved[k] : on]));
}
function applyTools() {
  const ch = toolChoice();
  const off = TOOLS.filter(([k]) => !ch[k]).flatMap(t => t[2]).map(id => `[data-testid="${id}"]`);
  try {
    const d = Q("#kframe").contentDocument; if (!d || !d.head) return;
    let st = d.getElementById("tp-tools"); if (!st) { st = d.createElement("style"); st.id = "tp-tools"; d.head.appendChild(st); }
    st.textContent = off.length ? off.join(",") + "{display:none!important}" : "";
    // a group of the side toolbars whose buttons are all hidden would stay as an empty white box: it is hidden too
    const w = d.defaultView;
    d.querySelectorAll('[class*="LeftToolbar-module_buttons"] > *, [class*="RightToolbar-module_buttons"] > *').forEach(g => {
      g.style.display = "";
      const bs = g.matches("[data-testid]") ? [g] : [...g.querySelectorAll("[data-testid]")];          // a bare button (periodic table) is its own group
      if (bs.length === 0 || bs.every(b => b.getClientRects().length === 0)) g.style.display = "none";   // no box = hidden by the sheet or by a parent
    });
    if (kBar) kBar.update(ch);
    kFit();
    const kl = Q("#kload"); if (kl) kl.hidden = true;
  } catch (_) { /* editor not loaded yet: applied when it starts */ }
}
function toolCard() {
  const box = Q("#tools-body"); if (!box) return;
  const ch = toolChoice();
  box.innerHTML = TOOLS.map(([k]) => `<div style="margin:2px 0"><label><input type="checkbox" data-t="${k}" ${ch[k] ? "checked" : ""}> ${I18N.t(`draw.tools.${k}`)}</label></div>`).join("") +
    '<div class="bar" style="margin-top:6px"><button type="button" id="tools-all">' + I18N.t("draw.tools.showAll") + '</button><button type="button" id="tools-def">' + I18N.t("draw.tools.essentialOnly") + '</button></div>';
  const save = o => { try { localStorage.setItem(TKEY, JSON.stringify(o)); } catch (_) { /* this tab only */ } applyTools(); toolCard(); };
  box.querySelectorAll("input[data-t]").forEach(i => i.onchange = () => save({ ...toolChoice(), [i.dataset.t]: i.checked }));
  Q("#tools-all").onclick = () => save(Object.fromEntries(TOOLS.map(([k]) => [k, true])));
  Q("#tools-def").onclick = () => { try { localStorage.removeItem(TKEY); } catch (_) { /* nothing saved */ } applyTools(); toolCard(); };
}
toolCard(); mountFont(Q("#font-body"), () => K, k => I18N.t(k));
// Zoom preset: 100%. Re-centres if there is content.
function fitZoom(k, fr) {
  try {
    if (typeof k.editor.zoom !== "function") return;
    k.editor.zoom(1);
    try { k.editor.event.zoomChanged.dispatch(); } catch (_) { /* the «100 %» label of Ketcher is only updated by this event */ }
    const st = k.editor.struct();
    if (st && !st.isBlank() && typeof k.editor.zoomAccordingContent === "function") { k.editor.zoomAccordingContent(st); k.editor.centerStruct(); }
  } catch (_) { /* default zoom */ }
}
async function restore() {
  if (restored || !K) return;
  restored = true;
  if (typeof NB !== "undefined" && NB.ket) { try { await K.setMolecule(NB.ket); } catch (_) { /* old or empty drawing */ } }
  drawLabels();
}
async function changed() {
  if (!K) return;
  let ket = "";
  try { ket = await K.getKet(); } catch (_) { return; }
  drawLabels();                                                          // the labels drawn right at a change can miss the structure Ketcher is still adding (Quick draw, paste): this one runs when it has settled
  try { if (!NB.ket && K.editor.struct().isBlank()) return; } catch (_) { /* no isBlank: save as before */ }       // nothing drawn and nothing saved: no empty notebook entry
  NB.ket = ket; nbSave();
  showInfo();
}
// small warning line under the SMILES field (invalid SMILES, Ketcher not started); empty = hidden
const dnote = msg => { const w = Q("#smi-warn"); w.textContent = msg || ""; w.hidden = !msg; };

// ------------------------------------------------------------------ SMILES of the selection and property estimates
// Pieces = connected parts of the drawing (or of the selected atoms only). Each piece is written as a molfile from Ketcher's structure; Ketcher's own
// engine (Indigo, same as the SMILES of the whole canvas) turns it into SMILES and OpenChemLib reads that for MoleculeProperties.
// Open valences of a cut piece are closed with H (implicit H).
function pieces(only) {
  const st = K.editor.struct(), ids = [...st.atoms.keys()].filter(i => !only || only.has(i)), up = new Map(ids.map(i => [i, i]));
  const root = i => { while (up.get(i) !== i) { up.set(i, up.get(up.get(i))); i = up.get(i); } return i; };
  st.bonds.forEach(b => { if (up.has(b.begin) && up.has(b.end)) { const a = root(b.begin), c = root(b.end); if (a !== c) up.set(a, c); } });
  const groups = new Map();
  ids.forEach(i => { const r = root(i); if (!groups.has(r)) groups.set(r, []); groups.get(r).push(i); });
  const out = [...groups.values()].map(g => ({ ids: g, x: Math.min(...g.map(i => st.atoms.get(i).pp.x)), y: Math.min(...g.map(i => st.atoms.get(i).pp.y)) }));
  return out.sort((a, b) => a.x - b.x || a.y - b.y);
}
function pieceMolfile(ids, noCharge) {
  const st = K.editor.struct(), set = new Set(ids), idx = new Map(ids.map((id, n) => [id, n + 1])), atoms = ids.map(i => st.atoms.get(i));
  if (atoms.some(a => !(a.label in MONO))) return null;                      // R groups, abbreviations...: no structure
  const bonds = []; st.bonds.forEach(b => { if (set.has(b.begin) && set.has(b.end)) bonds.push(b); });
  const f = (v, w) => v.toFixed(4).padStart(w), n3 = v => String(v).padStart(3), chg = [];
  const L = ["", "  qqq", "", `${n3(atoms.length)}${n3(bonds.length)}  0  0  0  0  0  0  0  0999 V2000`];
  atoms.forEach((a, k) => { L.push(`${f(a.pp.x, 10)}${f(-a.pp.y, 10)}${f(0, 10)} ${a.label.padEnd(3)} 0  0  0  0  0  0  0  0  0  0  0  0`); if (a.charge && !(noCharge && noCharge.has(ids[k]))) chg.push([k + 1, a.charge]); });
  bonds.forEach(b => L.push(`${n3(idx.get(b.begin))}${n3(idx.get(b.end))}${n3(b.type >= 1 && b.type <= 4 ? b.type : 1)}  0  0  0  0`));
  for (let i = 0; i < chg.length; i += 8) L.push("M  CHG" + n3(Math.min(8, chg.length - i)) + chg.slice(i, i + 8).map(([a, c]) => " " + n3(a) + " " + n3(c)).join(""));
  L.push("M  END");
  return L.join("\n");
}
async function pieceSmiles(ids, noCharge) {
  const mf = pieceMolfile(ids, noCharge); if (!mf) return null;
  try {
    const r = await Promise.race([K.structService.convert({ struct: mf, output_format: "chemical/x-daylight-smiles" }, {}), new Promise((_, ko) => setTimeout(() => ko(new Error("timeout")), 20000))]);
    return String(r.struct || "").trim().split(/\s+/)[0] || null;
  } catch (_) { return null; }
}
// Neutral form of a charged piece: drop |net charge| charges of the same sign from atoms that can take the H back (COO- -> COOH, NH3+ -> NH2,
// pyridinium NH+ -> pyridine). A charge that cannot be dropped (quaternary ammonium, O+ ...) is permanent: null. Nitro and N-oxide groups (+ and -
// together, net 0) are never touched.
const VMAX = { N: 3, O: 2, S: 2, P: 3 };
function neutralPlan(ids, q) {
  const st = K.editor.struct(), set = new Set(ids), bs = new Map(ids.map(i => [i, 0]));
  st.bonds.forEach(b => { if (set.has(b.begin) && set.has(b.end)) { const o = b.type === 1 ? 1 : b.type === 2 ? 2 : b.type === 3 ? 3 : 1.5; bs.set(b.begin, bs.get(b.begin) + o); bs.set(b.end, bs.get(b.end) + o); } });
  const ok = ids.filter(i => { const a = st.atoms.get(i); return a.charge && Math.sign(a.charge) === Math.sign(q) && Math.abs(a.charge) === 1 && VMAX[a.label] && bs.get(i) <= VMAX[a.label] + 0.01; });
  return ok.length >= Math.abs(q) ? new Set(ok.slice(0, Math.abs(q))) : null;
}
// «Spezza il legame»: the Delete key of Ketcher on a selection made only of bonds removes the bonds and keeps the atoms (the same as the eraser on a bond)
function breakBonds(ids) {
  try {
    const d = Q("#kframe").contentDocument, bonds = ids || (K.editor.selection() || {}).bonds || []; if (!bonds.length) return false;
    K.editor.selection({ bonds: [...bonds] });                       // only the bonds: Delete would also remove the selected atoms
    try { d.defaultView.focus(); } catch (_) { /* focus is only a help */ }
    const ta = d.querySelector("textarea");                              // Ketcher listens on its hidden text area when it has the focus, otherwise on the document
    [...new Set([ta, d.activeElement, d.body].filter(Boolean))].forEach(tg => {
      if (K.editor.selection() == null) return;                           // already done by the first one: Delete would find nothing
      ["keydown", "keyup"].forEach(t => tg.dispatchEvent(new d.defaultView.KeyboardEvent(t, { key: "Delete", code: "Delete", keyCode: 46, which: 46, bubbles: true })));
    });
    setTimeout(() => { try { drawLabels(); showInfo(); } catch (_) { /* labels are redrawn at the next change */ } }, 300);
    return true;
  } catch (_) { return false; }
}
window.breakBonds = breakBonds;
const fragHelpers = { pieces: () => pieces(), pieceMolfile: ids => pieceMolfile(ids), t: (k, p) => I18N.t(k, p), copy: s => { navigator.clipboard.writeText(s).catch(() => {}); if (window.toast) toast(I18N.t("draw.copy.done")); },
  num: v => { const dec = +Q("#lb-dec").value; return dec ? rh(v, dec).toFixed(dec) : String(rh(v, 0)); }, table: o => window.TDATI.table(o) };
// cut bonds of a piece selection: bonds between a selected and an unselected atom
function cutBonds(set) { let n = 0; K.editor.struct().bonds.forEach(b => { if (set.has(b.begin) !== set.has(b.end)) n++; }); return n; }
const copyBtn = (text, label) => `<button class="sm" data-cp="${EH(text)}" title="${EH(I18N.t("draw.copy.title"))}">${label || EH(I18N.t("draw.copy.button"))}</button>`;
let infoRun = 0;
// One run at a time: Ketcher's structure service (Indigo) can hang when several conversions overlap, so a change that arrives while a run is going
// only marks "again" and the run repeats once with the final selection.
let infoBusy = false, infoAgain = false;
async function showInfo() {
  if (infoBusy) { infoAgain = true; return; }
  infoBusy = true;
  try { do { infoAgain = false; await showInfo1(); } while (infoAgain); }
  catch (e) {                                                                  // never leave an empty or stale box: say what went wrong
    console.error("QqQ Draw: properties not computed", e);
    const pc = Q("#prop-card"); if (pc) { pc.hidden = false; Q("#prop-body").innerHTML = '<div class="muted sm">' + EH(I18N.t("draw.props.failed", { message: e && e.message ? e.message : e })) + '</div>'; }
  } finally { infoBusy = false; }
}
async function showInfo1() {
  const sc = Q("#sel-card"), pc = Q("#prop-card"); if (!K || !sc || !pc) return;
  await loadOCL();
  const run = ++infoRun, sel = K.editor.selection() || {}, st = K.editor.struct(), set = new Set(sel.atoms || []);
  (sel.bonds || []).forEach(id => { const b = st.bonds.get(id); if (b) { set.add(b.begin); set.add(b.end); } });
  const all = pieces(set.size ? set : null).map(p => ({ ...p, c: countAtoms(p.ids.map(i => st.atoms.get(i))) }));
  for (const p of all) {                                                       // SMILES (Ketcher) and molecule (OpenChemLib) of each piece
    p.smi = await pieceSmiles(p.ids); p.mol = null;
    if (p.smi) { try { p.mol = OCL.Molecule.fromSmiles(p.smi); } catch (_) { /* no properties */ } }
    p.counter = !!p.c && p.c.q !== 0 && p.ids.length === 1;                     // Cl-, Na+ ... counter-ion: not a molecule of its own
    p.nmol = null;
    if (neutOn && p.c && p.c.q !== 0 && !p.counter) {                          // neutral form (charge excluded)
      const plan = neutralPlan(p.ids, p.c.q), nsmi = plan ? await pieceSmiles(p.ids, plan) : null;
      if (nsmi) { try { p.nmol = OCL.Molecule.fromSmiles(nsmi); } catch (_) { /* permanent */ } }
    }
  }
  if (run !== infoRun) return;                                                 // the selection changed meanwhile: a newer call wins
  // 1) SMILES of the selection (also a piece, not a whole molecule)
  if (!set.size) sc.hidden = true;
  else {
    sc.hidden = false;
    const cuts = cutBonds(set);
    Q("#sel-body").innerHTML = all.map(p => {
      const smi = p.smi || "";                                                  // written by Ketcher (Indigo): the same SMILES as for the whole canvas
      return smi ? `<div style="display:flex;gap:6px;align-items:center;margin:3px 0"><code style="word-break:break-all;flex:1;user-select:all">${EH(smi)}</code>${copyBtn(smi)}</div>` : '<div class="muted sm">' + EH(I18N.t("draw.selection.noFormula")) + '</div>';
    }).join("") + (cuts ? `<div class="muted sm">${EH(I18N.t("draw.selection.cuts", { n: cuts }))}</div>` : "");
  }
  // 2) property estimates: the selected pieces, or every structure of the drawing
  const charged = all.some(p => p.c && p.c.q !== 0);
  const rows = []; let usedNeutral = false, perm = false;
  for (const p of all) {
    if (!p.mol || !p.c) continue;
    if (neutOn && p.counter) continue;                                          // counter-ions are dropped
    const ion = p.c.q !== 0, m = ion ? (neutOn ? p.nmol : null) : p.mol;       // an ion: its neutral form, or nothing when the charge is kept
    if (ion && neutOn && !m) perm = true;
    const pr = m ? new OCL.MoleculeProperties(m) : null, v = (x, d) => m ? x.toFixed(d) : "&ndash;";
    if (m && ion) usedNeutral = true;
    const fo = m ? m.getMolecularFormula().formula : formulaOf(p.c.n).formula;       // a cut piece is closed with H: the formula is that of the SMILES shown
    const sign = ion && !m ? (p.c.q > 0 ? "<sup>+</sup>" : "<sup>&minus;</sup>") : "";
    rows.push(`<tr><td>${fmtF(fo)}${sign}</td><td class="num"><b>${v(m && pr.logP, 2)}</b></td><td class="num">${v(m && pr.logS, 2)}</td><td class="num">${v(m && pr.polarSurfaceArea, 0)}</td><td class="num">${m ? pr.donorCount + "/" + pr.acceptorCount : "&ndash;"}</td></tr>`);
  }
  pc.hidden = !rows.length && !charged;
  if (!pc.hidden) Q("#prop-body").innerHTML = (rows.length ? `<table class="sm"><tr><th>${EH(I18N.t(set.size ? "draw.props.selection" : "draw.props.structure"))}</th><th class="num">logP</th><th class="num">logS</th><th class="num">TPSA</th><th class="num" title="${EH(I18N.t("draw.props.da.title"))}">D/A</th></tr>${rows.join("")}</table>` : "")
    + (charged ? `<label class="sm" style="display:block;margin-top:4px" title="${EH(I18N.t("draw.props.neutral.title"))}"><input type="checkbox" id="prop-neut"${neutOn ? " checked" : ""}> ${EH(I18N.t("draw.props.neutral.label"))}</label>` : "")
    + (usedNeutral ? '<div class="muted sm">' + EH(I18N.t("draw.props.usedNeutral")) + '</div>' : "")
    + (perm ? '<div class="muted sm">' + EH(I18N.t("draw.props.permanent")) + '</div>' : "");
}
let neutOn = true;                                                             // "Escludi la carica": on by default
document.addEventListener("change", e => { if (e.target && e.target.id === "prop-neut") { neutOn = e.target.checked; showInfo(); } });
const fmtF = f => EH(f).replace(/(\d+)/g, "<sub>$1</sub>");
document.addEventListener("click", e => { const b = e.target.closest && e.target.closest("[data-cp]"); if (b) { try { navigator.clipboard.writeText(b.dataset.cp); if (window.toast) toast(I18N.t("draw.copy.smilesToast")); b.textContent = I18N.t("draw.copy.done"); setTimeout(() => { b.textContent = I18N.t("draw.copy.button"); }, 1200); } catch (_) { /* clipboard blocked */ } } });

// ------------------------------------------------------------------ formula and mass written under each structure
// Optional (checkboxes "#lb-f" formula, "#lb-m" mass + "#lb-dec" decimals). Drawn in an overlay group of Ketcher's own SVG, so the label follows zoom and scroll
// but is NOT part of the structure (undo, .ket and SMILES are untouched). Added as text to the exported images.
const MONO = { H: 1.00782503, D: 2.01410178, C: 12, N: 14.00307401, O: 15.99491462, F: 18.99840322, Na: 22.98976928, Mg: 23.9850417, Al: 26.98153853,
  Si: 27.97692653, P: 30.97376163, S: 31.97207100, Cl: 34.96885268, K: 38.96370668, Ca: 39.96259098, Fe: 55.9349375, Cu: 62.9295975, Zn: 63.9291422,
  As: 74.9215965, Se: 79.9165213, Br: 78.9183371, Sn: 119.9021947, I: 126.904473, Hg: 201.970643, B: 11.0093054, Li: 7.01600455 };
const ELECTRON = 0.00054858;
// one entry per connected structure: formula (Hill order), charge, monoisotopic mass, bounding box (Ketcher coordinates, y down)
// counts {C: 8, H: 9, ...} -> Hill formula and monoisotopic mass
function formulaOf(n) {
  const keys = Object.keys(n).filter(k => n[k] > 0).sort();
  const order = n.C > 0 ? ["C", ...(n.H > 0 ? ["H"] : []), ...keys.filter(k => k !== "C" && k !== "H")] : keys;
  return { formula: order.map(k => k + (n[k] > 1 ? n[k] : "")).join(""), mass: order.reduce((m, k) => m + MONO[k] * n[k], 0) };
}
// element counts of some atoms of the drawing (with the H they carry in the drawing); null if an atom has no formula
function countAtoms(atoms) {
  const n = {}; let q = 0;
  for (const a of atoms) {
    if (!(a.label in MONO)) return null;                 // R groups, "any atom", abbreviations: no formula
    n[a.label] = (n[a.label] || 0) + 1;
    const h = a.implicitH || 0; if (h) n.H = (n.H || 0) + h;
    q += a.charge || 0;
  }
  return { n, q };
}
function structures() {
  if (!K) return [];
  const st = K.editor.struct(), ids = [...st.atoms.keys()], up = new Map(ids.map(i => [i, i]));
  const root = i => { while (up.get(i) !== i) { up.set(i, up.get(up.get(i))); i = up.get(i); } return i; };
  st.bonds.forEach(b => { const a = root(b.begin), c = root(b.end); if (a !== c) up.set(a, c); });
  const groups = new Map();
  ids.forEach(i => { const r = root(i); if (!groups.has(r)) groups.set(r, []); groups.get(r).push(st.atoms.get(i)); });
  const out = [];
  groups.forEach(atoms => {
    const c = countAtoms(atoms); if (!c || !atoms.length) return;
    const xs = atoms.map(a => a.pp.x), ys = atoms.map(a => a.pp.y);
    const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
    out.push({ ...formulaOf(c.n), n: c.n, q: c.q, cx: (x0 + x1) / 2, y: y1, x0, x1, y0, y1 });
  });
  return out;
}
// reaction arrows: what changes from the structure before the arrow to the one after it (e.g. "+O", "-CH2")
function arrowDeltas() {
  if (!K || !Q("#lb-a").checked) return [];            // optional, off by default: the students do this calculation themselves
  const comps = structures(), out = [];
  K.editor.struct().rxnArrows.forEach(ar => {
    const [p0, p1] = ar.pos, left = p0.x <= p1.x ? p0 : p1, right = p0.x <= p1.x ? p1 : p0, ym = (p0.y + p1.y) / 2;
    const near = c => ym >= c.y0 - 1.5 && ym <= c.y1 + 1.5;                // roughly on the same line as the arrow
    const before = comps.filter(c => near(c) && c.cx < left.x).sort((a, b) => b.x1 - a.x1)[0];
    const after = comps.filter(c => near(c) && c.cx > right.x).sort((a, b) => a.x0 - b.x0)[0];
    if (!before || !after) return;
    const els = [...new Set([...Object.keys(before.n), ...Object.keys(after.n)])], gain = {}, loss = {};
    for (const e of els) { const d = (after.n[e] || 0) - (before.n[e] || 0); if (d > 0) gain[e] = d; if (d < 0) loss[e] = -d; }
    const conv = f => f === "H3N" ? "NH3" : f;                              // Hill order would write H3N: students know NH3
    const g = conv(formulaOf(gain).formula), l = conv(formulaOf(loss).formula), dm = after.mass - before.mass;
    const parts = [];
    const push = (sign, f) => { parts.push([sign, ""]); for (const m of f.matchAll(/([A-Z][a-z]?)(\d*)/g)) { parts.push([m[1], ""]); if (m[2]) parts.push([m[2], "sub"]); } parts.push([" ", ""]); };
    if (g) push("+", g); if (l) push("\u2212", l);
    if (!g && !l) parts.push([I18N.t("draw.labels.sameFormula") + " ", ""]);
    parts.push([`\u0394m ${dm >= 0 ? "+" : "\u2212"}${Math.round(Math.abs(dm))}`, ""]);   // unit resolution: the integer is enough
    out.push({ x: (p0.x + p1.x) / 2, y: ym, parts });
  });
  return out;
}
// text of a label as LINES of pieces: [[text, "sub" | "sup" | "it" | ""], ...] (formula on the first line, "m/z 241" in italics on the next)
function labelParts(d0) {
  const d = d0, parts = [], f = Q("#lb-f").checked, m = Q("#lb-m").checked, dec = +Q("#lb-dec").value, lines = [parts];
  if (f) {
    for (const x of d.formula.matchAll(/([A-Z][a-z]?)(\d*)/g)) { parts.push([x[1], ""]); if (x[2]) parts.push([x[2], "sub"]); }
    if (d.q) parts.push([(Math.abs(d.q) > 1 ? Math.abs(d.q) : "") + (d.q > 0 ? "+" : "−"), "sup"]);
  }
  if (m) {
    const raw = d.q ? (d.mass - d.q * ELECTRON) / Math.abs(d.q) : d.mass;
    const line = f ? (lines.push([]), lines[1]) : parts;
    const lab = d.q ? "m/z" : dec ? "exact mass" : "nominal mass";
    line.push([lab, "it"], [` ${dec ? rh(raw, dec).toFixed(dec) : rh(raw, 0)}`, ""]);
  }
  return lines;
}
const labelsOn = () => Q("#lb-f").checked || Q("#lb-m").checked || Q("#lb-a").checked;
function drawLabels() {
  if (!K) return;
  const svg = K.editor.render.paper.canvas, doc = svg.ownerDocument, ns = "http://www.w3.org/2000/svg", sc = K.editor.render.options.microModeScale || 40;
  let g = svg.querySelector("#qqq-labels"); // kept from the old name: renaming it would break DOM lookups/styles
  if (!g) { g = doc.createElementNS(ns, "g"); g.id = "qqq-labels"; g.setAttribute("pointer-events", "none"); }
  svg.appendChild(g);                                     // always last: drawn above the structure
  g.textContent = "";
  if (!labelsOn()) return;
  // the Ketcher canvas stays white in every app theme, so the labels (and the exports) always use the dark-on-white colours
  const text = (x, y, lines, color, size) => lines.forEach((parts, i) => g.appendChild(svgLabel(doc, parts, x, y + i * size * 1.3, size, color)));
  for (const d of structures()) text(d.cx * sc, d.y * sc + 30, labelParts(d), "#3b3b3b", 13);
  for (const a of arrowDeltas()) text(a.x * sc, a.y * sc - 12, [a.parts], "#2b5c8a", 12);
}
// the same labels as Ketcher text objects, only in the copy of the drawing that is exported
const SUBC = "\u2080\u2081\u2082\u2083\u2084\u2085\u2086\u2087\u2088\u2089", SUPC = "\u2070\u00b9\u00b2\u00b3\u2074\u2075\u2076\u2077\u2078\u2079";
// pieces -> plain text with Unicode subscripts/superscripts (only for the fallback: Ketcher text objects)
const plain = parts => parts.map(([t, k]) => k === "sub" ? String(t).replace(/\d/g, c => SUBC[+c])
  : k === "sup" ? String(t).replace(/\+/g, "\u207a").replace(/\u2212/g, "\u207b").replace(/\d/g, c => SUPC[+c]) : t).join("");
function ketWithLabels(ket) {
  if (!labelsOn()) return ket;
  const j = JSON.parse(ket);
  const add = (lines, x, y, px) => {                       // one Ketcher text object, one block per line, italic pieces marked
    const blocks = lines.map((parts, n) => {
      const text = plain(parts), ranges = [{ offset: 0, length: text.length, style: `CUSTOM_FONT_SIZE_${px}px` }]; let off = 0;
      for (const [t, k] of parts) { const len = plain([[t, k]]).length; if (k === "it") ranges.push({ offset: off, length: len, style: "ITALIC" }); off += len; }
      return { key: "qqq" + j.root.nodes.length + "_" + n, text, type: "unstyled", depth: 0, inlineStyleRanges: ranges, entityRanges: [], data: {} };
    });
    j.root.nodes.push({ type: "text", data: { content: JSON.stringify({ blocks, entityMap: {} }), position: { x, y, z: 0 } } });
  };
  for (const d of structures()) { const L = labelParts(d), n = Math.max(...L.map(p => plain(p).length)); add(L, d.cx - n * 0.105, -(d.y + 0.6), 16); }
  for (const a of arrowDeltas()) { const t = plain(a.parts); add([a.parts], a.x - t.length * 0.09, -(a.y - 0.75), 14); }
  return JSON.stringify(j);
}
["#lb-f", "#lb-m", "#lb-dec", "#lb-a"].forEach(id => Q(id).addEventListener("change", () => {
  drawLabels(); NB.labA = Q("#lb-a").checked; NB.labF = Q("#lb-f").checked; NB.labM = Q("#lb-m").checked; NB.labDec = +Q("#lb-dec").value; nbSave();
}));
document.addEventListener("nbloaded", () => {      // older notebooks only have NB.labels (both on)
  Q("#lb-f").checked = NB.labF !== undefined ? NB.labF : NB.labels !== false; Q("#lb-m").checked = NB.labM !== undefined ? NB.labM : NB.labels !== false;
  Q("#lb-dec").value = String(Math.min(5, Math.max(0, +NB.labDec || 0)));
  Q("#lb-a").checked = NB.labA === true;
  Q("#ex-name").value = NB.drawName || stamp();
  drawLabels();
});
// the two example drawings (made with this very tab: see tests_e2e/make_examples.py)
Q("#ex-link").onclick = e => {
  e.preventDefault();
  const fig = (src, alt, cap) => `<figure style="margin:0 0 14px"><img src="static/${src}" alt="${alt}" style="max-width:100%;height:auto"><figcaption class="muted sm">${cap}</figcaption></figure>`;
  window.big(I18N.t("draw.examples.title"),
    fig("esempio-trasformazione.png", EH(I18N.t("draw.examples.transform.alt")), I18N.t("draw.examples.transform.caption")) +
    fig("esempio-frammentazione.png", EH(I18N.t("draw.examples.fragment.alt")), I18N.t("draw.examples.fragment.caption")));
};

// Labels of the exported image as real SVG text (subscripts and superscripts shifted with dy, like the old caption did).
// Ketcher turns text objects into glyph outlines and spaces Unicode subscripts badly, so the export is made WITHOUT text objects:
// the position of the structures in the exported SVG is found by matching the bond end points with the atom positions of the
// .ket (the export scale is 40 px per unit), then the labels are added as <text>. If the match fails, the old way is used.
const EXPORT_SCALE = 40;
function exportOffset(svgText, ket) {
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml");
  doc.querySelectorAll("defs").forEach(d => d.remove());
  const atoms = [];
  Object.keys(ket).filter(k => /^mol\d+$/.test(k)).forEach(k => (ket[k].atoms || []).forEach(a => a.location && atoms.push([a.location[0] * EXPORT_SCALE, -a.location[1] * EXPORT_SCALE])));
  const ends = [];
  doc.querySelectorAll("path").forEach(pa => {
    const n = (pa.getAttribute("d") || "").match(/-?\d+(?:\.\d+)?/g);
    const tr = (pa.getAttribute("transform") || "").match(/matrix\(([^)]*)\)/);                 // Ketcher draws in ket units with matrix(40,0,0,40,tx,ty)
    const [a, b, c, d, e, f] = tr ? tr[1].split(/[\s,]+/).map(Number) : [1, 0, 0, 1, 0, 0];
    if (n && (pa.getAttribute("d") || "").trim()[0] === "M") for (let i = 0; i + 1 < n.length; i += 2) ends.push([a * +n[i] + c * +n[i + 1] + e, b * +n[i] + d * +n[i + 1] + f]);
  });
  const votes = new Map();
  for (const [ax, ay] of atoms) for (const [ex, ey] of ends) {
    const k = Math.round(ex - ax) + "," + Math.round(ey - ay); votes.set(k, (votes.get(k) || 0) + 1);
  }
  // symmetric molecules (rings) match well at several shifts: keep only shifts that leave every atom inside the picture
  // (Ketcher crops the picture tightly around the drawing), then take the best supported one
  const vb = (doc.documentElement.getAttribute("viewBox") || "0 0 0 0").split(/[\s,]+/).map(Number);
  const inside = (ox, oy) => atoms.every(([ax, ay]) => ax + ox >= vb[0] - 3 && ax + ox <= vb[0] + vb[2] + 3 && ay + oy >= vb[1] - 3 && ay + oy <= vb[1] + vb[3] + 3);
  const ranked = [...votes].sort((p, q) => q[1] - p[1]).slice(0, 40);
  const need = Math.max(2, Math.min(atoms.length, 3));
  const pick = ranked.find(([k, c]) => c >= need && inside(...k.split(",").map(Number)));
  if (!pick) return null;
  const [bx, by] = pick[0].split(",").map(Number);
  // refine with the matching pairs
  let sx = 0, sy = 0, m = 0;
  for (const [ax, ay] of atoms) for (const [ex, ey] of ends) if (Math.abs(ex - ax - bx) <= 1.5 && Math.abs(ey - ay - by) <= 1.5) { sx += ex - ax; sy += ey - ay; m++; }
  return { ox: sx / m, oy: sy / m };
}
const svgNS = "http://www.w3.org/2000/svg";
let _meas = null;
const textWidth = (parts, size) => {
  _meas = _meas || document.createElement("canvas").getContext("2d");
  return parts.reduce((w, [s, k]) => { _meas.font = `${k === "it" ? "italic " : ""}${k && k !== "it" ? size * 0.72 : size}px Arial, Helvetica, sans-serif`; return w + _meas.measureText(String(s)).width; }, 0);
};
function svgLabel(doc, parts, x, y, size, color) {
  const t = doc.createElementNS(svgNS, "text");
  t.setAttribute("x", x); t.setAttribute("y", y); t.setAttribute("text-anchor", "middle");
  t.setAttribute("font-family", "Arial, Helvetica, sans-serif"); t.setAttribute("font-size", size); t.setAttribute("fill", color);
  let shift = 0;                                   // current vertical shift in em: tspans carry dy relative to the previous one
  for (const [txt, k] of parts) {
    const want = k === "sub" ? 0.28 : k === "sup" ? -0.45 : 0, sp = doc.createElementNS(svgNS, "tspan");
    sp.textContent = String(txt);
    if (want !== shift) { sp.setAttribute("dy", ((want - shift) * size).toFixed(2)); shift = want; }
    if (k === "it") sp.setAttribute("font-style", "italic"); else if (k) sp.setAttribute("font-size", (size * 0.72).toFixed(2));
    t.appendChild(sp);
  }
  return t;
}
function addExportLabels(svgText, ket) {
  const off = exportOffset(svgText, ket); if (!off) return null;
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml"), root = doc.documentElement;
  const vb = (root.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  if (vb.length !== 4 || vb.some(isNaN)) return null;
  const g = doc.createElementNS(svgNS, "g"); g.setAttribute("id", "qqq-labels"); // kept from the old name: renaming it would break DOM lookups/styles
  let [x0, y0, x1, y1] = [vb[0], vb[1], vb[0] + vb[2], vb[1] + vb[3]];
  const put = (lines, x, y, size, color) => lines.forEach((parts, i) => {
    const yy = y + i * size * 1.3;
    g.appendChild(svgLabel(doc, parts, x, yy, size, color));
    const w = textWidth(parts, size);
    x0 = Math.min(x0, x - w / 2 - 8); x1 = Math.max(x1, x + w / 2 + 8); y0 = Math.min(y0, yy - size - 4); y1 = Math.max(y1, yy + size * 0.6 + 4);
  });
  for (const d of structures()) put(labelParts(d), off.ox + d.cx * EXPORT_SCALE, off.oy + (d.y + 0.6) * EXPORT_SCALE + 14, 16, "#3b3b3b");
  for (const a of arrowDeltas()) put([a.parts], off.ox + a.x * EXPORT_SCALE, off.oy + (a.y - 0.75) * EXPORT_SCALE + 12, 14, "#2b5c8a");
  // grow the picture to hold the labels (background rectangle included)
  const bg = [...doc.querySelectorAll("rect")].find(r => !r.closest("defs"));
  root.setAttribute("viewBox", `${x0} ${y0} ${x1 - x0} ${y1 - y0}`);
  const sx = (x1 - x0) / vb[2], sy = (y1 - y0) / vb[3];
  const pw = parseFloat(root.getAttribute("width")), ph = parseFloat(root.getAttribute("height"));
  if (pw) root.setAttribute("width", pw * sx); if (ph) root.setAttribute("height", ph * sy);
  if (bg) { bg.setAttribute("x", x0); bg.setAttribute("y", y0); bg.setAttribute("width", x1 - x0); bg.setAttribute("height", y1 - y0); }
  root.appendChild(g);
  return new XMLSerializer().serializeToString(doc);
}

// ------------------------------------------------------------------ export
// Ketcher crops the picture tightly around the structures and can cut a text object written near the edge: add a margin on every side
function padSvg(svgText, px) {
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml"), root = doc.documentElement;
  const vb = (root.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  if (vb.length !== 4 || vb.some(isNaN)) return svgText;
  const [x, y, w, h] = vb, bg = [...doc.querySelectorAll("rect")].find(r => !r.closest("defs"));
  root.setAttribute("viewBox", `${x - px} ${y - px} ${w + 2 * px} ${h + 2 * px}`);
  const pw = parseFloat(root.getAttribute("width")), ph = parseFloat(root.getAttribute("height"));
  if (pw) root.setAttribute("width", pw * (w + 2 * px) / w); if (ph) root.setAttribute("height", ph * (h + 2 * px) / h);
  if (bg) { bg.setAttribute("x", x - px); bg.setAttribute("y", y - px); bg.setAttribute("width", w + 2 * px); bg.setAttribute("height", h + 2 * px); }
  return new XMLSerializer().serializeToString(doc);
}
// transparent export: drop the white background rectangle that Ketcher draws behind the structures
function noBackground(svgText) {
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml");
  doc.querySelectorAll("rect").forEach(q => { if (!q.closest("defs") && /^(rgb\(100%,\s*100%,\s*100%\)|#fff(fff)?|white)$/i.test(q.getAttribute("fill") || "")) q.remove(); });
  return new XMLSerializer().serializeToString(doc);
}
async function image(format) {
  await start();
  const raw = await K.getKet(), gen = k => K.generateImage(k, { outputFormat: "svg", backgroundColor: "255,255,255" });
  let svgText = null;
  if (labelsOn()) { try { svgText = addExportLabels(await (await gen(raw)).text(), JSON.parse(raw)); } catch (_) { svgText = null; } }   // real SVG text
  if (!svgText) svgText = await (await gen(ketWithLabels(raw))).text();                                                              // fallback: Ketcher text objects
  svgText = padSvg(svgText, 16);
  const clear = Q("#ex-nobg").checked && format !== "jpg";       // transparent background: PNG and SVG only (JPEG has no transparency)
  if (clear) svgText = noBackground(svgText);
  svgText = svgText.replace(/<svg\b([^>]*)>/, (m, at) => `<svg${at}><title>${EH(I18N.t(clear ? "draw.export.svgTitle.clear" : "draw.export.svgTitle.white", { app: APP_NAME, stamp: stamp().slice(-15, -5) }))}</title>`);
  const svg = new Blob([svgText], { type: "image/svg+xml" });
  if (format === "svg") return svg;
  // PNG / JPEG: rasterise the vector at high resolution (Ketcher's own PNG is small): at least 3x, about 3600 px wide, never more than 12000 px on a side
  const img = new Image(), url = URL.createObjectURL(svg);
  await new Promise((ok, ko) => { img.onload = ok; img.onerror = ko; img.src = url; });
  const sc = Math.min(8, Math.max(3, 3600 / Math.max(img.width, 1)), 12000 / Math.max(img.width, img.height, 1));
  const c = document.createElement("canvas"); c.width = Math.round(img.width * sc); c.height = Math.round(img.height * sc);
  const g = c.getContext("2d"); if (!clear) { g.fillStyle = "#fff"; g.fillRect(0, 0, c.width, c.height); }
  g.drawImage(img, 0, 0, c.width, c.height);
  URL.revokeObjectURL(url);
  const out = await new Promise(r => c.toBlob(r, format === "jpg" ? "image/jpeg" : "image/png", 0.95));
  if (format === "png" && window.pngWithMeta) {                    // tEXt chunks, like the plots of the Dati tab (no personal data)
    try { return await window.pngWithMeta(out, [["Title", I18N.t("draw.export.metaTitle")], ["Description", I18N.t(clear ? "draw.export.metaDesc.clear" : "draw.export.metaDesc.white", { scale: sc.toFixed(1) })], ["Software", APP_NAME], ["Creation Time", new Date().toISOString()]]); } catch (_) { /* saved without metadata */ }
  }
  return out;
}
// ---- export file names: disegno_AAAA-MM-GG_HHMM.<ext>; one base name for PNG, JPEG, SVG and .ket of the same minute, "_trasparente" for a transparent
// PNG/SVG, "-2", "-3" for a repeated name; ASCII lower case only (works on Mac, Windows and the web). The student can change the base name.
const AUTO = /^(disegno|drawing)_\d{4}-\d\d-\d\d_\d{4}$/, usedNames = new Map();
const stamp = () => { const d = new Date(), z = n => String(n).padStart(2, "0"); return `${I18N.t("draw.export.filePrefix")}_${d.getFullYear()}-${z(d.getMonth() + 1)}-${z(d.getDate())}_${z(d.getHours())}${z(d.getMinutes())}`; };
const cleanName = t => String(t || "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9_-]+/g, "_").replace(/_{2,}/g, "_").replace(/^[_-]+|[_-]+$/g, "").slice(0, 60);
function freshName() { const f = Q("#ex-name"); if (!f.value.trim() || AUTO.test(f.value.trim())) f.value = stamp(); }
function exportName(ext, clear) {
  freshName();
  const base = (cleanName(Q("#ex-name").value) || stamp()) + (clear ? "_" + I18N.t("draw.export.transparentSuffix") : ""), key = base + "." + ext, n = (usedNames.get(key) || 0) + 1;
  usedNames.set(key, n);
  return base + (n > 1 ? "-" + n : "") + "." + ext;
}
Q("#ex-name").addEventListener("change", () => { const c = cleanName(Q("#ex-name").value); Q("#ex-name").value = c || stamp(); NB.drawName = AUTO.test(Q("#ex-name").value) ? "" : Q("#ex-name").value; nbSave(); });
const exwarn = msg => { const w = Q("#ex-warn"); w.textContent = msg || ""; w.hidden = !msg; };
// every export goes through here: a failure is written in the page instead of being silent
async function doExport(kind) {
  exwarn("");
  try {
    await start();
    if (kind === "ket") { saveBlob(new Blob([await K.getKet()], { type: "application/json" }), exportName("ket")); if (window.toast) toast(I18N.t("draw.export.saved")); return; }
    if (kind === "jpg" && Q("#ex-nobg").checked) return;
    const clear = Q("#ex-nobg").checked && kind !== "jpg", blob = await image(kind);
    if (!blob || !blob.size) throw new Error(I18N.t("draw.export.emptyImage"));
    saveBlob(blob, exportName(kind, clear));
    if (window.toast) toast(I18N.t("draw.export.saved"));
  } catch (e) { exwarn(I18N.t("draw.export.failed", { message: e && e.message ? e.message : e })); }
}
Q("#ex-png").onclick = () => doExport("png");
Q("#ex-jpg").onclick = () => doExport("jpg");
Q("#ex-svg").onclick = () => doExport("svg");
Q("#ex-ket").onclick = () => doExport("ket");
function syncBg() {                                    // JPEG cannot be transparent: its button is off (grey) while "Sfondo trasparente" is on
  const on = Q("#ex-nobg").checked, j = Q("#ex-jpg");
  j.disabled = on; j.title = on ? I18N.t("draw.export.jpegNoAlpha") : "";
}
Q("#ex-nobg").addEventListener("change", syncBg); syncBg();
freshName();
Q("#ex-load").onclick = async () => { const v = Q("#ex-smi").value.trim(); if (!v) return; await start(); dnote("");
  try { const before = await K.getKet(); await K.addFragment(v); if (await K.getKet() === before) dnote(I18N.t("draw.quick.invalidNone")); else { Q("#ex-smi").value = ""; fitZoom(K, Q("#kframe")); /* Ketcher brings the zoom back to 100 % after a paste */ drawLabels(); } }   // ADDS next to what is drawn (never setMolecule: it would erase the student's work); Ketcher ignores some invalid SMILES without an error
  catch (e) { dnote(I18N.t("draw.quick.invalid", { message: e.message })); } };

window.TPDraw = { image, info: showInfo, smiles: async () => { await start(); return K.getSmiles(); }, ready: () => !!K };
document.addEventListener("tpview", e => { if (e.detail.view === "draw") start(); });
try { if (S.view === "draw") start(); } catch (_) { /* S not there yet: the first tpview starts it */ }   // a direct address (#/disegno, Cmd+Shift+R): the editor starts without waiting for the engine
document.addEventListener("nbloaded", () => { if (K) { restored = false; restore(); } });
// "Nuova sessione": the drawing and its controls go back to the start (the notebook was already emptied)
document.addEventListener("nbreset", () => {
  clearTimeout(timer);
  Q("#lb-f").checked = true; Q("#lb-m").checked = true; Q("#lb-a").checked = false; Q("#lb-dec").value = "0"; Q("#ex-nobg").checked = false; syncBg();
  Q("#ex-name").value = stamp(); Q("#ex-smi").value = ""; dnote(""); exwarn(""); usedNames.clear();
  if (K) { try { K.editor.clear(); } catch (_) { /* nothing to clear */ } restored = true; FRAME.next(() => { drawLabels(); showInfo(); }); }
});

// ------------------------------------------------------------------ label under a molecule: right click = copy the formula or the mass
// geometry of the labels (same numbers as drawLabels): a box per structure, in the user units of Ketcher's svg
function labelBoxes() {
  if (!K || !(Q("#lb-f").checked || Q("#lb-m").checked)) return [];
  const sc = K.editor.render.options.microModeScale || 40, size = 13;
  return structures().map(d => {
    const lines = labelParts(d), w = Math.max(...lines.map(l => textWidth(l, size))), x = d.cx * sc, y = d.y * sc + 30;
    return { d, x0: x - w / 2 - 6, x1: x + w / 2 + 6, y0: y - size - 4, y1: y + (lines.length - 1) * size * 1.3 + 8 };
  });
}
function labelHit(ev) {
  try {
    const svg = K.editor.render.paper.canvas, m = svg.getScreenCTM(); if (!m) return null;
    const pt = svg.createSVGPoint(); pt.x = ev.clientX; pt.y = ev.clientY; const u = pt.matrixTransform(m.inverse());
    const b = labelBoxes().find(b => u.x >= b.x0 && u.x <= b.x1 && u.y >= b.y0 && u.y <= b.y1);
    return b ? b.d : null;
  } catch (_) { return null; }
}
// formula as the student can paste it in the Data view: Hill formula, then the charge (C10H14N+, C6H5O2-)
const formulaText = d => d.formula + (d.q ? (Math.abs(d.q) > 1 ? Math.abs(d.q) : "") + (d.q > 0 ? "+" : "-") : "");
const massText = d => { const raw = d.q ? (d.mass - d.q * ELECTRON) / Math.abs(d.q) : d.mass, dec = +Q("#lb-dec").value; return dec ? rh(raw, dec).toFixed(dec) : String(rh(raw, 0)); };
function labelMenu(ev, d, fr) {
  const m = Q("#ctx"); if (!m) return;
  const fr0 = fr.getBoundingClientRect(), mx = fr0.left + ev.clientX, my = fr0.top + ev.clientY;
  m.innerHTML = "";
  const item = (label, text) => { const e = document.createElement("div"); e.textContent = label; e.onclick = () => { navigator.clipboard.writeText(text).catch(() => {}); m.hidden = true; if (window.toast) toast(I18N.t("draw.copy.done")); }; m.appendChild(e); };
  item(I18N.t("draw.ctx.copyFormula", { formula: formulaText(d) }), formulaText(d));
  item(I18N.t(d.q ? "draw.ctx.copyMassMz" : "draw.ctx.copyMass", { mass: massText(d) }), massText(d));
  m.hidden = false;
  m.style.left = Math.min(mx, innerWidth - 270) + "px"; m.style.top = Math.max(4, Math.min(my, innerHeight - m.offsetHeight - 8)) + "px";
}
// ------------------------------------------------------------------ side cards: hide / show (more room to draw), remembered in this browser
const SKEY = "qqq.disegno.riquadri";
function sideApply(hidden) {
  Q("#v-draw").classList.toggle("noside", hidden);
  const b = Q("#side-toggle"); Q("#side-toggle .tl").textContent = I18N.t(hidden ? "draw.side.show" : "draw.side.hide");
  Q(hidden ? "#side-corner" : "#side-row").appendChild(b);   // the button sits above the cards, or in the canvas corner when they are hidden
  try { localStorage.setItem(SKEY, hidden ? "0" : "1"); } catch (_) { /* storage not available */ }
  setTimeout(() => { if (kFit) kFit(); if (K) { try { fitZoom(K, Q("#kframe")); } catch (_) { /* not critical */ } } }, 60);
}
Q("#side-toggle").onclick = () => sideApply(!Q("#v-draw").classList.contains("noside"));
try { if (localStorage.getItem(SKEY) === "0") { Q("#v-draw").classList.add("noside"); Q("#side-toggle .tl").textContent = I18N.t("draw.side.show"); Q("#side-corner").appendChild(Q("#side-toggle")); } } catch (_) { /* default: visible */ }
// ------------------------------------------------------------------ trackpad: a two-finger swipe sideways moves the drawing; it must not go back in the page history
function frameExtras(fr) {
  const d = fr.contentDocument; if (!d) return;
  d.documentElement.style.overscrollBehaviorX = "none"; d.body.style.overscrollBehaviorX = "none";
  d.addEventListener("wheel", e => { if (!e.ctrlKey && Math.abs(e.deltaX) > Math.abs(e.deltaY)) e.preventDefault(); }, { passive: false });
}
// ------------------------------------------------------------------ shortcuts card: depends on the device (macOS / Windows and Linux / tablet)
(() => {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent), coarse = matchMedia("(pointer:coarse)").matches, mod = mac ? "\u2318" : "Ctrl";
  const t = I18N.t;
  const rows = [[`${mod} C / V / X`, t("draw.keys.copyPasteCut")], [`${mod} Z`, t("draw.keys.undo")], [mac ? "\u21e7 \u2318 Z" : "Ctrl Y", t("draw.keys.redo")], [`${mod} A`, t("draw.keys.selectAll")],
    [mac ? "\u232b" : t("draw.keys.delete.key"), t("draw.keys.delete")], [t("draw.keys.shiftClick.key"), t("draw.keys.shiftClick")], [t("draw.keys.drag.key"), t("draw.keys.drag")], ["Esc", t("draw.keys.escape")],
    [t("draw.keys.rightClick.key"), t("draw.keys.rightClick")]];
  if (coarse) rows.push([t("draw.keys.twoFingers.key"), t("draw.keys.twoFingers")], [t("draw.keys.pen.key"), t("draw.keys.pen")], [t("draw.keys.longPress.key"), t("draw.keys.longPress")]);
  else rows.push([t("draw.keys.trackpad.key"), t("draw.keys.trackpad")]);
  Q("#keys-body").innerHTML = `<table>${rows.map(r => `<tr><td>${EH(r[0])}</td><td>${EH(r[1])}</td></tr>`).join("")}</table>`;
})();
