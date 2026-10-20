// @ts-check
// Flow (administrator mode): the canvas. Cards laid out from left to right and joined by smooth lines; zoom and pan with buttons, keys, the wheel with Ctrl and a drag
// on the background (the buttons and keys do everything the drag does, WCAG 2.5.7). Draws a model made by F.canvasModel; knows nothing about the plan.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CW = 220, CH = 64;

  /** the one-line summary of a card: what differs from the default, or «Default settings» @param {any} S @param {any} node */
  F.summary = (S, node) => {
    if (node.kind === "input") return node.file.name;
    const parts = [];
    for (const id of node.ids) {
      const inst = F.instanceOf(S.plan, id), step = inst && S.reg.byId.get(inst.step.split("@")[0]);
      if (!inst || !step) continue;
      if (inst.algorithm) parts.push(inst.algorithm);
      for (const [k, v] of Object.entries(inst.params || {})) {
        const d = F.declared(step, F.effective(inst, step).algorithm), p = [...d].find(([n]) => n === k), unit = p && p[1]["x-unit"] ? " " + p[1]["x-unit"] : "";
        parts.push(`${p && p[1].title ? p[1].title.toLowerCase() : k} ${Array.isArray(v) ? v.join("–") : v}${unit}`);
      }
    }
    return parts.length ? parts.slice(0, 2).join(", ") : "Default settings";
  };

  /** the canvas: {root, draw(model), fit()} @param {any} S @param {{select: (key: string) => void, remove: (node: any) => void, next: (node: any, anchor: HTMLElement) => void, selected: () => string, simple: () => boolean}} ui */
  F.canvas = (S, ui) => {
    const { el, icon } = F, ns = "http://www.w3.org/2000/svg";
    const stage = el("div", { class: "fl-stage" }), svg = document.createElementNS(ns, "svg");
    svg.setAttribute("class", "fl-lines"); svg.setAttribute("aria-hidden", "true");
    stage.append(svg);
    const view = el("div", { class: "fl-view", role: "application", tabindex: 0, "aria-label": "Flow canvas. Arrow keys move the view, plus and minus zoom. Tab goes through the steps in order.", id: "fl-canvas" }, stage);
    const v = { x: 0, y: 0, k: 1 }, size = { width: 600, height: 300 };
    const apply = (/** @type {boolean} */ soft) => { stage.classList.toggle("fl-soft", !!soft); stage.style.transform = `translate(${v.x}px,${v.y}px) scale(${v.k})`; };
    const zoom = (/** @type {number} */ f, /** @type {boolean} */ soft) => {
      const cx = view.clientWidth / 2, cy = view.clientHeight / 2, k = Math.min(2, Math.max(0.3, v.k * f));
      v.x = cx - (cx - v.x) * k / v.k; v.y = cy - (cy - v.y) * k / v.k; v.k = k; apply(soft);
    };
    const fit = () => {
      const w = view.clientWidth || 800, h = view.clientHeight || 500;
      v.k = Math.min(1, Math.max(0.3, Math.min(w / size.width, h / size.height))); v.x = Math.max(0, (w - size.width * v.k) / 2); v.y = Math.max(0, Math.min(48, (h - size.height * v.k) / 2)); apply(true);
    };
    const pan = (/** @type {number} */ dx, /** @type {number} */ dy) => { v.x += dx; v.y += dy; apply(false); };
    view.addEventListener("keydown", e => {
      if (e.target !== view) return;
      const step = e.shiftKey ? 160 : 40, m = /** @type {Record<string, [number, number]>} */ ({ ArrowLeft: [step, 0], ArrowRight: [-step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] })[e.key];
      if (m) { pan(m[0], m[1]); e.preventDefault(); } else if (e.key === "+" || e.key === "=") zoom(1.2, true); else if (e.key === "-") zoom(1 / 1.2, true); else if (e.key === "0") fit();
    });
    view.addEventListener("wheel", e => { if (e.ctrlKey) { e.preventDefault(); zoom(e.deltaY < 0 ? 1.1 : 1 / 1.1, false); } }, { passive: false });
    view.addEventListener("pointerdown", e => {
      if (e.target !== view && e.target !== stage && e.target !== svg) return;
      let lx = e.clientX, ly = e.clientY; view.setPointerCapture(e.pointerId); view.classList.add("fl-pan");
      const move = (/** @type {PointerEvent} */ m) => { pan(m.clientX - lx, m.clientY - ly); lx = m.clientX; ly = m.clientY; };
      const up = () => { view.removeEventListener("pointermove", move); view.removeEventListener("pointerup", up); view.classList.remove("fl-pan"); };
      view.addEventListener("pointermove", move); view.addEventListener("pointerup", up);
    });
    const tools = el("div", { class: "fl-zoom", role: "group", "aria-label": "View" },
      F.btn("plus", "Zoom in", () => zoom(1.25, true)), F.btn("minus", "Zoom out", () => zoom(1 / 1.25, true)), F.btn("fit", "Fit all steps in view", fit), F.btn("one", "Show at 100%", () => { v.k = 1; v.x = 0; v.y = 0; apply(true); }));
    const root = el("div", { class: "fl-cv" }, view, tools);

    // keys already drawn: only a card that is new plays the entrance animation
    const seen = new Set();
    /** @param {any} node */
    const card = node => {
      const fresh = !seen.has(node.key);
      const probs = F.nodeProblems(S, node), cat = F.CATS[node.cat], sel = ui.selected() === node.key, bad = probs.length > 0;
      const chip = el("span", { class: "fln-chip " + (bad ? "bad" : "ok") }, icon(bad ? "warn" : "ok", 12), el("span", { text: bad ? "Needs attention" : "Ready" }));
      const c = el("div", { class: "fln" + (sel ? " sel" : "") + (bad ? " bad" : "") + (fresh ? " fl-new" : ""), tabindex: 0, role: "button", "data-key": node.key, "aria-pressed": sel,
        "aria-label": `${node.title}. ${cat.word}. ${F.summary(S, node)}. ${bad ? probs[0].text : "Ready"}`, style: `left:${node.x}px;top:${node.y}px;--cc:${cat.color}`,
        onclick: () => ui.select(node.key),
        onkeydown: (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); ui.select(node.key); } else if ((e.key === "Delete" || e.key === "Backspace") && !node.locked && node.kind !== "input") { e.preventDefault(); ui.remove(node); } } },
      el("span", { class: "fln-ic" }, icon(cat.icon, 20)), el("span", { class: "fln-tx" }, el("span", { class: "fln-t", text: node.title }), el("span", { class: "fln-s", text: F.summary(S, node) })), chip);
      if (node.kind !== "input") c.append(el("button", { type: "button", class: "fln-next", "aria-label": `Add a step after ${node.title}`, title: "Add a step after this one", onclick: (/** @type {Event} */ e) => { e.stopPropagation(); ui.next(node, /** @type {HTMLElement} */ (e.currentTarget)); } }, icon("plus", 14)));
      return c;
    };
    /** @param {{nodes: any[], edges: [string, string][]}} model */
    const draw = model => {
      const hadFocus = root.contains(document.activeElement) && document.activeElement instanceof HTMLElement ? document.activeElement.dataset.key : "";
      Object.assign(size, F.layout(model.nodes, model.edges));
      stage.style.width = size.width + "px"; stage.style.height = size.height + "px"; svg.setAttribute("width", String(size.width)); svg.setAttribute("height", String(size.height));
      stage.querySelectorAll(".fln").forEach(n => n.remove()); svg.replaceChildren();
      const by = new Map(model.nodes.map(n => [n.key, n])), sel = ui.selected();
      for (const [a, b] of model.edges) {
        const p = by.get(a), q = by.get(b), x1 = p.x + CW, y1 = p.y + CH / 2, x2 = q.x, y2 = q.y + CH / 2, dx = Math.max(40, (x2 - x1) / 2), path = document.createElementNS(ns, "path");
        path.setAttribute("d", `M${x1} ${y1} C${x1 + dx} ${y1} ${x2 - dx} ${y2} ${x2} ${y2}`); path.setAttribute("class", "fle" + (a === sel || b === sel ? " hot" : "")); svg.append(path);
      }
      const order = [...model.nodes].sort((m, n) => m.x - n.x || m.y - n.y);
      for (const n of order) stage.append(card(n));
      seen.clear(); for (const n of order) seen.add(n.key);
      if (hadFocus) { const t = stage.querySelector(`[data-key="${CSS.escape(hadFocus)}"]`); if (t instanceof HTMLElement) t.focus({ preventScroll: true }); }
    };
    /** moves the view so the card is in the middle @param {string} key */
    const reveal = key => {
      const n = stage.querySelector(`[data-key="${CSS.escape(key)}"]`);
      if (!(n instanceof HTMLElement)) return;
      v.x = view.clientWidth / 2 - (n.offsetLeft + CW / 2) * v.k; v.y = view.clientHeight / 2 - (n.offsetTop + CH / 2) * v.k; apply(true); n.focus({ preventScroll: true });
    };
    return { root, draw, fit, view, reveal };
  };
})();
