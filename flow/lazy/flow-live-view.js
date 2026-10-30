// @ts-check
// Flow (administrator mode): what the live run says about each card. Keeps the last event flowlive/1 of every node and job (the web client and the desktop client both feed it), and gives
// the words of a state: «waiting», «3 of 6 files · 42 %», «done in 12 s», «out of date», «error: …». Nothing is computed here: fractions, counts and times come in the events; this file only
// picks the worst state of a card made of several steps and formats numbers. Never a time left. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  /** the worst state first: the order of the scheduler for a card */
  const RANK = ["error", "refused", "blocked", "skipped", "running", "stale", "paused", "ready", "cancelled", "waiting", "reused", "done"];
  const TERMINAL = new Set(["done", "reused", "error", "refused", "blocked", "skipped", "cancelled"]);
  /** @type {Map<string, any>} */ const nodes = new Map();
  /** @type {Map<string, Map<string, any>>} */ const jobs = new Map();
  /** @type {Map<string, number>} */ const memory = new Map();
  let rev = 0;

  /** seconds as a person says them: «0.9 s», «12 s», «1 min 5 s» @param {number} ms */
  const seconds = ms => {
    const s = ms / 1000;
    if (s < 0.1) return "under 0.1 s";
    if (s < 10) return s.toFixed(1) + " s";
    if (s < 60) return Math.round(s) + " s";
    return `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
  };
  const percent = (/** @type {number} */ f) => Math.min(100, Math.max(0, Math.round(f * 100)));

  /** the reason of an error or a refusal, in the words of the page @param {any} e */
  const why = e => (e && e.error && e.error.error_key ? F.message({ key: e.error.error_key, params: e.error.params || {} }) : "");

  /** the words of the card «Centroid spectra» once its files are known: «7 files ready (already centroided)», «2 of 3 files ready (1 centroided earlier, 1 already centroided)»; «Reading 7 files…» while Data has not classed them @param {number} n files @param {number} known files whose mode is known @param {number} done profile files with a centroid index @param {number} already files that were centroid */
  const centroidWords = (n, known, done, already) => {
    const files = (/** @type {number} */ k) => `${k} file${k === 1 ? "" : "s"}`;
    if (!n) return "";
    if (!known) return `Reading ${files(n)}…`;
    const ready = done + already, what = !done ? "already centroided" : !already ? "centroided earlier" : `${done} centroided earlier, ${already} already centroided`;
    if (!ready) return `0 of ${files(n)} ready`;
    return `${ready === n ? files(n) : `${ready} of ${files(n)}`} ready (${what})`;
  };

  /** the mark, the words and the look (ok, run, warn, bad, idle) of one node or job event; `title` is the full text for the tooltip @param {any} e @returns {{state: string, icon: string, word: string, title: string, tone: string, frac: number | null}} */
  const look = e => {
    const st = e.state, frac = typeof e.frac === "number" ? e.frac : null, many = (e.di || 0) > 1, files = many ? `${e.parte || 0} of ${e.di} files` : "";
    // `word` is short enough for one line of a card; `title` (tooltip, aria-label) is the whole sentence
    const mk = (/** @type {string} */ icon, /** @type {string} */ word, /** @type {string} */ tone, /** @type {string} */ title = word) => ({ state: st, icon, word, title, tone, frac });
    switch (st) {
      case "running": return mk("run", frac === null ? files || "working" : [files, percent(frac) + " %"].filter(Boolean).join(" · "), "run");
      case "done": return mk("ok", e.ms ? "done in " + seconds(e.ms) : "done", "ok");
      case "reused": return mk("again", "Up to date (nothing changed)", "ok");
      case "stale": return mk("stale", "Settings changed: press Run", "warn", "Settings changed: runs again on Run");
      case "error": return mk("bad", "This step failed", "bad", why(e) || "This step failed");
      case "refused": return mk("bad", "This step was refused", "bad", why(e) || "This step was refused");
      case "blocked": return e.blocked_by === "permission.memory" ? mk("lock", "Allow more memory: Run now", "warn", "Large files: allow more memory with Run now")
        : e.blocked_by === "input.insilico" ? mk("lock", "Needs the candidate structures", "warn", "Download the in-silico candidate package or choose a list of candidate structures, then run again")
        : e.blocked_by === "platform.computer" ? mk("lock", "Needs the desktop app", "warn", "This step needs the desktop app (high-resolution profile mzML, or centroid parameters set by hand). Use the app.") : mk("lock", "Fix the plan first", "warn", "Fix the plan first (see Issues)");
      case "skipped": return mk("skip", "skipped", "idle");
      case "paused": return mk("pause", "paused", "idle");
      case "cancelled": return mk("skip", "cancelled", "idle");
      default: return mk("wait", "waiting", "idle");
    }
  };

  /** one event; an event of an older revision of the plan is dropped @param {any} e @returns {{instance: string, from: string, to: string} | null} the change of state of a node, if it is one */
  const take = e => {
    if (!e || e.format !== "flowlive/1" || typeof e.rev !== "number" || e.rev < rev) return null;
    rev = e.rev;
    if (e.kind === "job") {
      const m = jobs.get(e.instance) || new Map();
      jobs.set(e.instance, m); m.set(String(e.item ?? ""), e);
      return null;
    }
    if (e.kind !== "node") return null;
    const was = nodes.get(e.instance);
    nodes.set(e.instance, e);
    return !was || was.state !== e.state ? { instance: e.instance, from: was ? was.state : "", to: e.state } : null;
  };

  F.liveView = {
    TERMINAL,
    look,
    centroidWords,
    seconds,
    /** the events of a read, in order @param {any[]} list */
    ingest(list) {
      const changes = list.map(take).filter(Boolean);
      if (list.length) document.dispatchEvent(new CustomEvent("flow:live-view", { detail: { changes } }));
    },
    /** a new scheduler: everything said before is forgotten */
    clear() { nodes.clear(); jobs.clear(); memory.clear(); rev = 0; document.dispatchEvent(new CustomEvent("flow:live-view", { detail: { changes: [] } })); },
    /** the memory a step asks for, in MB, when its runner knows @param {string} instance @param {number | null} mb */
    memory(instance, mb) { if (mb) memory.set(instance, mb); },
    memoryOf: (/** @type {string} */ instance) => memory.get(instance) || null,
    node: (/** @type {string} */ instance) => nodes.get(instance) || null,
    /** the jobs of a step, in the order they came @param {string} instance @returns {any[]} */
    jobsOf: instance => [...(jobs.get(instance) || new Map()).values()],
    /** what a card made of the steps `ids` shows: the node event of the worst state @param {string[]} ids @returns {any | null} */
    of(ids) {
      let best = null;
      for (const id of ids) {
        const n = nodes.get(id);
        if (n && (!best || RANK.indexOf(n.state) < RANK.indexOf(best.state))) best = n;
      }
      return best;
    },
    /** true when a step waits for the user to agree to more memory */
    askingMemory: () => [...nodes.values()].some(n => n.state === "blocked" && n.blocked_by === "permission.memory"),
    /** true when no node is left to run or to wait for: what the run waits for is outside (a click, a file) */
    settled: () => nodes.size > 0 && [...nodes.values()].every(n => TERMINAL.has(n.state)),
  };
})();
