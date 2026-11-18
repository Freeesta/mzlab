// @ts-check
// The one drop handler of the Dati view: every drop target (landing zone, method zone, side list, work area) binds through DROP.target; the page-wide
// guard keeps a missed drop from opening the file. Accepted types, the unknown-extension rule and the note come from upload (shell.js) and OPENNOW.
"use strict";

const DROP = (() => {
  const has = (/** @type {DragEvent} */ e) => [...((e.dataTransfer && e.dataTransfer.types) || [])].includes("Files");
  /** file handles of a picked or dropped .wiff / .wiff.scan (Chromium), by lower-case name: the folder dialog of wiff-pair.js opens in their folder */
  const handles = /** @type {Map<string, any>} */ (new Map());
  const keep = (/** @type {DataTransfer} */ dt) => { for (const i of [...dt.items || []]) { const n = i.kind === "file" && /\.wiff(\.scan)?$/i.test((i.getAsFile() || { name: "" }).name) && /** @type {any} */ (i).getAsFileSystemHandle ? i : null; if (n) /** @type {any} */ (n).getAsFileSystemHandle().then((/** @type {any} */ h) => { if (h) handles.set(h.name.toLowerCase(), h); }, () => {}); } };
  /** @param {HTMLElement} el @param {{ kind?: string, ready?: (e: DragEvent) => boolean, over?: (on: boolean) => void }} o */
  function target(el, o) {
    const ready = o.ready || (() => true), set = o.over || (on => el.classList.toggle("over", on));
    ["dragenter", "dragover"].forEach(t => el.addEventListener(t, (/** @type {DragEvent} */ e) => { if (!ready(e)) return; e.preventDefault(); set(true); }, true));
    el.addEventListener("dragleave", (/** @type {DragEvent} */ e) => { if (!el.contains(/** @type {Node | null} */ (e.relatedTarget))) set(false); }, true);
    el.addEventListener("drop", (/** @type {DragEvent} */ e) => {
      set(false);
      if (!ready(e) || !e.dataTransfer) return;
      e.preventDefault(); e.stopPropagation();
      keep(e.dataTransfer);
      if (o.kind === "dam") upload([...e.dataTransfer.files], "dam");
      else if (!CartCarica.fromDrop(e)) upload([...e.dataTransfer.files], "data");
    }, true);
  }
  addEventListener("dragover", e => e.preventDefault()); addEventListener("drop", e => e.preventDefault());
  return { has, target, handles };
})();
