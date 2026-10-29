// @ts-check
// Flow (administrator mode): what a card of the canvas says about its steps while the plan runs (a line with a mark and the words, a thin progress bar) and the table «Files» of the panel
// of a step (one row per file: state, share, time, memory). Both read F.liveView and repaint in place when it changes, so the focus and the selection stay where they are. The mark and the
// words carry the state; the colour only adds to them. English only. Docs: docs/agenti/flow-live.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const NS = "http://www.w3.org/2000/svg";
  /** one mark for each look, so that the state does not hang on the colour */
  const MARKS = /** @type {Record<string, string>} */ ({
    wait: "M12 7v5l3 2M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z", run: "M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5", ok: "M5 12l5 5 9-10", again: "M4 12a8 8 0 0 1 14-5M20 4v4h-4M20 12a8 8 0 0 1-14 5M4 20v-4h4",
    stale: "M12 8v5M12 16.5v.01M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z", bad: "M15 9l-6 6M9 9l6 6M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z", lock: "M7 11V8a5 5 0 0 1 10 0v3M6 11h12v9H6z",
    pause: "M9 5v14M15 5v14", skip: "M6 12h12",
  });
  const mark = (/** @type {string} */ name) => {
    const s = document.createElementNS(NS, "svg"), p = document.createElementNS(NS, "path");
    for (const [k, v] of Object.entries({ viewBox: "0 0 24 24", width: 14, height: 14, fill: "none", stroke: "currentColor", "stroke-width": 2, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true" })) s.setAttribute(k, String(v));
    p.setAttribute("d", MARKS[name] || MARKS.wait); s.append(p);
    return s;
  };
  const ids = (/** @type {Element} */ n) => String(/** @type {HTMLElement} */ (n).dataset.ids || "").split(" ").filter(Boolean);

  /** the line of a card: mark, words and bar. Empty (and hidden) for a card that is not a step @param {any} node */
  const build = node => {
    const { el } = F, list = node.kind === "input" ? [] : node.ids || [];
    return el("span", { class: "fln-live", "data-ids": list.join(" "), hidden: !list.length },
      el("span", { class: "fln-lm" }), el("span", { class: "fln-lw", text: "" }),
      el("span", { class: "fln-bar", role: "progressbar", "aria-valuemin": 0, "aria-valuemax": 100, "aria-label": `Progress of ${node.title}`, hidden: true }, el("i", {})));
  };

  /** @param {HTMLElement} line */
  const paintLine = line => {
    const e = F.liveView.of(ids(line)), card = /** @type {HTMLElement | null} */ (line.closest(".fln"));
    if (!e) { line.dataset.state = ""; line.querySelector(".fln-lw")?.replaceChildren(); line.querySelector(".fln-lm")?.replaceChildren(); /** @type {HTMLElement} */ (line.querySelector(".fln-bar")).hidden = true; if (card) { card.removeAttribute("data-live"); if (card.dataset.lbl) card.setAttribute("aria-label", card.dataset.lbl); } return; }
    const v = F.liveView.look(e), note = F.live && F.live.centroidNote ? F.live.centroidNote(ids(line)) : "";
    if (note && (v.state === "done" || v.state === "reused")) { v.title = `${note} · ${v.title}`; v.word = note.replace(/ \(.*\)$/, ""); }
    else if (note && v.state === "blocked") v.title = `${v.title} (${note})`;
    const word = line.querySelector(".fln-lw"), bar = /** @type {HTMLElement} */ (line.querySelector(".fln-bar"));
    line.dataset.state = v.state; line.dataset.tone = v.tone;
    if (word && word.textContent !== v.word) { word.textContent = v.word; word.setAttribute("title", v.title); }
    const m = line.querySelector(".fln-lm");
    if (m && m.dataset.mark !== v.icon) { m.dataset.mark = v.icon; m.replaceChildren(mark(v.icon)); }
    bar.hidden = v.state !== "running";
    if (!bar.hidden) {
      if (v.frac === null) { bar.removeAttribute("aria-valuenow"); bar.classList.add("indet"); bar.style.removeProperty("--p"); }
      else { bar.classList.remove("indet"); bar.setAttribute("aria-valuenow", String(Math.round(v.frac * 100))); bar.style.setProperty("--p", Math.round(v.frac * 100) + "%"); }
    }
    if (card) { card.dataset.live = v.state; if (card.dataset.lbl) card.setAttribute("aria-label", `${card.dataset.lbl}. ${v.title}`); }
  };

  /** the table «Files» of the steps `list`: a row per job that has a file; hidden while there is none @param {any} node */
  const files = node => {
    const { el } = F, list = node.kind === "input" ? [] : node.ids || [];
    if (!list.length) return null;
    const tbl = el("table", { class: "flx-table" }, el("caption", { text: "Files" }), el("thead", {}, el("tr", {}, ...["File", "Mode", "State", "Share", "Time", "Memory"].map(h => el("th", { scope: "col", text: h })))), el("tbody", {}));
    return el("div", { class: "fln-files", "data-ids": list.join(" "), hidden: true }, el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": `Files of ${node.title}` }, tbl));
  };

  /** @param {HTMLElement} box */
  const paintFiles = box => {
    const { el } = F, rows = [];
    for (const id of ids(box)) for (const j of F.liveView.jobsOf(id)) if (j.item) rows.push({ id, j, v: F.liveView.look(j) });
    box.hidden = !rows.length;
    const body = box.querySelector("tbody");
    if (!body) return;
    body.replaceChildren(...rows.map(({ id, j, v }) => {
      const mb = F.liveView.memoryOf(id), share = v.state === "running" ? (v.frac === null ? "…" : Math.round(v.frac * 100) + " %") : v.state === "done" || v.state === "reused" ? "100 %" : "–";
      const mode = F.live && F.live.modeOf ? F.live.modeOf(String(j.item)) : "", state = v.word.replace(/^done in .*/, "done");
      return el("tr", {}, el("th", { scope: "row", text: String(j.item) }), el("td", { text: mode || "–" }), el("td", {}, el("span", { class: "fln-lm", "aria-hidden": "true" }, mark(v.icon)), " ", mode === "centroid" && F.live.isCentroidStep(id) ? "already centroided" : state), el("td", { text: share }),
        el("td", { text: j.ms ? F.liveView.seconds(j.ms) : "–" }), el("td", { text: mb ? `≈ ${Math.ceil(mb)} MB` : "–" }));
    }));
  };

  let queued = 0;
  /** repaints every line and table that is on the page, once per frame */
  const paint = () => {
    if (queued) return;
    queued = requestAnimationFrame(() => { queued = 0; document.querySelectorAll(".fln-live").forEach(n => paintLine(/** @type {HTMLElement} */ (n))); document.querySelectorAll(".fln-files").forEach(n => paintFiles(/** @type {HTMLElement} */ (n))); });
  };
  document.addEventListener("flow:live-view", paint);

  F.liveCard = { build, files, paint };
})();
