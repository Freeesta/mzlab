// @ts-check
// TPMINE-PRIVATE  TPUI: two small controls of the page of Tabui that 20_tpmine.js only wires. `sidePanel(sec)` is the button that hides the loading panel (the choice is kept in
// localStorage `mzlab.tabui.side`, "0" = hidden); `removeFiles(o)` takes rows out of the list of files and shows «Removed · Undo» (the box of the notices of the Data tab).
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

  /** «Removed · Undo» in `#tolto-box` (same look as the Data tab; 6 s, it stops under the pointer or the focus) @param {string} text @param {() => void} undo */
  function notice(text, undo) {
    let box = document.getElementById("tolto-box");
    if (!box) { box = document.createElement("div"); box.id = "tolto-box"; box.setAttribute("role", "status"); box.setAttribute("aria-live", "polite"); document.body.appendChild(box); }
    const n = document.createElement("div"), t = document.createElement("span"), b = document.createElement("button");
    n.className = "tolto"; t.textContent = text; b.type = "button"; b.textContent = "Undo";
    n.append(t, b); box.appendChild(n);
    const close = typeof G.avvisoAuto === "function" ? G.avvisoAuto(n, { ms: 6000, x: false }) : () => n.remove();
    b.onclick = () => { close(); undo(); };
  }

  /**
   * @param {{ st: any, idx: number[], busy: () => boolean, say: (t: string, err?: boolean) => void, key: (n: string) => string, label: (n: string) => string,
   *   sort: () => void, redraw: () => void, pick: HTMLElement }} o st.files are the rows; `key` tells the same sample; `redraw` repaints the table and the estimates
   */
  function removeFiles(o) {
    const { st } = o, gone = o.idx.map(i => st.files[i]).filter(Boolean); if (!gone.length) return;
    if (o.busy()) return o.say("Wait for the Dig to finish before removing files.", true);
    const first = Math.min(...o.idx);
    st.files = st.files.filter((/** @type {any} */ f) => !gone.includes(f)); if (!st.files.length) st.noAuto = true;
    o.redraw();
    const next = /** @type {HTMLElement|null} */ (document.querySelector(`#tp-files [data-rm="${Math.min(first, st.files.length - 1)}"]`)); (next || o.pick).focus();
    notice(gone.length === 1 ? `Removed ${o.label(gone[0].name)}` : `Removed ${gone.length} files`, () => {
      st.files = st.files.concat(gone.filter((/** @type {any} */ f) => !st.files.some((/** @type {any} */ x) => o.key(x.name) === o.key(f.name)))); st.noAuto = false; o.sort(); o.redraw();
    });
  }

  G.TPUI = { sidePanel, removeFiles };
})();
