// @ts-check
"use strict";
// «Choose samples»: when a SCIEX .wiff holds several samples the page lists them (engine listing `samples_list/1`, container and index only, the .wiff.scan is not read) and
// the user ticks which to open (docs/agenti/wiff-sample-choice.md, PR 4). No sample logic here: the list, the first ticks (`sample_default`) and the «1,3,5» field
// (`sample_selection`) are the engine's; this file draws a dialog and reports the ticked ids. Lazy (carica("wiff-choose.js")), called by wiffToMzml (shell.js).

const wcT = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
const wcMb = (/** @type {number} */ n) => n >= 1 << 20 ? wcT("pick.mb", { n: (n / 1048576).toFixed(n >= 10 << 20 ? 0 : 1) }) : wcT("pick.kb", { n: Math.max(n > 0 ? 1 : 0, Math.round(n / 1024)) });
const wcEl = (/** @type {string} */ tag, /** @type {Record<string, any>} */ a = {}, /** @type {(Node | string)[]} */ kids = []) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(a)) { if (k === "text") e.textContent = v; else if (k in e && k !== "list") /** @type {any} */ (e)[k] = v; else e.setAttribute(k, v); }
  for (const c of kids) e.append(c);
  return e;
};
// literal keys (tools/controlla_i18n.py finds a key by its text)
const WC_KIND = /** @type {Record<string, string>} */ ({ ms: "pick.kind.ms", ms2: "pick.kind.ms2", sim: "pick.kind.sim", mrm: "pick.kind.mrm", dad: "pick.kind.dad" });
const WC_WHY = /** @type {Record<string, string>} */ ({ empty: "pick.why.empty", inferred: "pick.why.inferred", refused: "pick.why.refused" });
/** @typedef {{ id: number, name: string, scans: number, size?: number, kinds?: string[], why?: string }} Smp */
/** @typedef {{ key: string, name: string, samples: Smp[], ticked: Set<number> }} Entry */

