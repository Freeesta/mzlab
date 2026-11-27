// @ts-check
"use strict";
// «Choose samples»: when a SCIEX .wiff holds several samples the page lists them (engine listing `samples_list/1`, container and index only, the .wiff.scan is not read) and
// the user ticks which to open (docs/agenti/wiff-sample-choice.md). No sample logic here: the list (kinds, scan counts, `why`) and the first ticks (`sample_default`,
// empty samples left out) are the engine's; this file draws a dialog and reports the ticked ids. Lazy (loadScript("wiff-choose.js")), called by wiffToMzml (shell.js).

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
const WC_WHY = /** @type {Record<string, string>} */ ({ inferred: "pick.why.inferred", refused: "pick.why.refused" });
/** @typedef {{ id: number, name: string, scans: number, size?: number, kinds?: string[], why?: string }} Smp */
/** @typedef {{ key: string, name: string, samples: Smp[], ticked: Set<number> }} Entry */

/** the dialog: resolves with `Map(key -> ids)` or null (closed) */
function wcDialog(/** @type {Entry[]} */ entries) {
  return new Promise(done => {
    const prior = /** @type {HTMLElement | null} */ (document.activeElement);
    const dlg = /** @type {HTMLDialogElement} */ (wcEl("dialog", { className: "wiffchoose", id: "wiffchoose" }));
    dlg.setAttribute("aria-labelledby", "wc-title");
    const total = entries.reduce((a, e) => a + e.samples.length, 0);
    const boxes = /** @type {{ cb: HTMLInputElement, e: Entry, s: Smp }[]} */ ([]);
    const status = wcEl("p", { className: "wc-sel", role: "status" });
    status.setAttribute("aria-live", "polite");
    const ok = /** @type {HTMLButtonElement} */ (wcEl("button", { type: "button", className: "go", text: wcT("pick.open") }));
    const upd = () => {
      const on = boxes.filter(b => b.cb.checked).length;
      status.textContent = wcT("pick.sel", { n: on, total });
      ok.disabled = !on;
    };
    const setAll = (/** @type {boolean} */ v) => { boxes.forEach(b => { b.cb.checked = v; }); upd(); };
    const all = wcEl("button", { type: "button", text: wcT("pick.all") }), none = wcEl("button", { type: "button", text: wcT("pick.none") });
    all.onclick = () => setAll(true); none.onclick = () => setAll(false);
    const body = wcEl("div", { className: "wc-body" });
    for (const e of entries) {
      const tb = wcEl("table", { className: "wc-tab" });
      tb.setAttribute("aria-label", wcT("pick.table", { name: e.name }));
      if (entries.length > 1) tb.append(wcEl("caption", { text: e.name }));
      const th = (/** @type {string} */ k, /** @type {string} */ cls = "") => { const h = wcEl("th", { text: wcT(k), className: cls }); h.setAttribute("scope", "col"); return h; };
      tb.append(wcEl("thead", {}, [wcEl("tr", {}, [th("pick.col.open"), th("pick.col.name"), th("pick.col.kinds"), th("pick.col.scans", "n"), th("pick.col.size", "n")])]));
      const tbody = wcEl("tbody");
      for (const s of e.samples) {
        const cb = /** @type {HTMLInputElement} */ (wcEl("input", { type: "checkbox", checked: e.ticked.has(s.id), id: `wc-${boxes.length}` }));
        cb.setAttribute("aria-label", s.name);
        cb.onchange = upd;
        boxes.push({ cb, e, s });
        // `why` is the engine's: an `empty` sample acquired nothing (drawn dim, «empty» in place of a count); a refused or rebuilt one keeps its reason next to the name
        const empty = s.why === "empty", note = s.why && !empty ? wcT("pick.note", { why: wcT(WC_WHY[s.why] || "pick.why.refused") }) : "", kinds = (s.kinds || []).map(k => wcT(WC_KIND[k] || k)).join(", ");
        tbody.append(wcEl("tr", { className: empty ? "wc-empty" : "" }, [wcEl("td", {}, [cb]), wcEl("td", {}, [wcEl("label", { htmlFor: cb.id, text: s.name + note })]), wcEl("td", { text: kinds }), wcEl("td", { className: "n", text: empty ? wcT("pick.empty") : s.scans.toLocaleString("en") }), wcEl("td", { className: "n", text: s.size ? wcMb(s.size) : "" })]));
      }
      tb.append(tbody); body.append(wcEl("div", { className: "wc-scroll" }, [tb]));
    }
    const x = wcEl("button", { type: "button", className: "wc-x", text: "\u00d7" });
    x.setAttribute("aria-label", wcT("pick.close"));
    const head = wcEl("div", { className: "wc-head" }, [wcEl("h2", { id: "wc-title", text: wcT("pick.title") }), x]);
    dlg.append(head, wcEl("p", { className: "wc-intro", text: wcT("pick.intro") }), wcEl("div", { className: "wc-bar" }, [all, none, status]), body, wcEl("div", { className: "wc-act" }, [ok]));
    let result = /** @type {Map<string, number[]> | null} */ (null);
    const finish = () => { dlg.close(); dlg.remove(); if (prior && prior.isConnected) prior.focus(); done(result); };
    ok.onclick = () => { result = new Map(entries.map(e => [e.key, boxes.filter(b => b.e === e && b.cb.checked).map(b => b.s.id)])); finish(); };
    x.onclick = finish;
    dlg.addEventListener("cancel", ev => { ev.preventDefault(); finish(); });     // Esc
    // Enter opens, except on a button (where it presses that button)
    dlg.addEventListener("keydown", ev => { const t = /** @type {HTMLElement} */ (ev.target); if (ev.key === "Enter" && t.tagName !== "BUTTON" && !ok.disabled) { ev.preventDefault(); ok.click(); } });
    document.body.appendChild(dlg); upd(); dlg.showModal();
    ok.focus();
  });
}

