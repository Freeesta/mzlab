// @ts-check
// Flow (administrator mode): the page. Loads the three lists of descriptors, keeps the plan (an mzflow/2 object), draws the palette, the inputs and the chain,
// validates after every change and exports / imports the plan. Running the chain is not here: the plan is run by the native runner (mzlab-cli, example `flow`).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const LISTS = ["steps-1.json", "steps-attributio-1.json", "steps-tabui-1.json"];
  const CLI = (/** @type {string} */ file) => `cargo run --release -p mzlab-cli --example flow -- ${file} workdir 4`;

  /** every problem of the plan as the Rust checker would report it (F.validateExtra adds the checks of what the checker does not know yet) @param {any} plan @param {any} reg */
  F.validate = (plan, reg) => {
    try { return [...F.check(F.normalize(F.cleanRefs(F.withoutCompare ? F.withoutCompare(plan) : plan)), reg, "native"), ...(F.validateExtra ? F.validateExtra(plan, reg) : [])]; }
    catch (e) { const x = /** @type {any} */ (e); return [{ instance: "", port: "", key: x.key || "err.flow.json", params: x.params || {} }]; }
  };

  /** the page in `sec`; `lists` are the three parsed descriptor files @param {HTMLElement} sec @param {any[][]} lists */
  F.mount = (sec, lists) => {
    const { el, btn } = F, reg = F.registry(lists);
    /** @type {any} */ const S = { plan: F.newPlan(), reg, problems: [], history: [], snap: "", cli: false, note: "" };
    const pal = F.palette(reg, stepId => { F.addStep(S.plan, reg, stepId); refresh(`fl-card-${S.plan.steps[S.plan.steps.length - 1].id}`); });
    const top = el("div", { class: "fl-top" }), inputs = el("div", { class: "fl-inputs" }), chain = el("div"), goal = el("div", { class: "fl-goal", role: "status" });
    const cli = el("div"), main = el("div", { class: "fl-main" }, inputs, goal, chain, cli);
    const root = el("div", { class: "fl", id: "fl-root" }, top, pal.root, main);
    sec.textContent = ""; sec.append(root);

    const download = () => {
      const a = el("a", { href: URL.createObjectURL(new Blob([F.exportText(S.plan)], { type: "application/json" })), download: "flow.mzflow.json" });
      document.body.append(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    };
    const fileInput = el("input", { type: "file", accept: ".json,application/json", hidden: true, id: "fl-file", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files[0]; e.target.value = "";
      if (!f) return;
      const r = F.importText(await f.text());
      if (r.error) { S.note = F.message({ key: r.error.key, params: r.error.params }); refresh(); return; }
      S.plan = r.plan; S.note = `Opened ${f.name}.`; refresh();
    } });
    const undo = btn("undo", "Undo", () => { const prev = S.history.pop(); if (prev) { S.plan = JSON.parse(prev); S.snap = prev; refresh(null, true); } });
    const status = el("span", { class: "fl-st", role: "status", id: "fl-status" });

    /** the inputs strip: one chip per file with its type, a resolution choice for scan data, and a choice to add one */
    const drawInputs = () => {
      inputs.textContent = "";
      inputs.append(el("strong", { text: "Inputs" }));
      for (const f of S.plan.inputs) {
        const res = ["raw", "mzml", "mzidx"].includes(f.type) ? el("select", { "aria-label": `Resolution of ${f.name}`, onchange: (/** @type {any} */ e) => { if (e.target.value) f.facets = { ...(f.facets || {}), resolution: [e.target.value] }; else delete f.facets; refresh(); } },
          ...[["", "resolution: any"], ["hr", "high resolution"], ["lr", "unit resolution"]].map(([v, t]) => el("option", { value: v, text: t, selected: (f.facets && f.facets.resolution && f.facets.resolution[0] || "") === v }))) : null;
        inputs.append(el("span", { class: "fl-chip" }, F.mark(f.type), el("span", { text: f.name }), res, btn("x", `Remove input ${f.name}`, () => { F.removeInput(S.plan, f.name); refresh(); })));
      }
      inputs.append(el("select", { id: "fl-add-input", "aria-label": "Add an input file", onchange: (/** @type {any} */ e) => { if (e.target.value) { F.addInput(S.plan, e.target.value); refresh(); } } },
        el("option", { value: "", text: "+ input file…" }), ...F.INPUT_TYPES.map((/** @type {string} */ t) => el("option", { value: t, text: t }))));
    };
    const drawGoal = () => {
      const g = F.goal(S.plan, reg);
      goal.classList.toggle("ok", g.reached);
      const names = (/** @type {string[]} */ ids) => ids.map(i => reg.byId.get(i).title).join(", ");
      goal.textContent = g.reached ? "The chain reaches the identification level."
        : g.canReach ? `The chain does not reach the identification level yet.${g.next.length ? " Next: " + names(g.next) + "." : ""}`
          : `The chain cannot reach the identification level from these inputs. Add an input of type ${g.missing.join(" and ") || "…"}.`;
    };
    const drawCli = () => {
      cli.textContent = "";
      if (!S.cli) return;
      const text = CLI("flow.mzflow.json");
      cli.append(el("div", { class: "fl-cli" }, el("strong", { text: "Run with the command line" }),
        el("p", { text: "Flow does not run the chain. Download the plan, put it in a folder with the input files, and run it with the native runner:" }), el("code", { text }),
        el("div", { class: "fl-meta" }, btn("export", "Download the plan", download, "Download plan"), btn("copy", "Copy the command", () => navigator.clipboard && navigator.clipboard.writeText(text), "Copy command")),
        S.problems.length ? el("p", { class: "fl-note", text: `The plan has ${S.problems.length} problem(s); the runner refuses it until they are solved.` }) : null));
    };
    const note = el("span", { class: "fl-note", role: "status", id: "fl-msg" });
    // the bar is built once: a click is never lost to a redraw caused by the field that just lost focus
    top.append(el("h2", { text: "Flow" }), undo, btn("import", "Open a plan (mzflow/2)", () => fileInput.click(), "Open"), fileInput, btn("export", "Export the plan as mzflow/2 JSON", download, "Export"),
      btn("run", "Export and run with the command line", () => { S.cli = !S.cli; refresh(); }, "Run with the CLI"),
      btn("reset", "Start a new plan", () => { S.plan = F.newPlan(); S.note = ""; refresh(); }, "New"), status, note);
    const drawTop = () => {
      const bad = S.problems.length;
      status.className = "fl-st " + (bad ? "bad" : "ok");
      status.replaceChildren(F.icon(bad ? "warn" : "ok"), `${S.plan.steps.length} step${S.plan.steps.length === 1 ? "" : "s"} · ${bad ? bad + " problem" + (bad === 1 ? "" : "s") : "valid"}`);
      note.textContent = S.note;
      undo.disabled = !S.history.length;
    };
    /** the elements above `n` that scroll, the page itself included @param {HTMLElement} n */
    const scrollers = n => { const out = [/** @type {HTMLElement} */ (document.scrollingElement || document.documentElement)]; for (let p = n.parentElement; p; p = p.parentElement) if (p.scrollHeight > p.clientHeight) out.push(p); return out; };
    /** validates, remembers the step for undo and draws everything again; focus returns to the element it was on @param {string | null} [focus] @param {boolean} [fromUndo] */
    function refresh(focus, fromUndo) {
      const active = /** @type {HTMLElement | null} */ (document.activeElement), key = focus === undefined ? active && active.dataset && active.dataset.fk : focus;
      const now = JSON.stringify(S.plan);
      if (!fromUndo && S.snap && now !== S.snap) { S.history.push(S.snap); if (S.history.length > 50) S.history.shift(); }
      S.snap = now;
      // the chain is drawn again from nothing: the page keeps its place, or a click that caused the redraw would land elsewhere
      const held = /** @type {[HTMLElement, number][]} */ (scrollers(root).map(n => [n, n.scrollTop]));
      S.problems = F.validate(S.plan, reg);
      drawInputs(); drawGoal(); pal.update(S.plan);
      chain.replaceChildren(F.chain(S, refresh)); drawCli(); drawTop();
      for (const [n, y] of held) n.scrollTop = y;
      if (key) { const t = root.querySelector(`[data-fk="${CSS.escape(key)}"], #${CSS.escape(key)}`); if (t instanceof HTMLElement) t.focus({ preventScroll: !key.startsWith("fl-card-") }); }
      sec.dispatchEvent(new CustomEvent("flow:changed", { bubbles: true }));
    }
    /** @type {any} */ (sec).__flow = S;
    refresh(null);
  };
})();
