// @ts-check
// Flow (administrator mode): the shell around the canvas. Top bar (templates, add an analysis, Simple | Advanced, undo, fit, checklist, run), the three views (Canvas,
// List, Plan) over ONE plan, the inspector, the empty-state template cards and small menus. The List view is the editor of F.mount (also the accessible alternative to
// the canvas); both draw the same plan and redraw on `flow:changed`. Running goes through F.runners: the command line, and «This computer» in the desktop app (flow-desktop.js).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  /** where a plan can run: each runner has a name and a function; the desktop and web runners plug in here later */
  F.runners = { cli: { name: "Command line", run: (/** @type {any} */ S) => ({ title: "Run with the command line", text: F.cliCommand("flow.mzflow.json"), plan: S }) } };

  /** a small menu under `anchor`; items {label, line, on?, run}. Closes on Escape, on a click outside and after a choice @param {HTMLElement} anchor @param {any[]} items @param {HTMLElement} host */
  F.menu = (anchor, items, host) => {
    host.querySelectorAll(".flx-pop").forEach(n => n.remove());
    const pop = F.el("div", { class: "flx-pop", role: "menu" });
    const shut = () => { pop.remove(); document.removeEventListener("pointerdown", out, true); anchor.setAttribute("aria-expanded", "false"); };
    const close = () => { shut(); anchor.focus({ preventScroll: true }); };
    const out = (/** @type {Event} */ e) => { if (!pop.contains(/** @type {Node} */ (e.target)) && e.target !== anchor) shut(); };
    anchor.setAttribute("aria-haspopup", "menu"); anchor.setAttribute("aria-expanded", "true");
    for (const it of items) pop.append(F.el("button", { type: "button", role: "menuitem", class: it.on ? "on" : "", onclick: () => { shut(); it.run(); } },
      F.el("span", { text: (it.on ? "✓ " : "") + it.label }), it.line ? F.el("small", { text: it.line }) : null));
    if (!items.length) pop.append(F.el("p", { class: "fl-note", text: "Nothing to add here." }));
    pop.addEventListener("keydown", e => { if (e.key === "Escape") close(); else if (e.key === "ArrowDown" || e.key === "ArrowUp") { const b = [...pop.querySelectorAll("button")], i = b.indexOf(/** @type {any} */ (document.activeElement)); b[(i + (e.key === "ArrowDown" ? 1 : -1) + b.length) % b.length].focus(); e.preventDefault(); } });
    host.append(pop);
    const r = anchor.getBoundingClientRect(), h = host.getBoundingClientRect();
    pop.style.left = Math.max(0, Math.min(r.left - h.left, h.width - 280)) + "px"; pop.style.top = r.bottom - h.top + 4 + "px";
    document.addEventListener("pointerdown", out, true);
    const first = pop.querySelector("button"); if (first) /** @type {HTMLElement} */ (first).focus();
  };

  /** the page: {S} is the shared state of F.mount @param {HTMLElement} sec @param {any[][]} lists */
  F.shell = (sec, lists) => {
    const { el, btn, icon } = F;
    F.injectShellCss();
    const listHost = el("div", { class: "flx-list", hidden: true }), reg0 = F.registry(lists);
    F.mount(listHost, lists, F.basePlan(reg0));
    const S = /** @type {any} */ (listHost).__flow;
    let simple = true, tab = "canvas", selected = "";
    try { simple = localStorage.getItem("mzlab.flow.mode") !== "advanced"; } catch (_) { /* private mode */ }
    const live = el("div", { class: "flx-toast", role: "status", id: "flx-live" });
    const say = (/** @type {string} */ msg, /** @type {(() => void) | null} */ undo) => { live.replaceChildren(el("span", { text: msg }), undo ? btn("undo", "Undo", () => { undo(); }, "Undo") : null); };
    /** a change made by the canvas: edit, then let the shared refresh redraw everything @param {() => void} fn @param {string} [focus] */
    const go = (fn, focus) => { fn(); S.refresh(focus === undefined ? null : focus); };
    const cvUi = {
      select: (/** @type {string} */ k) => { selected = k; redraw(); const t = el0(k); if (t) t.focus({ preventScroll: true }); },
      remove: (/** @type {any} */ n) => { go(() => { for (const id of n.ids) F.removeStep(S.plan, id); if (n.kind === "input") F.removeInput(S.plan, n.file.name); }, ""); selected = ""; redraw(); say(`Removed ${n.title}.`, S.undo); },
      next: (/** @type {any} */ n, /** @type {HTMLElement} */ anchor) => {
        const inst = F.instanceOf(S.plan, F.lastOf(n.ids));
        const items = F.nextSteps(S, inst).filter((/** @type {any} */ x) => !F.findStep(S.plan, x.step.id)).slice(0, 7).map((/** @type {any} */ x) => ({ label: F.nameOf(x.step.id)[0], line: F.nameOf(x.step.id)[2], run: () => { let made = /** @type {any} */ (null); go(() => { made = F.addNext(S, inst, x); }); selected = made.id; redraw(); cv.reveal(made.id); } }));
        F.menu(anchor, items, root);
      },
      selected: () => selected, simple: () => simple,
    };
    const el0 = (/** @type {string} */ k) => cv.view.querySelector(`[data-key="${CSS.escape(k)}"]`);
    const cv = F.canvas(S, cvUi);
    const inspHost = el("aside", { class: "flx-insp", "aria-label": "Settings of the selected step" }), palHost = el("aside", { class: "flx-pal flx-adv-only", "aria-label": "All steps" });
    const pal = F.palette(reg0, (/** @type {string} */ id) => { let made = /** @type {any} */ (null); go(() => { made = F.addStep(S.plan, S.reg, id); }); selected = made.id; redraw(); cv.reveal(made.id); });
    palHost.append(pal.root); const pf = pal.root.querySelector("#fl-filter"); if (pf) pf.id = "flx-filter";
    const tplBox = el("div", { class: "flx-tpl", role: "dialog", "aria-label": "Start from a template" });
    tplBox.addEventListener("keydown", e => { if (e.key === "Escape") { dismissed = true; redraw(); tplBtn.focus(); } });
    cv.root.append(tplBox);
    const planView = el("pre", { class: "flx-plan", tabindex: 0, "aria-label": "The plan as mzflow/2" });
    const status = el("span", { class: "fl-st", role: "status", id: "flx-status" });

    const model = () => F.canvasModel(S, simple);
    const nodeOfStep = (/** @type {string} */ id) => model().nodes.find((/** @type {any} */ n) => n.ids.includes(id));
    function redraw() {
      const m = model();
      F.layout(m.nodes, m.edges);
      if (selected && !m.nodes.some((/** @type {any} */ n) => n.key === selected)) selected = "";
      cv.draw(m);
      const node = m.nodes.find((/** @type {any} */ n) => n.key === selected) || null;
      inspHost.replaceChildren(F.inspector(S, node, { simple, done: (/** @type {string | undefined} */ f) => S.refresh(f), select: cvUi.select, remove: cvUi.remove, model: m }));
      pal.update(S.plan);
      const bad = S.problems.length;
      status.className = "fl-st " + (bad ? "bad" : "ok");
      status.replaceChildren(icon(bad ? "warn" : "ok"), bad ? `${bad} to fix` : "Ready to run");
      check.title = bad ? "Show what needs attention" : "Nothing needs attention";
      run.title = bad ? "The plan has things to fix first. The runner refuses it until they are solved." : "Export the plan and run it";
      planView.textContent = F.exportText(S.plan);
      tplBox.classList.toggle("show", !F.hasAddons(S.plan) && tab === "canvas" && !dismissed);
      root.classList.toggle("flx-simple", simple);
      const planTab = tabs[2]; planTab.hidden = simple; if (simple && tab === "plan") setTab("canvas");
      body.classList.toggle("flx-adv", !simple);
      for (const b of modeBtns) b.setAttribute("aria-pressed", String((b.dataset.mode === "simple") === simple));
    }
    let dismissed = false;
    const tplCards = () => {
      tplBox.replaceChildren(el("h3", { text: "Start from a template" }), el("p", { class: "fl-note", text: "The base is already here. A template adds analyses to it; you can change everything afterwards." }),
        el("div", { class: "flx-tpls" }, ...F.TEMPLATES.map((/** @type {any} */ t) => el("button", { type: "button", class: "flx-tc", "data-tpl": t.id, onclick: () => applyTemplate(t) },
          el("span", { class: "flx-pre", "aria-hidden": "true", style: "--cc:" + F.CATS.input.color }, el("i"), el("b"), el("i", { style: "--cc:" + F.CATS.process.color }), ...t.addons.flatMap((/** @type {string} */ _) => [el("b"), el("i", { style: "--cc:" + F.CATS.search.color })])),
          el("strong", { text: t.name }), el("small", { text: t.good })))),
        el("div", { class: "fl-meta" }, btn("x", "Start from the base only", () => { dismissed = true; redraw(); }, "Start from scratch")));
    };
    const applyTemplate = (/** @type {any} */ t) => { go(() => { S.plan = F.templatePlan(reg0, t); }); selected = ""; dismissed = true; redraw(); cv.fit(); say(`Template “${t.name}” loaded. Undo goes back.`, S.undo); };

    const modeBtns = ["simple", "advanced"].map(m => el("button", { type: "button", "data-mode": m, "aria-pressed": String((m === "simple") === simple), onclick: () => { simple = m === "simple"; try { localStorage.setItem("mzlab.flow.mode", m); } catch (_) { /* ignore */ } selected = ""; redraw(); cv.fit(); } }, el("span", { text: m === "simple" ? "Simple" : "Advanced" })));
    const tplBtn = btn("flow", "Start from a template", () => { dismissed = false; tplCards(); tplBox.classList.add("show"); const f = tplBox.querySelector("button"); if (f) /** @type {HTMLElement} */ (f).focus(); }, "Templates");
    const addBtn = btn("plus", "Add an analysis to the base", (/** @type {Event} */ e) => F.menu(/** @type {HTMLElement} */ (e.currentTarget), F.ADDONS.map((/** @type {any} */ a) => ({ label: a.name, line: a.line, on: a.steps.every((/** @type {string} */ s) => F.findStep(S.plan, s)),
      run: () => { let made = /** @type {any} */ (null); go(() => { made = F.addAddon(S, a); }); if (made) { const n = nodeOfStep(made.id); selected = n ? n.key : ""; redraw(); if (n) cv.reveal(n.key); } } })), root), "Add analysis");
    addBtn.id = "flx-add";
    const undoBtn = btn("undo", "Undo the last change", () => S.undo(), "Undo");
    const fitBtn = btn("fit", "Fit all steps in view", () => cv.fit(), "Fit");
    const check = btn("warn", "Show what needs attention", (/** @type {Event} */ e) => {
      const items = S.problems.map((/** @type {any} */ p) => { const f = F.friendly(S, p), n = nodeOfStep(p.instance); return { label: `${n ? n.title : p.instance}: ${f.text}`, line: "Show me", run: () => { if (n) { selected = n.key; redraw(); cv.reveal(n.key); } } }; });
      F.menu(/** @type {HTMLElement} */ (e.currentTarget), items, root);
    }, "Checklist");
    const run = btn("run", "Export the plan and run it", () => showRun(), "Run");
    run.classList.add("flx-run"); run.id = "flx-run";
    const showRun = () => {
      const r = F.runners.cli.run(S);
      planView.hidden = false;
      const box = el("div", { class: "fl-cli" }, el("strong", { text: r.title }), el("p", { text: F.runners.desktop.available() ? "Run it here, or download the plan and use the command line:" : "On the web Flow exports the plan. Download it, put it in a folder with your files and run it with the native runner:" }), el("code", { text: r.text }),
        el("div", { class: "fl-meta" }, btn("export", "Download the plan", S.download, "Download plan"), btn("copy", "Copy the command", () => navigator.clipboard && navigator.clipboard.writeText(r.text), "Copy command"), btn("x", "Close", () => box.remove(), "Close")));
      if (F.runners.desktop.available()) box.append(F.desktopRunUi(S));
      root.querySelectorAll(".fl-cli").forEach(n => n.remove()); bar.after(box);
    };
    const open = btn("import", "Open a plan (mzflow/2)", () => { const f = listHost.querySelector("#fl-file"); if (f) /** @type {HTMLElement} */ (f).click(); }, "Open");
    const bar = el("div", { class: "flx-bar" }, el("h2", { text: "Flow" }), tplBtn, addBtn, el("span", { class: "flx-seg", role: "group", "aria-label": "Mode" }, ...modeBtns), undoBtn, fitBtn, check, open, run, status);
    const tabs = ["canvas", "list", "plan"].map(t => el("button", { type: "button", role: "tab", "aria-selected": String(t === tab), "data-tab": t, onclick: () => setTab(t) }, el("span", { text: t === "canvas" ? "Canvas" : t === "list" ? "List view" : "Plan (JSON)" })));
    const body = el("div", { class: "flx-body" }, palHost, cv.root, inspHost);
    const root = el("div", { class: "fl flx", id: "flx-root" }, bar, el("div", { class: "flx-tabs", role: "tablist", "aria-label": "Views" }, ...tabs), live, body, planView, listHost);
    function setTab(/** @type {string} */ t) {
      tab = t; tabs.forEach(b => b.setAttribute("aria-selected", String(b.dataset.tab === t)));
      body.hidden = t !== "canvas"; planView.hidden = t !== "plan"; listHost.hidden = t !== "list"; redraw(); if (t === "canvas") cv.fit();
    }
    sec.textContent = ""; sec.append(root);
    listHost.addEventListener("flow:changed", () => { undoBtn.disabled = !S.history.length; redraw(); });
    tplCards(); planView.hidden = true; undoBtn.disabled = true;
    redraw(); requestAnimationFrame(() => cv.fit());
    S.redraw = redraw; /** @type {any} */ (sec).__flow = S; /** @type {any} */ (sec).__shell = { select: cvUi.select, model };
  };
})();