/**
 * Asks which samples to open. `pairs` = [{ key, name, mem }] (`mem`: the members of the dataset as `rb.nativeSamples` takes them), `mode` = "several" | "always" | "never".
 * Resolves with `null` when nothing needs asking (the setting, one sample per file, a listing that failed: opening goes on as it always did), `{ cancelled: true }`,
 * or `{ picks: Map(key -> ids), total: Map(key -> samples in the file), names: Map(key -> listed samples) }` for the files that were asked.
 */
async function wcAsk(/** @type {{ key: string, name: string, mem: { name: string, blob: Blob }[] }[]} */ pairs, /** @type {any} */ rb, /** @type {string} */ mode) {
  if (mode === "never" || !rb || !rb.nativeSamples || !rb.nativeAsk) return null;
  /** @type {Entry[]} */ const entries = [];
  for (const p of pairs) {
    try {
      const j = await rb.nativeSamples(p.mem, true), samples = /** @type {Smp[]} */ (j.samples || []);
      if (!samples.length) continue;
      const d = await rb.nativeAsk({ op: "natdefault", ids: samples.map(s => s.id), sizes: samples.map(s => s.size || 0), empty: samples.filter(s => s.why === "empty").map(s => s.id) });
      entries.push({ key: p.key, name: p.name, samples, ticked: new Set(d.samples) });
    } catch (_) { /* the normal opening reports a file that cannot be read */ }
  }
  if (!entries.length || mode !== "always" && entries.every(e => e.samples.length < 2)) return null;
  const picks = await wcDialog(entries);
  return picks ? { picks, total: new Map(entries.map(e => [e.key, e.samples.length])), names: new Map(entries.map(e => [e.key, e.samples])) } : { cancelled: true };
}
const WIFFCHOOSE = { ask: wcAsk, dialog: wcDialog };
