// @ts-check
// TPMINE-PRIVATE  TPUI: two small controls of the page of Tabui that 20_tpmine.js only wires. `sidePanel(sec)` is the button that hides the loading panel (the choice is kept in
// localStorage `mzlab.tabui.side`, "0" = hidden); `removeFiles(o)` takes rows out of the list of files and announces it to screen readers; Ctrl/Cmd+Z puts the latest removal back.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /** @param {HTMLElement} sec the section of Tabui, already filled (`.tp`, `#tp-sb`) */
  function sidePanel(sec) {
    const tp = /** @type {HTMLElement} */ (sec.querySelector(".tp")), b = /** @type {HTMLButtonElement} */ (sec.querySelector("#tp-sb"));
    const set = (/** @type {boolean} */ hide, /** @type {boolean} */ keep) => {
      tp.classList.toggle("tp-nol", hide); b.setAttribute("aria-pressed", String(hide)); b.textContent = hide ? "Show panel" : "Hide panel"; b.setAttribute("aria-label", hide ? "Show the loading panel" : "Hide the loading panel");
      if (keep) try { localStorage.setItem("mzlab.tabui.side", hide ? "0" : "1"); } catch (_) { /* storage may be blocked */ }
    };
    let was = false; try { was = localStorage.getItem("mzlab.tabui.side") === "0"; } catch (_) { /* storage may be blocked */ }
    set(was, false); b.onclick = () => set(!tp.classList.contains("tp-nol"), true);
  }

  /** @type {HTMLElement | null} */ let live = null;
  /** Polite announcement for screen readers (no visible notice). @param {string} text */
  function say(text) {
    if (!live) { live = document.createElement("div"); live.className = "sr-only"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); document.body.appendChild(live); }
    live.textContent = ""; const l = live; setTimeout(() => { l.textContent = text; }, 30);
  }

  /** Removals taken out of the list, latest last; Ctrl/Cmd+Z puts the latest back (in place of a visible Undo notice). @type {(() => void)[]} */
  const undos = [];
  document.addEventListener("keydown", e => {
    if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== "z" || !undos.length) return;
    const t = /** @type {HTMLElement | null} */ (e.target);
    if (t && t.closest("input, textarea, select, [contenteditable], dialog[open]")) return;
    if (!document.querySelector("#tp-files")) return;
    e.preventDefault(); /** @type {() => void} */ (undos.pop())();
  });

  /**
   * @param {{ st: any, idx: number[], busy: () => boolean, say: (t: string, err?: boolean) => void, key: (n: string) => string, label: (n: string) => string,
   *   sort: () => void, redraw: () => void, pick: HTMLElement }} o st.files are the rows; `key` tells the same sample; `redraw` repaints the table and the estimates
   */
  function removeFiles(o) {
    const { st } = o, gone = o.idx.map(i => st.files[i]).filter(Boolean); if (!gone.length) return;
    if (o.busy()) return o.say("Wait for the Dig to finish before removing files.", true);
    const first = Math.min(...o.idx), at = o.idx.filter(i => st.files[i]).sort((a, b) => a - b);
    gone.sort((a, b) => st.files.indexOf(a) - st.files.indexOf(b));
    st.files = st.files.filter((/** @type {any} */ f) => !gone.includes(f)); if (!st.files.length) st.noAuto = true;
    o.redraw();
    const next = /** @type {HTMLElement|null} */ (document.querySelector(`#tp-files [data-rm="${Math.min(first, st.files.length - 1)}"]`)); (next || o.pick).focus();
    undos.push(() => {
      // back at the old positions (the sort is stable: rows with equal kind and time keep their place)
      gone.forEach((/** @type {any} */ f, /** @type {number} */ i) => {
        if (!st.files.some((/** @type {any} */ x) => o.key(x.name) === o.key(f.name))) st.files.splice(Math.min(at[i], st.files.length), 0, f);
      });
      st.noAuto = false; o.sort(); o.redraw();
      say(gone.length === 1 ? `Restored ${o.label(gone[0].name)}` : `Restored ${gone.length} files`);
    });
    if (undos.length > 20) undos.shift();
    say((gone.length === 1 ? `Removed ${o.label(gone[0].name)}` : `Removed ${gone.length} files`) + ". Press Ctrl+Z to undo.");
  }

  G.TPUI = { sidePanel, removeFiles };
})();
