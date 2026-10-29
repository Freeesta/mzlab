// @ts-check
// Flow (administrator mode): the Terminal panel of the bar. One editable command line (`mzlab-cli flow run plan.json … --set INSTANCE.NAME=VALUE`, built by the engine: route `flow/line`),
// copied when the panel opens; «Run here» sends the edited line back to the engine, which applies its `--set` to the plan exactly as the CLI does, and runs that plan with the runner of the page.
// Used by flow-run-ui.js. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  const CSS = `.flw .fl-term{display:flex;flex-direction:column;gap:6px}.flw .fl-term input.fl-line{width:100%;max-width:none;box-sizing:border-box;font-family:ui-monospace,Menlo,Consolas,monospace;font-size:13px}
.flw .fl-term pre{margin:0;min-height:1.6em;max-height:30vh;overflow:auto;white-space:pre-wrap;word-break:break-word;background:var(--soft);border-radius:6px;padding:6px;font-size:12px}
.flw .fl-term textarea.fl-script{width:100%;box-sizing:border-box;min-height:12em;font-family:ui-monospace,Menlo,Consolas,monospace;font-size:12px}.flw .fl-term .fl-copied{color:var(--accent)}`;

  /** one call of the engine route `flow/line`: the text it answers; rejects with the Error of the engine ({key, params}) @param {any} S @param {string | null} line */
  F.lineCall = async (S, line) => {
    const plan = F.exportText(S.plan), extra = line == null ? {} : { line };
    if (/** @type {any} */ (window).MZLAB_DESKTOP) return F.desktopPost("flow/line", { plan, ...extra });
    const r = await fetch("api/flow/line", { method: "POST", body: JSON.stringify({ plan, registry: S.reg.steps, ...extra }) }), text = await r.text();
    if (r.ok) return text;
    let e = /** @type {any} */ ({}); try { e = JSON.parse(text); } catch (_) { /* a body that is not JSON */ }
    throw { key: e.error_key || "err.flow.run", params: e.params || {} };
  };

  /** the words of an error of the line: the CLI's own text for `err.cli.*` @param {any} e */
  const said = e => (e && e.key && String(e.key).startsWith("err.cli.") && e.params && e.params.message ? e.params.message : F.message(e));

  /** the folder after `--in` in the line, quotes kept as written ("" if none) @param {string} line */
  const folderOf = line => { const m = /--in\s+('[^']*'|"[^"]*"|\S+)/.exec(line); return m ? m[1] : ""; };

  /** @param {any} S @param {{ say: (msg: string) => void }} c @returns {HTMLElement} */
  F.terminalPanel = (S, c) => {
    const { el, btn } = F;
    if (!document.getElementById("fl-term-css")) { const s = document.createElement("style"); s.id = "fl-term-css"; s.textContent = CSS; document.head.append(s); }
    const input = /** @type {HTMLInputElement} */ (el("input", { type: "text", class: "fl-line", spellcheck: false, autocomplete: "off", "aria-label": "Command line" }));
    const out = el("pre", { "aria-live": "polite", "aria-label": "Output", role: "log" });
    const mark = el("span", { class: "fl-copied", role: "status" });
    let dirty = false, busy = false;
    const print = (/** @type {string} */ t) => { out.textContent += (out.textContent ? "\n" : "") + t; out.scrollTop = out.scrollHeight; };
    const copy = async (/** @type {string} */ t) => { try { await navigator.clipboard.writeText(t); mark.textContent = "Copied ✓"; } catch (_) { mark.textContent = "Could not copy: select the text and copy it."; } };
    const fresh = async () => {
      try { input.value = await F.lineCall(S, null); dirty = false; } catch (e) { print(said(e)); }
    };

    /** the lines of the steps: the web runner reports each event, the app lists the steps when the run ends @param {any} plan */
    const runHere = async (/** @type {any} */ plan) => {
      const name = (/** @type {any} */ inst) => F.nameOf(String(inst.step).split("@")[0])[0];
      const keep = S.plan; S.plan = plan;
      try {
        if (F.runners.desktop.available()) {
          const res = await F.runners.desktop.run(S);
          for (const s of res.steps) { const inst = plan.steps.find((/** @type {any} */ i) => i.id === s.id); print(`${inst ? name(inst) : s.id} — ${s.error ? F.message({ key: s.error.key, params: s.error.params }) : s.cached ? "reused (nothing changed)" : `done in ${(s.ms / 1000).toFixed(1)} s`}`); }
          await F.afterRun(S);
        } else if (F.runners.web.available()) {
          print("Files: the ones open in Data.");
          const on = (/** @type {Event} */ e) => { const ev = /** @type {any} */ (e).detail, inst = plan.steps.find((/** @type {any} */ i) => i.id === ev.passo); print(`${inst ? name(inst) : ev.passo} — ${F.webStepText(ev)}`); };
          document.addEventListener("flow:web-step", on);
          try { await F.runners.web.run(S); await F.webAfterRun(S); } finally { document.removeEventListener("flow:web-step", on); }
        } else print("No runner available here.");
      } finally { S.plan = keep; }
    };
    const run = async () => {
      if (busy) return;
      busy = true; go.disabled = true; out.textContent = ""; mark.textContent = "";
      try {
        const got = F.importText(await F.lineCall(S, input.value));
        if (got.error) throw got.error;
        await runHere(got.plan);
      } catch (e) { print(said(e)); }
      busy = false; go.disabled = false;
    };

    const go = /** @type {HTMLButtonElement} */ (btn("run", "Run the plan here with the settings in the line", run, "Run here"));
    const script = /** @type {HTMLTextAreaElement} */ (el("textarea", { class: "fl-script", readOnly: true, spellcheck: false, hidden: true, "aria-label": "Script" }));
    const scriptCopy = btn("copy", "Copy the script", () => copy(script.value), "Copy script"); scriptCopy.hidden = true;
    const show = btn("file", "Show the script that writes the plan and runs it", async () => {
      if (!script.hidden) { script.hidden = true; scriptCopy.hidden = true; show.setAttribute("aria-expanded", "false"); return; }
      script.value = F.cliFill((await F.runners.cli.run(S)).text, folderOf(input.value).replace(/^(['"])(.*)\1$/, "$2"));
      script.hidden = false; scriptCopy.hidden = false; show.setAttribute("aria-expanded", "true");
    }, "Show script");
    show.setAttribute("aria-expanded", "false");
    const reset = btn("undo", "Back to the line of the plan", async () => { await fresh(); mark.textContent = ""; }, "Reset");
    const box = el("div", { class: "fl-cli fl-term" }, input,
      el("div", { class: "fl-meta" }, btn("copy", "Copy the command line", () => copy(input.value), "Copy"), go, reset, show, scriptCopy, mark, btn("x", "Close", () => box.dispatchEvent(new Event("fl-close")), "Close")), out, script);
    input.addEventListener("input", () => { dirty = true; });
    input.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); run(); } });
    // the line follows the plan until the user types in it
    const follow = () => { if (!box.isConnected) document.removeEventListener("flow:changed", follow); else if (!dirty && !busy) fresh(); };
    document.addEventListener("flow:changed", follow);
    fresh().then(() => { if (input.value) copy(input.value); });
    return box;
  };
})();