/** the dialog: resolves with `Map(key -> ids)` or null (cancelled) */
function wcDialog(/** @type {Entry[]} */ entries, /** @type {any} */ rb) {
  return new Promise(done => {
    const prior = /** @type {HTMLElement | null} */ (document.activeElement);
    const dlg = /** @type {HTMLDialogElement} */ (wcEl("dialog", { className: "wiffchoose", id: "wiffchoose" }));
    dlg.setAttribute("aria-labelledby", "wc-title");
    const total = entries.reduce((a, e) => a + e.samples.length, 0);
    const boxes = /** @type {{ cb: HTMLInputElement, e: Entry, s: Smp }[]} */ ([]);
    const status = wcEl("p", { className: "wc-sel", role: "status" });
    status.setAttribute("aria-live", "polite");
    const ok = /** @type {HTMLButtonElement} */ (wcEl("button", { type: "button", className: "go", text: wcT("pick.open"), title: wcT("pick.open.tip") }));
    const cancel = wcEl("button", { type: "button", text: wcT("pick.cancel"), title: wcT("pick.cancel.tip") });
    const msg = wcEl("p", { className: "wc-msg", role: "alert" });
    const upd = () => {
      const on = boxes.filter(b => b.cb.checked);
      status.textContent = wcT("pick.sel", { n: on.length, total, size: wcMb(on.reduce((a, b) => a + (b.s.size || 0), 0)) });
      ok.disabled = !on.length;
    };
    const setAll = (/** @type {boolean} */ v) => { boxes.forEach(b => { b.cb.checked = v; }); upd(); };
    const bar = wcEl("div", { className: "wc-bar" });
    const all = wcEl("button", { type: "button", text: wcT("pick.all"), title: wcT("pick.all.tip") }), none = wcEl("button", { type: "button", text: wcT("pick.none"), title: wcT("pick.none.tip") });
    all.onclick = () => setAll(true); none.onclick = () => setAll(false);
    const ids = /** @type {HTMLInputElement} */ (wcEl("input", { type: "text", id: "wc-ids", size: 12, placeholder: "1,3,5", title: wcT("pick.ids.tip") }));
    ids.setAttribute("aria-label", wcT("pick.ids"));
    const go = wcEl("button", { type: "button", text: wcT("pick.ids.go"), title: wcT("pick.ids.tip") });
    // the engine reads the text against the ids of each file; a file that lacks one of them is named, nothing is ticked half way
    go.onclick = async () => {
      msg.textContent = "";
      try {
        const per = [];
        for (const e of entries) per.push((await rb.nativeAsk({ op: "natselect", present: e.samples.map(s => s.id), text: ids.value })).samples);
        boxes.forEach(b => { b.cb.checked = per[entries.indexOf(b.e)].includes(b.s.id); }); upd();
      } catch (x_) {
        const x = /** @type {any} */ (x_), j = x && x.json || {}, p = j.params || {};
        msg.textContent = j.error === "err.sample.unknown" ? wcT("pick.ids.unknown", { missing: p.missing, ids: p.ids }) : wcT("pick.ids.bad", { text: ids.value });
      }
    };
    ids.onkeydown = ev => { if (ev.key === "Enter") { ev.preventDefault(); go.click(); } };
    bar.append(all, none, wcEl("label", { htmlFor: "wc-ids", text: wcT("pick.ids") }), ids, go);
    const body = wcEl("div", { className: "wc-body" });
    for (const e of entries) {
      const size = e.samples.reduce((a, s) => a + (s.size || 0), 0);
      const tb = wcEl("table", { className: "wc-tab" }), cap = wcEl("caption", { text: wcT("pick.file", { name: e.name, n: e.samples.length, size: wcMb(size) }) });
      tb.setAttribute("aria-label", wcT("pick.table", { name: e.name }));
      const th = (/** @type {string} */ k, /** @type {string} */ cls = "") => { const h = wcEl("th", { text: wcT(k), className: cls }); h.setAttribute("scope", "col"); return h; };
      tb.append(cap, wcEl("thead", {}, [wcEl("tr", {}, [th("pick.col.open"), th("pick.col.name"), th("pick.col.id", "n"), th("pick.col.kinds"), th("pick.col.scans", "n"), th("pick.col.size", "n")])]));
      const tbody = wcEl("tbody");
      for (const s of e.samples) {
        const cb = /** @type {HTMLInputElement} */ (wcEl("input", { type: "checkbox", checked: e.ticked.has(s.id) }));
        cb.setAttribute("aria-label", s.name);
        cb.onchange = upd;
        boxes.push({ cb, e, s });
        const note = s.why ? wcT("pick.note", { why: wcT(WC_WHY[s.why] || "pick.why.refused") }) : "", kinds = (s.kinds || []).map(k => wcT(WC_KIND[k] || k)).join(", ");
        tbody.append(wcEl("tr", {}, [wcEl("td", {}, [cb]), wcEl("td", { text: s.name + note }), wcEl("td", { className: "n", text: String(s.id) }), wcEl("td", { text: kinds }), wcEl("td", { className: "n", text: s.scans ? s.scans.toLocaleString("en") : wcT("pick.empty") }), wcEl("td", { className: "n", text: s.size ? wcMb(s.size) : "" })]));
      }
      tb.append(tbody); body.append(wcEl("div", { className: "wc-scroll" }, [tb]));
    }
    const foot = wcEl("div", { className: "wc-act" }, [ok, cancel]);
    dlg.append(wcEl("h2", { id: "wc-title", text: wcT("pick.title") }), wcEl("p", { className: "wc-intro", text: wcT("pick.intro", { files: entries.length === 1 ? entries[0].name : wcT("pick.files", { n: entries.length }), samples: total }) }), bar, msg, body, status, foot);
    let result = /** @type {Map<string, number[]> | null} */ (null);
    const finish = () => { dlg.close(); dlg.remove(); if (prior && prior.isConnected) prior.focus(); done(result); };
    ok.onclick = () => { result = new Map(entries.map(e => [e.key, boxes.filter(b => b.e === e && b.cb.checked).map(b => b.s.id)])); finish(); };
    cancel.onclick = finish;
    dlg.addEventListener("cancel", ev => { ev.preventDefault(); finish(); });     // Esc
    document.body.appendChild(dlg); upd(); dlg.showModal();
    ok.focus();
  });
}

/**
 * Asks which samples to open. `pairs` = [{ key, name, mem }] (`mem`: the members of the dataset as `rb.nativeSamples` takes them), `mode` = "several" | "always" | "never".
 * Resolves with `null` when nothing needs asking (the setting, one sample per file, a listing that failed: opening goes on as it always did), `{ cancelled: true }`,
 * or `{ picks: Map(key -> ids), total: Map(key -> samples in the file) }` for the files that were asked.
 */
async function wcAsk(/** @type {{ key: string, name: string, mem: { name: string, blob: Blob }[] }[]} */ pairs, /** @type {any} */ rb, /** @type {string} */ mode) {
  if (mode === "never" || !rb || !rb.nativeSamples || !rb.nativeAsk) return null;
  /** @type {Entry[]} */ const entries = [];
  for (const p of pairs) {
    try {
      const j = await rb.nativeSamples(p.mem, true), samples = /** @type {Smp[]} */ (j.samples || []);
      if (!samples.length) continue;
      const d = await rb.nativeAsk({ op: "natdefault", ids: samples.map(s => s.id), sizes: samples.map(s => s.size || 0) });
      entries.push({ key: p.key, name: p.name, samples, ticked: new Set(d.samples) });
    } catch (_) { /* the normal opening reports a file that cannot be read */ }
  }
  if (!entries.length || mode !== "always" && entries.every(e => e.samples.length < 2)) return null;
  const picks = await wcDialog(entries, rb);
  return picks ? { picks, total: new Map(entries.map(e => [e.key, e.samples.length])) } : { cancelled: true };
}
const WIFFCHOOSE = { ask: wcAsk, dialog: wcDialog };
