// @ts-check
// Flow (administrator mode): what the Run buttons of the bar do. «Run» runs the plan where Flow can (this computer in the desktop app, else this browser);
// «Terminal» opens the panel of flow-terminal.js (the command line of the native runner, copied at once, editable, runnable here). Used by flow-shell.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** the command with the folder of the user in the place of the one the engine wrote: quoted once, in single quotes, so spaces are fine @param {string} text @param {string} folder */
  F.cliFill = (text, folder) => {
    const f = folder.trim().replace(/^(['"])(.*)\1$/, "$2");
    return f ? text.replace(/^MZLAB_IN='[^'\n]*'$/m, () => "MZLAB_IN='" + f.replace(/'/g, "'\\''") + "'") : text;
  };

  /** @param {any} S @param {{ place: (box: HTMLElement) => void, root: HTMLElement, say: (msg: string) => void }} c */
  F.runActions = (S, c) => {
    const { el, btn } = F;
    /** @type {HTMLElement | null} */ let shown = null, arrow = null;
    const shut = () => { c.root.querySelectorAll(".fl-cli").forEach(n => n.remove()); shown = null; if (arrow) arrow.setAttribute("aria-expanded", "false"); };

    const panel = async () => {
      shut();
      const box = F.terminalPanel(S, { say: c.say });
      box.addEventListener("fl-close", shut);
      c.place(box); shown = box;
      return box;
    };

    return {
      /** runs the plan here if a runner exists; else shows the command line */
      start: async () => {
        const make = F.runners.desktop.available() ? F.desktopRunUi : F.runners.web.available() ? F.webRunUi : null;
        if (!make) { await panel(); return; }
        shut();
        const box = el("div", { class: "fl-cli fl-runbox" }, make(S), btn("x", "Close", shut, "Close"));
        c.place(box); shown = box;
        const go = /** @type {HTMLButtonElement | null} */ (box.querySelector('button[aria-label^="Run the plan"]'));
        if (go && !go.disabled) go.click();
      },
      /** the Terminal button: show or hide the panel @param {HTMLElement} a */
      toggle: async a => {
        arrow = a;
        if (shown && shown.isConnected && shown.querySelector(".fl-line")) { shut(); return; }
        await panel(); a.setAttribute("aria-expanded", "true");
      },
    };
  };
})();
