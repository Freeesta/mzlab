// @ts-check
// Flow (administrator mode): the run box of «This browser» (flow-web-run.js). One line per step fed by the events of the runner (waiting, file k of n, the memory the step asks for and
// whether it works in memory or in parts, done, reused, left out because it needs a computer, refused with its numbers); a note on phones and tablets; Run is off when the series
// asks for more memory than the high level of this browser, with the reason beside it. Texts are English like the rest of Flow.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** the words of one event @param {any} ev */
  F.webStepText = ev => {
    const mem = ev.mb ? ` · ≈ ${Math.ceil(ev.mb)} MB${ev.modo === "slabs" ? " · in parts" : ev.modo === "memory" ? " · in memory" : ""}` : "";
    switch (ev.stato) {
      case "waiting": return ev.via === "computer" ? "waiting (needs a computer)" : "waiting";
      case "running": return (ev.di > 1 ? `working, file ${ev.parte} of ${ev.di}` : "working") + mem;
      case "done": return "done" + mem;
      case "reused": return "reused (nothing changed)";
      case "blocked": return `needs about ${Math.ceil(ev.mb || 0)} MB, more than the memory in use: Run asks before using more`;
      case "refused": return `needs about ${Math.ceil(ev.mb || 0)} MB, more than this browser gives`;
      case "error": return "failed";
      default: return ev.via === "computer" ? "left out: this step needs a computer" : "skipped";
    }
  };

  /** the run box of the web @param {any} S */
  F.webRunUi = S => {
    const { el, btn } = F, R = F.runners.web;
    const list = el("ul", { class: "fl-slots" }), note = el("p", { class: "fl-note", role: "status" });
    const why = el("p", { class: "fl-note", role: "note", id: "flx-web-why" });
    const title = (/** @type {any} */ inst) => F.nameOf(String(inst.step).split("@")[0])[0];
    const slots = new Map();
    const reset = () => { list.textContent = ""; slots.clear(); for (const inst of S.plan.steps) { const t = el("span", { text: " — waiting" }); slots.set(inst.id, t); list.append(el("li", {}, el("strong", { text: title(inst) }), t)); } };
    const onStep = (/** @type {Event} */ e) => { const ev = /** @type {any} */ (e).detail, t = slots.get(ev.passo); if (t) t.textContent = " — " + F.webStepText(ev); };
    const go = btn("run", "Run the plan in this browser", async () => {
      go.dataset.busy = "1"; go.disabled = true; note.textContent = "Running…"; reset();
      document.addEventListener("flow:web-step", onStep);
      try {
        const res = await R.run(S);
        note.textContent = "Finished. The results are kept in this browser (OPFS).";
        if (await F.webAfterRun(S)) note.textContent += " The feature table is open below.";
        // the MSn steps give their texts: one link each to save them
        for (const f of (res && res.files) || []) {
          const a = el("a", { href: URL.createObjectURL(new Blob([f.text], { type: "text/plain" })), download: f.name, text: f.name });
          slots.get(f.id)?.append(" · ", a);
        }
      } catch (e) { note.textContent = F.message(/** @type {any} */ (e)); }
      document.removeEventListener("flow:web-step", onStep);
      go.dataset.busy = ""; gate();
    }, "Run in this browser");
    // Run waits for a valid plan and stays off while the series needs more memory than the high level; the reason is written beside the button
    let est = /** @type {any} */ (null);
    function gate() {
      if (!box.isConnected && box.dataset.shown) { document.removeEventListener("flow:changed", gate); return; }
      box.dataset.shown = "1";
      const bad = S.problems.length, high = R.highMb(), over = est && high && est.mb > high;
      go.disabled = go.dataset.busy === "1" || bad > 0 || !!over;
      if (over) why.textContent = `This series needs about ${Math.ceil(est.mb)} MB; this browser gives at most ${high} MB. Run it in the desktop app or with the command line.`;
      else why.textContent = R.small() ? "Flow is made for computers: on a phone or tablet the memory is small and the run may stop." : "";
      if (bad) note.textContent = `Cannot run yet: the plan has ${bad} problem${bad === 1 ? "" : "s"}. Open the checklist to see what to fix.`;
      else if (note.textContent.startsWith("Cannot run yet")) note.textContent = "";
    }
    reset();
    const box = el("div", { class: "fl-meta" }, go, why, note, list);
    R.preflight(S).then((/** @type {any} */ x) => { est = x; gate(); }).catch(() => { /* no estimate yet: the run decides */ });
    document.addEventListener("flow:changed", gate); gate();
    // the live session keeps the detections: the estimate follows them
    let again = 0;
    document.addEventListener("flow:web-step", () => { clearTimeout(again); again = setTimeout(() => { R.preflight(S).then((/** @type {any} */ x) => { est = x; gate(); }).catch(() => {}); }, 400); });
    return box;
  };
})();
