// @ts-check
// Flow (administrator mode): the live run of «This browser». While the plan is built, the jobs the browser can do start by themselves, file by file, and only the ones a change touches are made again.
// The choice of the jobs is the scheduler of the engine (api/flow/sched/<op>: plan_key, states, `next`); this file sends it the plan on every change (debounce 300 ms), runs the jobs it hands out
// (flow-live-jobs.js), reports started / progress / finished, and derives the events `flow:web-step` of flow-web-run.js from the events of the scheduler. The outputs stay in OPFS, never in
// the scheduler. Loaded late (LATE of flow-tool.js), never on the way to the first draw. English only. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const TERMINAL = new Set(["done", "reused", "error", "refused", "blocked", "skipped", "cancelled"]);
  const WEB = /** @type {Record<string, string>} */ ({ waiting: "waiting", ready: "waiting", paused: "waiting", stale: "waiting", running: "running", done: "done", reused: "reused", error: "error", refused: "refused", blocked: "skipped", skipped: "skipped", cancelled: "skipped" });
  const kit = () => F.runners.web.kit;
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** true when this browser does the step (the MSn search in a library is for the `tree` algorithm only) */
  const here = (/** @type {any} */ inst) => kit().KNOWN.includes(base(inst)) && (base(inst) !== "spectra.match" || inst.algorithm === "tree");
  const budgetMb = () => Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0;
  /** @type {any} */ let x = null;
  // the two switches of the bar: Live (the plan is followed and its jobs start by themselves, on by default) and the user's Pause
  const LIVE_KEY = "mzlab.flow.live";
  let following = true, held = false;
  try { following = localStorage.getItem(LIVE_KEY) !== "0"; } catch (_) { /* private mode: Live stays on */ }

  /** one live session: the plan sent, the items (one per .mzidx), the keys and the states the scheduler reported @param {any} S */
  const session = S => ({
    S, plan: S.plan, items: /** @type {any[]} */ ([]), ok: false, q: Promise.resolve(), seq: 0, sent: "", keys: new Map(), nodes: new Map(), info: new Map(), errors: new Map(), last: new Map(),
    running: new Map(), observing: new Map(), waitingInput: new Set(), memAsk: new Map(), allowed: new Set(), loading: new Map(), listeners: new Set(), log: /** @type {any[]} */ ([]), logFrom: 0,
    waitingProfile: new Set(), files: new Map(), pooled: 0, peak: 0, pumping: /** @type {any} */ (null), again: false, waiters: /** @type {(() => void)[]} */ ([]), started: 0, tick: new Map(), registry: null, drainSoon: 0,
  });

  /** @param {any} x @param {string} op @param {any} [body] one call to the scheduler; the calls go one after the other, and a change is followed by a read of the events */
  const call = (x, op, body) => {
    const go = x.q.then(async () => {
      const r = await fetch("api/flow/sched/" + op, { method: "POST", body: JSON.stringify(body || {}) }), t = await r.text(), v = t ? JSON.parse(t) : null;
      if (v && typeof v === "object" && !Array.isArray(v) && v.error_key) throw Object.assign(new Error(v.error_key), { key: v.error_key, params: v.params || {} });
      return v;
    });
    x.q = go.catch(() => {});
    if (!["events", "nodes", "jobs", "next", "rev"].includes(op)) { clearTimeout(x.drainSoon); x.drainSoon = setTimeout(() => drain(x).catch(() => {}), 0); }
    return go;
  };

  const jk = (/** @type {any} */ j) => `${j.instance}|${j.item ?? ""}`;
  const jobOf = (/** @type {string} */ k) => { const at = k.indexOf("|"), item = k.slice(at + 1); return { instance: k.slice(0, at), item: item === "" ? null : item }; };
  const instOf = (/** @type {any} */ x, /** @type {string} */ id) => x.plan.steps.find((/** @type {any} */ s) => s.id === id);

  /** the event `flow:web-step` of the runner, kept in the log of the session too @param {any} x @param {any} ev */
  const emitWeb = (x, ev) => {
    x.log.push(ev);
    for (const f of x.listeners) f(ev);
    document.dispatchEvent(new CustomEvent("flow:web-step", { detail: ev }));
  };

  /** an event of the scheduler for a card becomes the event of the runner of the page: the same words, the memory the step asks for and the file k of n @param {any} x @param {any} n */
  const fromNode = (x, n) => {
    const inst = instOf(x, n.instance);
    if (!inst) return;
    const known = here(inst), memory = n.state === "blocked" && [...x.memAsk.values()].some((/** @type {any} */ a) => a.spec.instance === n.instance);
    const stato = memory ? "blocked" : WEB[n.state] || "waiting", info = x.info.get(n.instance) || {};
    const ev = { passo: n.instance, stato, via: known ? "browser" : "computer", parte: n.parte || 0, di: n.di || 0, mb: base(inst) === "features.detect" ? x.peak || null : info.mb ?? null, modo: info.modo ?? null };
    const was = x.last.get(n.instance);
    if (was && was.stato === ev.stato && was.parte === ev.parte && was.di === ev.di && was.mb === ev.mb) return;
    // the last file counted before the end, as the runner of a plan has always reported
    if (stato === "done" && was && was.stato === "running" && was.parte < ev.di) { const last = { ...was, parte: ev.di }; x.last.set(n.instance, last); emitWeb(x, last); }
    x.last.set(n.instance, ev);
    if (ev.mb) view()?.memory(n.instance, ev.mb);
    emitWeb(x, ev);
  };

  /** @param {any} x @param {any} e one event flowlive/1 */
  const onEvent = (x, e) => {
    if (e.kind === "job" && e.key) x.keys.set(jk(e), e.key);
    if (e.kind === "job" && (e.state === "done" || e.state === "reused" || e.state === "error")) x.allowed.delete(jk(e));
    if (e.kind === "node") { x.nodes.set(e.instance, e); fromNode(x, e); }
  };
  const view = () => F.liveView;
  const wake = (/** @type {any} */ x) => { const w = x.waiters.splice(0); for (const f of w) f(); };
  /** the events of the scheduler since the last read @param {any} x */
  const drain = async x => {
    const list = await call(x, "events", { since: x.seq });
    for (const e of list) { x.seq = Math.max(x.seq, e.seq); onEvent(x, e); }
    if (list.length) { view()?.ingest(list); wake(x); }
  };

  /** the plan the scheduler works on: the plan of the page with the placeholder scan input replaced by one input per .mzidx of OPFS (or loading in Data), as `runPlanFromData` does for the app;
   * `ids` = what identifies each input (size and date of its index) @param {any} S @param {any} x */
  const build = async (S, x) => {
    const files = await kit().listIdx(), plan = JSON.parse(JSON.stringify(S.plan)), open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.map((/** @type {any} */ f) => String(f && f.file || "").split("#")[0]) : [];
    const stemOf = (/** @type {string} */ n) => n.replace(/\.[^.]+$/, "");
    // `mode`: how Data classed the file (profile | centroid | mixed), the base of the choice of the centroid step; `cen`: the index of its centroids, when Data holds one
    const modeOf = (/** @type {string} */ n) => { const f = (typeof ST !== "undefined" && Array.isArray(ST.files) ? ST.files : []).find((/** @type {any} */ g) => g.name === n); return f && f.spectrum_mode ? String(f.spectrum_mode) : ""; };
    const items = files.map((/** @type {any} */ f) => { const stem = f.name.replace(/\.mzidx$/, ""), input = open.find((/** @type {string} */ n) => n && (n === stem || stemOf(n) === stem)) || stem + ".mzML"; return { input, stem, idx: f, cen: f.cen || null, mode: modeOf(input) }; });
    for (const stem of x.loading.keys()) if (!items.some((/** @type {any} */ i) => i.stem === stem)) items.push({ input: stem + ".mzML", stem, idx: null, cen: null, mode: modeOf(stem + ".mzML") });
    // an MSn step reads the infusion open in Data, not the first file of the series: its key follows that file
    const infusion = (() => { try { return F.runPlanFromData(S).names[0]; } catch (_) { return ""; } })(), msnSteps = F.webMsn ? F.webMsn.STEPS : [];
    const scan = plan.inputs.filter((/** @type {any} */ i) => i.type === "raw" || i.type === "mzml"), first = scan[0];
    if (items.length && first) {
      const refs = items.map((/** @type {any} */ i) => "file:" + i.input);
      plan.inputs.splice(plan.inputs.indexOf(first), 1, ...items.map((/** @type {any} */ i) => ({ name: i.input, sha256: "", type: first.type })));
      for (const s of plan.steps) for (const [port, v] of Object.entries(s.in || {})) {
        const list = [].concat(/** @type {any} */ (v)), at = list.indexOf("file:" + first.name);
        if (at < 0) continue;
        const many = (F.stepOf(S.reg, s) || { in: {} }).in[port]?.many;
        s.in[port] = many ? [...list.slice(0, at), ...refs, ...list.slice(at + 1)] : msnSteps.includes(base(s)) && refs.includes("file:" + infusion) ? "file:" + infusion : refs[0];
      }
    }
    return { plan, items, ids: Object.fromEntries(items.map((/** @type {any} */ i) => [i.input, i.idx ? `${i.idx.key}:${i.mode}` : "loading"])) };
  };

  /** the surface the executors use (flow-live-jobs.js) @param {any} x */
  const surface = x => {
    x.call = (/** @type {string} */ op, /** @type {any} */ body) => call(x, op, body);
    x.jk = jk;
    x.item = (/** @type {string | null} */ name) => x.items.find((/** @type {any} */ i) => i.input === name);
    x.inst = (/** @type {string} */ id) => instOf(x, id);
    // the index a file's peaks are found on: its centroids when the plan centroids and the file is in profile, else the index of Data
    x.profile = (/** @type {any} */ it) => !!it && (it.mode === "profile" || it.mode === "mixed");
    x.hasCentroid = () => x.plan.steps.some((/** @type {any} */ i) => base(i) === "spectra.centroid");
    x.source = (/** @type {any} */ it) => (it && x.profile(it) && x.hasCentroid() ? it.cen : it && it.idx);
    // a profile file with no centroid step and no «run on profile» written in the plan is not detected: the user chooses
    x.profileHeld = (/** @type {any} */ it) => x.profile(it) && !x.hasCentroid() && x.plan.profile !== "as_is";
    x.alignOf = (/** @type {any} */ inst) => {
      if (base(inst) !== "features.gapfill") return inst;
      const ref = [].concat(/** @type {any} */ (inst.in.table || []))[0];
      return instOf(x, String(ref).split(".")[0]) || inst;
    };
    // the detections an alignment reads, in the order of the items: [{ key, path }]
    x.upstream = (/** @type {any} */ al) => {
      const ids = [...new Set(Object.values(al.in || {}).flat().map(String).filter(r => !r.startsWith("file:")).map(r => r.split(".")[0]))], out = /** @type {any[]} */ ([]);
      for (const it of x.items) for (const id of ids) { const key = x.keys.get(`${id}|${it.input}`); const src = x.source(it); if (key && src) out.push({ key, path: src.path }); }
      return out;
    };
  };

  /** starts what the scheduler offers while the pool has a free worker; the instant jobs (reused, waiting for a file) do not take a place @param {any} x */
  const step = async x => {
    for (let guard = 0; guard < 64 && x.ok && x.items.length; guard++) {
      const p = await kit().getPool(), free = Math.max(0, p.limit - x.pooled);
      p.onJob = (/** @type {any} */ e) => progress(x, e.job, e.frac);
      p.onProgress = (/** @type {any} */ e) => { if (e.phase === "detect") x.peak = Math.max(x.peak, e.memMB || 0); };
      if (!free) return;
      const specs = await call(x, "next", { slots: free, mem_mb: budgetMb() || 1e9, mb_default: 0 });
      if (!specs.length) return;
      for (const spec of specs) {
        const inst = instOf(x, spec.instance), exec = inst && F.liveJobs[base(inst)], key = jk(spec);
        if (x.running.has(key)) continue;
        if (!exec) { await call(x, "block", { job: { instance: spec.instance, item: spec.item }, by: "platform.computer" }); continue; }
        let r = /** @type {any} */ ({});
        try { r = await exec(x, spec, inst); } catch (e) {
          const err = /** @type {any} */ (e);
          x.errors.set(inst.id, err);
          await call(x, "started", { job: { instance: spec.instance, item: spec.item } }).catch(() => {});
          await call(x, "finished", { job: { instance: spec.instance, item: spec.item }, result: { error_key: err.key || "error.featWorker", params: F.liveJobs.strings(err.params) } }).catch(() => {});
        }
        if (r.done) {
          x.pooled++;
          x.running.set(key, r.done);
          r.done.catch(() => {}).then(() => { x.pooled--; x.running.delete(key); return drain(x); }).catch(() => {}).then(() => { wake(x); return pump(x); });
        }
      }
    }
  };
  /** @param {any} x */
  const pump = x => {
    if (x.pumping) { x.again = true; return x.pumping; }
    x.pumping = (async () => { try { do { x.again = false; await step(x); } while (x.again); } catch (_) { /* a call failed: the next change starts it again */ } finally { x.pumping = null; } })();
    return x.pumping;
  };

  /** progress of a job at most four times a second @param {any} x @param {string} key @param {number} frac */
  const progress = (x, key, frac) => {
    const now = performance.now();
    if (now - (x.tick.get(key) || 0) < 250 && frac < 1) return;
    x.tick.set(key, now);
    call(x, "progress", { job: jobOf(key), frac }).catch(() => {});
  };

  /** the index of a file Data was loading is now in OPFS: the job that watched it ends, the one that waited for the file goes on @param {any} x */
  const settleIndex = async x => {
    for (const [key, spec] of [...x.observing]) { const it = x.item(spec.item); if (it && it.idx) { x.observing.delete(key); await call(x, "finished", { job: jobOf(key), result: { ms: 0 } }); } }
    for (const key of [...x.waitingInput]) { const it = x.item(jobOf(key).item); const inst = instOf(x, jobOf(key).instance); if (it && inst && (base(inst) === "features.detect" ? x.source(it) : it.idx)) { x.waitingInput.delete(key); await call(x, "unblock", { job: jobOf(key) }); } }
    for (const key of [...x.waitingProfile]) { const it = x.item(jobOf(key).item); if (!it || !x.profileHeld(it)) { x.waitingProfile.delete(key); await call(x, "unblock", { job: jobOf(key) }); } }
    for (const [stem] of [...x.loading]) if (x.items.some((/** @type {any} */ i) => i.stem === stem && i.idx)) x.loading.delete(stem);
  };

  /** @param {any} x @param {{ plan: any, items: any[], ids: Record<string, string> }} b */
  const send = async (x, b) => {
    x.plan = b.plan; x.items = b.items;
    const text = JSON.stringify(F.exportPlan(b.plan));
    if (!x.started) {
      if (!x.registry) x.registry = JSON.parse(JSON.stringify(x.S.reg.steps));
      view()?.clear();
      await call(x, "new", { plan: JSON.parse(text), registry: x.registry, version: String(G.MZLAB_VERSION || ""), identities: b.ids });
      x.started = 1; x.sent = text; x.ok = true; x.ids = b.ids;
      if (!following || held) await call(x, "pause");
      for (const inst of b.plan.steps) emitWeb(x, { passo: inst.id, stato: "waiting", via: here(inst) ? "browser" : "computer", parte: 0, di: 0, mb: null, modo: null });
      return;
    }
    // a changed fingerprint (an index that was loading is now in OPFS) re-keys the jobs only at the next `update`, even when the plan text is the same
    let rekey = false;
    for (const [name, id] of Object.entries(b.ids)) if (x.ids[name] !== id) { await call(x, "set_identity", { name, text: id }); rekey = true; }
    x.ids = b.ids;
    if (text === x.sent && !rekey) return;
    x.sent = text;
    for (const c of await call(x, "update", { plan: JSON.parse(text) })) {
      const key = jk(c);
      x.memAsk.delete(key); x.observing.delete(key); x.waitingInput.delete(key); x.waitingProfile.delete(key);
      kit().poolIfAny()?.cancelJob(key);
    }
  };

  /** reads the plan again and tells the scheduler what changed @param {any} x */
  const replan = async x => {
    await send(x, await build(x.S, x));
    await settleIndex(x);
    await drain(x);
    await pump(x);
  };

  /** the live session of the plan of S: made on the first call; null when this browser cannot (a phone, no OPFS) @param {any} S @param {boolean} force */
  const attach = async (S, force) => {
    const R = F.runners.web;
    if (!R.available() || (!force && R.small())) return null;
    if (!x || x.S !== S) { x = session(S); surface(x); }
    const live = x;
    if (!live.started) await replan(live);
    return live;
  };

  // ------------------------------------------------------------------ run
  const outputsKept = async (/** @type {any} */ x) => {
    const p = await kit().getPool(), D = F.liveJobs.DIR;
    for (const j of await call(x, "jobs")) {
      if (j.state !== "done" && j.state !== "reused") continue;
      const inst = instOf(x, j.instance), key = x.keys.get(jk(j)), id = inst && base(inst);
      if (!key || (id !== "features.detect" && id !== "features.align")) continue;
      if (!await F.liveJobs.has(p, D + key + (id === "features.detect" ? ".mzfeatf" : ".mzfeat"))) return false;
    }
    return true;
  };
  /** a new scheduler for the same plan: what was kept in OPFS is found again by its plan_key, what is gone is made again @param {any} x */
  const restart = async x => {
    await call(x, "drop");
    Object.assign(x, { started: 0, seq: 0, ok: false, sent: "", pooled: 0, nodes: new Map() });
    view()?.clear();
    x.keys.clear(); x.memAsk.clear(); x.allowed.clear();
    await replan(x);
  };

  const refused = (/** @type {any} */ x, /** @type {string} */ instance) => x.errors.get(instance) || Object.assign(new Error("error.featMemory"), { key: "error.featMemory", params: {} });
  /** the page asks to use more memory for the blocked alignments; the user answers once, and only here (never in the middle of an edit) @param {any} x @param {any} o */
  const answerAsks = async (x, o) => {
    for (const [key, a] of [...x.memAsk]) {
      const high = kit().highMb(), budget = budgetMb(), job = jobOf(key);
      const ok = o.askMore ? await o.askMore({ fase: "align", stima_mb: a.est.mb, budget_mb: budget, libero_mb: budget, high_mb: high }) : false;
      x.memAsk.delete(key);
      if (ok) x.allowed.add(key);
      await call(x, "unblock", { job });
      if (!ok) {
        x.errors.set(a.spec.instance, a.err);
        await call(x, "started", { job });
        await call(x, "finished", { job, result: { error_key: "error.featMemory", params: F.liveJobs.strings(a.err.params) } });
      }
    }
    await drain(x); await pump(x);
  };

  /** runs the plan of S in the live session: starts it if it is not running, and resolves when every step this browser can do is done or reused. Resolves with { steps, table, llm }, rejects with {key, params}
   * @param {any} S @param {{ onEvent?: (e: any) => void, askMore?: (i: any) => Promise<boolean> }} [opts] */
  const run = async (S, opts = {}) => {
    const o = { askMore: G.mzlabAskMore, ...opts };
    const live = await attach(S, true);
    // Run is a request to get the plan done: it follows the plan again and goes on if it was paused
    if (live && live.ok && (!following || held)) { following = true; held = false; await hold(live); }
    if (!live || !live.ok) throw Object.assign(new Error("err.flow.input.missing"), { key: "err.flow.input.missing", params: {} });
    await replan(live);
    if (!live.items.length) throw Object.assign(new Error("err.flow.input.missing"), { key: "err.flow.input.missing", params: {} });
    if (!await outputsKept(live)) await restart(live);
    // Run is a new request: an MSn step that failed (the engine refused, the file was not open) is tried again, the other failures wait for a change of the plan
    for (const j of await call(live, "jobs")) { const inst = instOf(live, j.instance); if (inst && F.webMsn?.STEPS.includes(base(inst)) && ["error", "refused", "cancelled"].includes(j.state)) await call(live, "retry", { job: { instance: j.instance, item: j.item ?? null } }); }
    // what happened since the last run is reported again to this run; with nothing new, every finished step is reused
    const hist = live.log.slice(live.logFrom);
    if (o.onEvent) live.listeners.add(o.onEvent);
    try {
      if (hist.length) for (const ev of hist) { if (o.onEvent) o.onEvent(ev); document.dispatchEvent(new CustomEvent("flow:web-step", { detail: ev })); }
      else for (const n of await call(live, "nodes")) { const inst = instOf(live, n.instance); if (inst) emitWeb(live, { passo: n.instance, stato: n.state === "done" || n.state === "reused" ? "reused" : WEB[n.state] || "waiting", via: here(inst) ? "browser" : "computer", parte: n.di, di: n.di, mb: null, modo: "reused" }); }
      let before = "";
      for (;;) {
        await pump(live); await drain(live);
        const nodes = /** @type {any[]} */ (await call(live, "nodes")), pending = nodes.filter(n => !TERMINAL.has(n.state)), seen = JSON.stringify(nodes.map(n => n.state));
        if (live.memAsk.size && !live.running.size) { await answerAsks(live, o); continue; }
        if (!pending.length && !live.running.size) break;
        // nothing runs, nothing is watched and nothing changed since the last look: what is left waits for something outside (a file, a click)
        if (!live.running.size && !live.observing.size && !live.pumping && seen === before) break;
        before = seen;
        await new Promise(res => { live.waiters.push(res); setTimeout(res, 5000); });
      }
      const bad = /** @type {any[]} */ ((await call(live, "nodes"))).find(n => n.state === "error" || n.state === "refused");
      if (bad) throw refused(live, bad.instance);
    } finally { live.listeners.delete(o.onEvent); }
    live.logFrom = live.log.length;
    const al = S.plan.steps.find((/** @type {any} */ i) => base(i) === "features.align"), key = al && live.keys.get(`${al.id}|`);
    const table = key ? await (await kit().getPool()).fs.read(F.liveJobs.DIR + key + ".mzfeat") : null;
    // the record of the run (prov/1, corsa/1) for the plans of the base chain; a failure of it never fails the run
    const record = F.webProv ? await F.webProv.record(live, /** @type {any[]} */ (await call(live, "jobs"))).catch((/** @type {any} */ e) => { console.warn("record of the run:", e && e.key || e && e.message || e); return null; }) : null;
    // a run launched by the assistant keeps who launched it (llm of prov/1: S.llmRef = {log_sha256, turn, provider, model, level})
    return { record, steps: S.plan.steps.map((/** @type {any} */ i) => i.id), table, files: S.plan.steps.map((/** @type {any} */ i) => live.files.get(i.id)).filter(Boolean), llm: S.llmRef || null };
  };

  /** the estimate of the alignment when every detection of the live session is kept in OPFS, else null @param {any} S */
  const preflight = async S => {
    const live = x && x.S === S && x.ok ? x : null, al = live && S.plan.steps.find((/** @type {any} */ i) => base(i) === "features.align");
    if (!live || !al) return null;
    const up = live.upstream(al), p = await kit().getPool();
    if (!up.length || up.length < live.items.length) return null;
    const stored = [];
    for (const u of up) { const path = F.liveJobs.DIR + u.key + ".mzfeatf"; if (!await F.liveJobs.has(p, path)) return null; stored.push(await p.fs.read(path)); }
    return kit().stima(stored, kit().recipeOf(S.plan));
  };

  // ------------------------------------------------------------------ the controls of the bar
  const said = () => document.dispatchEvent(new CustomEvent("flow:live-ctl"));
  /** the scheduler starts jobs only while the plan is followed and the user has not paused @param {any} live */
  const hold = async live => {
    if (!live || !live.ok) return;
    const go = following && !held;
    await call(live, go ? "resume" : "pause");
    await drain(live);
    if (go) await pump(live);
    said();
  };
  const setHeld = async (/** @type {boolean} */ on) => { held = on; await hold(x); said(); };
  const setLive = async (/** @type {boolean} */ on) => {
    following = on;
    try { localStorage.setItem(LIVE_KEY, on ? "1" : "0"); } catch (_) { /* private mode */ }
    if (on) change();
    await hold(x); said();
  };
  /** the user agrees to more memory (the only question the live run asks) */
  const runNow = async () => { if (x && x.memAsk.size) await answerAsks(x, { askMore: G.mzlabAskMore }); };

  // ------------------------------------------------------------------ events of the page
  /** @type {any} */ let timer = 0;
  const change = () => {
    clearTimeout(timer);
    if (!following) return;
    timer = setTimeout(async () => {
      const S = /** @type {any} */ (document.querySelector(".flx-list"))?.__flow;
      if (!S) return;
      try { if (x && x.S === S) await replan(x); else await attach(S, false); } catch (_) { /* the plan cannot be scheduled yet: the next change tries again */ }
    }, 300);
  };
  document.addEventListener("flow:changed", change);
  // the index Data builds: the watching job shows its progress, and the file is found when it is in OPFS
  window.addEventListener("mzlab-index", (/** @type {any} */ e) => {
    const d = e.detail || {}, stem = String(d.name || "").replace(/\.[^.]+$/, "");
    if (!x || !stem) return;
    const was = x.loading.has(stem);
    x.loading.set(stem, d.frac || 0);
    for (const [key, spec] of x.observing) { const it = x.item(spec.item); if (it && it.stem === stem) progress(x, key, Math.min(0.99, d.frac || 0)); }
    if (!was || d.frac >= 1) change();
  });
  /** how Data classed a file of the live session (profile | centroid | mixed), or "" @param {string} name */
  const modeOf = name => { const it = x && x.ok ? x.item(name) : null; return it ? it.mode : ""; };
  /** the words of the card «Centroid spectra»: how many files are ready and why (centroided earlier, already centroided); "" for another card or before any file is listed @param {string[]} ids */
  const centroidNote = ids => {
    if (!x || !x.ok || !x.items.length || !ids.some(id => { const i = instOf(x, id); return i && base(i) === "spectra.centroid"; })) return "";
    const n = x.items.length, mode = x.items.filter((/** @type {any} */ i) => i.mode), done = mode.filter((/** @type {any} */ i) => x.profile(i) && i.cen).length, already = mode.filter((/** @type {any} */ i) => !x.profile(i)).length;
    return F.liveView.centroidWords(n, mode.length, done, already);
  };
  F.live = { isCentroidStep: id => { const i = x && instOf(x, id); return !!i && base(i) === "spectra.centroid"; }, modeOf, centroidNote, run, preflight, attach, change, state: () => x, following: () => following, held: () => held, setLive, setHeld, runNow, active: () => !!(x && x.ok) };
  change();
})();
