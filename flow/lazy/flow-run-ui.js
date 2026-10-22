// @ts-check
// Flow (administrator mode): what the Run buttons of the bar do. «Run» runs the plan where Flow can (this computer in the desktop app, else this browser);
// «Run on CLI» copies the command of the native runner at once and a small arrow shows it in a box you can edit. Used by flow-shell.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** @param {any} S @param {{ place: (box: HTMLElement) => void, root: HTMLElement, say: (msg: string) => void }} c */
  F.runActions = (S, c) => {
    const { el, btn } = F;
    /** @type {HTMLElement | null} */ let shown = null, arrow = null;
    const shut = () => { c.root.querySelectorAll(".fl-cli").forEach(n => n.remove()); shown = null; if (arrow) arrow.setAttribute("aria-expanded", "false"); };
    const text = async () => (await F.runners.cli.run(S)).text;
    const copy = async (/** @type {string} */ t) => { try { await navigator.clipboard.writeText(t); c.say("Copied"); } catch (_) { c.say("Could not copy: select the text and copy it."); } };

    const panel = async () => {
      shut();
      const ta = el("textarea", { class: "fl-cmd", rows: 12, spellcheck: false, "aria-label": "Command for a terminal (you can edit it)" });
      ta.value = await text();
      const box = el("div", { class: "fl-cli" }, el("p", { class: "fl-note", text: "Paste it in a terminal." }), ta,
        el("div", { class: "fl-meta" }, btn("export", "Download the plan", S.download, "Download plan"), btn("copy", "Copy the command", () => copy(ta.value), "Copy"), btn("x", "Close", shut, "Close")));
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
      copy: async () => { await copy(await text()); },
      /** the arrow beside «Run on CLI»: show or hide the editable command @param {HTMLElement} a */
      toggle: async a => {
        arrow = a;
        if (shown && shown.isConnected && shown.querySelector("textarea")) { shut(); return; }
        await panel(); a.setAttribute("aria-expanded", "true");
      },
    };
  };
})();
