// @ts-check
// Flow (administrator mode): the live controls of the bar and the announcements. «Live» (on by default) follows the plan and starts its jobs by itself, «Pause» / «Resume» stops and starts
// the launch of new jobs (the ones that run finish), «Run now» appears only where a step waits for the user's OK to use more memory. The client is F.live in the browser and F.desktopLive in the
// app. The state changes of the cards are read out through the status line of the page (#flx-live), at most one every 2 s. English only. Docs: docs/agenti/flow-live.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const GAP = 2000;

  /** the client of this page: the app runs on this computer, the browser in its own pool */
  const client = () => (F.runners.desktop.available() ? F.desktopLive : F.live);

  /** puts the controls before `before` in `bar`; `ctx.say(text)` writes to the status line and `ctx.titleOf(instance)` names a step @param {HTMLElement} bar @param {HTMLElement} before @param {{say: (text: string) => void, titleOf: (instance: string) => string}} ctx */
  const mount = (bar, before, ctx) => {
    if (bar.querySelector(".flx-live-ctl")) return;
    const { el, btn } = F;
    const live = btn("run", "Live: follow the plan and run its steps as you change it", () => { const c = client(); c.setLive(!c.following()).catch(() => {}); }, "Live");
    const pause = btn("stop", "Pause: stop starting new jobs", () => { const c = client(); c.setHeld(!c.held()).catch(() => {}); }, "Pause");
    const now = btn("ok", "Run now: use more memory for the steps that wait for your OK", () => { const c = client(); if (c.runNow) c.runNow().catch(() => {}); }, "Run now");
    const group = el("span", { class: "flx-live-ctl", role: "group", "aria-label": "Live run" }, live, pause, now);
    live.id = "flx-live-on"; pause.id = "flx-live-pause"; now.id = "flx-live-now";
    const sync = () => {
      const c = client(), on = c.following(), held = c.held();
      live.setAttribute("aria-pressed", String(on));
      pause.disabled = !on || !c.active();
      pause.setAttribute("aria-pressed", String(held));
      pause.setAttribute("aria-label", held ? "Resume: start the waiting jobs" : "Pause: stop starting new jobs");
      const word = pause.querySelector("span"); if (word) word.textContent = held ? "Resume" : "Pause";
      now.hidden = !F.liveView.askingMemory();
    };
    before.before(group);
    sync();
    // the listeners go with the page: a shell that is drawn again brings its own
    const ctl = () => { if (group.isConnected) sync(); else { document.removeEventListener("flow:live-ctl", ctl); document.removeEventListener("flow:live-view", seen); } };
    document.addEventListener("flow:live-ctl", ctl);

    // the changes of state, one line at most every 2 s (the latest wins); an offer to undo something is never replaced
    let last = 0, pending = /** @type {any} */ (null), timer = 0;
    const say = () => {
      timer = 0;
      if (!pending) return;
      const n = F.liveView.node(pending.instance), toast = document.getElementById("flx-live");
      pending = null;
      if (!n || (toast && toast.querySelector("button"))) return;
      last = Date.now();
      ctx.say(`${ctx.titleOf(n.instance)}: ${F.liveView.look(n).word}`);
    };
    const seen = (/** @type {Event} */ e) => {
      if (!group.isConnected) return ctl();
      sync();
      const list = /** @type {any} */ (e).detail && /** @type {any} */ (e).detail.changes || [];
      if (!list.length) return;
      pending = list[list.length - 1];
      if (!timer) timer = window.setTimeout(say, Math.max(0, last + GAP - Date.now()));
    };
    document.addEventListener("flow:live-view", seen);
  };

  F.liveBar = { mount };
})();
