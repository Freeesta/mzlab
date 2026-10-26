// @ts-check
// Flow (administrator mode): the readers of the assistant on the panel. For each batch of readers named by the router it shows one card with a line per reader (queued, running, done, discarded,
// error, skipped, cancelled) and a short outcome. Only display: the runner is llm/llm-agente.js (`host.readerStage`, `host.reader`), the checks are Rust. Loaded by flow-shell.js with flow-llm.js. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.flx .flx-llm-readers{border-left-color:var(--muted)}.flx .flx-llm-readers ul{list-style:none;padding-left:0;margin:0;display:flex;flex-direction:column;gap:4px}
.flx .flx-llm-readers li{display:flex;flex-wrap:wrap;gap:6px;align-items:baseline}.flx .flx-llm-readers .name{font-weight:600}
.flx .flx-llm-readers .state{border:1px solid var(--line);border-radius:6px;padding:0 6px;font-size:12px;color:var(--muted)}
.flx .flx-llm-readers li[data-state=running] .state{border-color:var(--accent);color:var(--accent)}.flx .flx-llm-readers li[data-state=done] .state{border-color:var(--ok);color:var(--ok)}
.flx .flx-llm-readers li[data-state=error] .state,.flx .flx-llm-readers li[data-state=discarded] .state{border-color:var(--bad);color:var(--bad)}
.flx .flx-llm-readers .out{flex-basis:100%;margin:0;color:var(--muted);font-size:12px;overflow-wrap:anywhere}
@media (prefers-reduced-motion:reduce){.flx .flx-llm-readers *{transition:none!important;animation:none!important}}
`;
  /** the readers of the router (`router::reader_of`) */
  const NAMES = /** @type {Record<string, string>} */ ({ metadati: "Metadata", cromatografia: "Chromatograms", parametri: "Parameters", interprete: "Table check", grafici: "Charts" });
  /** the state shown for each status of the runner: [key, word, mark] (the word and the mark say it without the colour) */
  const STATES = /** @type {Record<string, [string, string, string]>} */ ({
    queued: ["queued", "Queued", "…"], running: ["running", "Running", "▶"], accepted: ["done", "Done", "✓"], dropped: ["discarded", "Discarded", "✗"],
    failed: ["error", "Error", "✗"], skipped: ["skipped", "Skipped", "–"], cancelled: ["cancelled", "Cancelled", "–"],
  });
  const LIMIT = 240;

  /** a short line for a summary (an object of the reader) or the problems of a discarded one @param {any} info */
  const outcome = info => {
    if (info.status === "accepted") {
      const s = info.summary; const t = typeof s === "string" ? s : JSON.stringify(s);
      return t && t !== "null" ? (t.length > LIMIT ? t.slice(0, LIMIT) + "…" : t) : "";
    }
    if (info.status === "skipped") return "No data to read.";
    if (!Array.isArray(info.problems)) return "";
    return info.problems.slice(0, 3).map((/** @type {any} */ p) => (F.message({ key: p.key || p.error_key, params: p.params || {} }) || p.key || p.error_key || "") + "").filter(Boolean).join(" ");
  };

  /** the display of the readers; `put` adds a card to the log @param {(card: HTMLElement) => void} put */
  const make = put => {
    const { el } = F;
    if (!document.getElementById("flx-llm-readers-css")) { const css = document.createElement("style"); css.id = "flx-llm-readers-css"; css.textContent = CSS; document.head.append(css); }
    /** @type {{list: HTMLElement, rows: Record<string, {li: HTMLElement, state: HTMLElement, out: HTMLElement}>} | null} */ let batch = null;
    let pending = 0;
    /** @param {string} role @param {string} status @param {string} [text] */
    const paint = (role, status, text) => {
      const row = batch && batch.rows[role]; if (!row) return;
      const [key, word, mark] = STATES[status] || ["error", status, "?"];
      row.li.dataset.state = key; row.state.textContent = `${mark} ${word}`; row.out.hidden = !text; row.out.textContent = text || "";
    };
    /** a reader waits its turn: the first one of a batch opens the card @param {string} role */
    const queued = role => {
      if (!batch || pending === 0) {
        const list = el("ul", { "aria-label": "Readers of the assistant" });
        put(el("div", { class: "flx-llm-card flx-llm-readers", role: "group", "aria-label": "Readers" }, el("strong", { text: "Readers" }), list));
        batch = { list, rows: {} }; pending = 0;
      }
      if (batch.rows[role]) return;
      const state = el("span", { class: "state" }), out = el("p", { class: "out", hidden: true });
      const li = el("li", {}, el("span", { class: "name", text: NAMES[role] || role }), state, out);
      batch.list.append(li); batch.rows[role] = { li, state, out }; pending++;
      paint(role, "queued");
    };
    /** a reader changes stage before its verdict: `running`, or `cancelled` when the turn was stopped @param {string} role @param {string} stage */
    const stage = (role, stage) => {
      if (!batch || !batch.rows[role]) queued(role);
      paint(role, stage);
      if (stage === "cancelled") pending = Math.max(0, pending - 1);
    };
    /** the verdict of a reader: {role, status: accepted|dropped|failed|skipped, problems, summary} @param {any} info */
    const verdict = info => {
      if (!batch || !batch.rows[info.role]) queued(info.role);
      paint(info.role, info.status, outcome(info));
      pending = Math.max(0, pending - 1);
    };
    return { queued, stage, verdict };
  };
  F.llmReaders = { make, NAMES };
})();
