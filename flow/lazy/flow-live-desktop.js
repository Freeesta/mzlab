// @ts-check
// Flow (administrator mode): the live run of «This computer», only in the desktop app. Same model as the browser (flow-live.js): the plan goes to the native engine (routes flow/live/* of
// mzlab://), the events flowlive/1 it answers are read every 300-1500 ms into F.liveView, so the cards say what each step does, and a change of the plan is sent as an update while the
// run is on. Run starts the session if there is none, waits until nothing is left to do and asks `finish` for the prov/1. English only. Docs: docs/agenti/flow-live.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const LIVE_KEY = "mzlab.flow.live";
  let following = true, held = false;
  try { following = localStorage.getItem(LIVE_KEY) !== "0"; } catch (_) { /* private mode: Live stays on */ }
  /** @type {{ S: any, names: string[], excluded: string[], warns: string[], since: number, sent: string } | null} */ let session = null;
  let timer = 0, settledAt = 0, changing = 0;

  const call = (/** @type {string} */ op, /** @type {any} */ body) => F.desktopPost("flow/live/" + op, body || {}).then((/** @type {string} */ t) => (t ? JSON.parse(t) : null));
  const said = () => document.dispatchEvent(new CustomEvent("flow:live-ctl"));

  /** the plan of the page with the files open in Data as its scan inputs, without the ones the app does not hold @param {any} S @param {{ excluded: string[] }} keep */
  const planOf = (S, keep) => {
    const { plan, names, warns } = F.runPlanFromData(S);
    for (const bad of keep.excluded) if (names.includes(bad)) F.removeInput(plan, bad);
    return { plan, names: names.filter((/** @type {string} */ n) => !keep.excluded.includes(n)), warns };
  };

  /** the events since the last read into the view; the poll slows down when nothing happens */
  const poll = async () => {
    if (!session) return;
    const list = await call("events", { since: session.since });
    for (const e of list) session.since = Math.max(session.since, e.seq);
    if (list.length) { F.liveView.ingest(list); settledAt = 0; } else if (!settledAt) settledAt = Date.now();
  };
  const loop = () => {
    clearTimeout(timer);
    if (!session) return;
    timer = window.setTimeout(async () => { try { await poll(); } catch (_) { /* the next read tries again */ } loop(); }, settledAt && Date.now() - settledAt > 3000 ? 1500 : 300);
  };

  /** sends the plan of S with `op` (start | update); a file the app does not hold is left out and the call goes on with the others (`keep.excluded` remembers them) @param {string} op @param {any} S @param {{ excluded: string[], sent?: string }} keep */
  const send = async (op, S, keep) => {
    for (;;) {
      const { plan, names, warns } = planOf(S, keep), text = F.exportText(plan);
      if (op === "update" && text === keep.sent) return { text, names, warns, same: true };
      try { await call(op, { plan: text, files: S.samplesJson || {} }); return { text, names, warns, same: false }; } catch (e) {
        const x = /** @type {any} */ (e), bad = x && x.key === "err.flow.input.missing" ? x.params.input : null;
        if (!bad || !names.includes(bad) || names.length < 2) throw e;
        keep.excluded.push(bad);
      }
    }
  };

  /** starts the session with the plan of S @param {any} S */
  const start = async S => {
    const keep = { excluded: /** @type {string[]} */ ([]) }, sent = await send("start", S, keep);
    F.liveView.clear();
    session = { S, names: sent.names, excluded: keep.excluded, warns: sent.warns, since: 0, sent: sent.text };
    if (!following || held) await call("pause");
    loop(); said();
  };

  /** the plan of S sent as a change of the run in progress */
  const update = async (/** @type {any} */ S) => {
    if (!session) return;
    const sent = await send("update", S, session);
    session.warns = sent.warns; session.S = S;
    if (sent.same) return;
    session.sent = sent.text; settledAt = 0; loop();
  };

  /** the scheduler starts jobs only while the plan is followed and the user has not paused */
  const hold = async () => { if (session) await call(following && !held ? "resume" : "pause"); said(); };
  const setHeld = async (/** @type {boolean} */ on) => { held = on; await hold(); };
  const setLive = async (/** @type {boolean} */ on) => {
    following = on;
    try { localStorage.setItem(LIVE_KEY, on ? "1" : "0"); } catch (_) { /* private mode */ }
    if (on && session) await update(session.S);
    await hold();
  };

  /** waits until no step is left to run: every node is done or ended, or nothing runs and nothing changes between two looks (what is left waits for something outside) */
  const idle = async () => {
    let before = "";
    for (;;) {
      await poll();
      const nodes = /** @type {any[]} */ (await call("nodes")), seen = JSON.stringify(nodes.map(n => n.state));
      if (nodes.every(n => F.liveView.TERMINAL.has(n.state))) return nodes;
      if (!nodes.some(n => n.state === "running" || n.state === "ready" || n.state === "stale") && seen === before) return nodes;
      before = seen;
      await new Promise(r => setTimeout(r, 300));
    }
  };

  /** runs the plan of S to its end and gives { prov, dir, excluded, warns }; rejects with {key, params} @param {any} S */
  const run = async S => {
    if (!session) await start(S); else await update(S);
    if (!following || held) { following = true; held = false; await hold(); }
    const nodes = await idle(), bad = nodes.find(n => n.state === "error" || n.state === "refused");
    if (bad) throw { key: bad.error ? bad.error.error_key : "err.flow.run", params: bad.error ? bad.error.params : {} };
    const out = await call("finish", S.llmRef ? { llm: S.llmRef } : {}), s = /** @type {NonNullable<typeof session>} */ (session);
    return { ...out, excluded: s.excluded, warns: s.warns };
  };

  document.addEventListener("flow:changed", () => {
    if (!session || !following) return;
    const S = /** @type {any} */ (document.querySelector(".flx-list"))?.__flow;
    clearTimeout(changing);
    changing = window.setTimeout(() => { if (S && session) update(S).catch(() => { /* the next change tries again */ }); }, 300);
  });

  F.desktopLive = { run, start, update, setLive, setHeld, following: () => following, held: () => held, active: () => !!session, drop: async () => { session = null; clearTimeout(timer); await call("drop").catch(() => {}); said(); } };
})();
