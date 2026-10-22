// @ts-check
// Flow (administrator mode): the panel of the assistant, to the right of the canvas. It shows the conversation, the level, the estimated cost, what will be sent, and the cards where the user
// decides (L0 «Apply»/«Discard» of a proposed plan, L1 «Allow»/«Deny», L2 a register of the edits with «Undo»). The loop is llm/llm-agente.js (the state machine is Rust); the tools are flow-llm-tools.js.
// Loaded by flow-shell.js only when the assistant is on (profile «ricerca», switched on in the settings, consent given). English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.flx .flx-body.flx-llm-on{grid-template-columns:minmax(0,1fr) 340px var(--llmw,360px)}.flx .flx-body.flx-adv.flx-llm-on{grid-template-columns:200px minmax(0,1fr) 340px var(--llmw,360px)}
.flx .flx-llm{display:flex;flex-direction:column;gap:6px;min-width:0;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--ink);padding:8px;max-height:70vh;position:relative}
.flx .flx-llm[hidden]{display:none}.flx .flx-llm h3{margin:0;font-size:14px}.flx .flx-llm-head{display:flex;flex-wrap:wrap;gap:6px;align-items:center}
.flx .flx-llm-level{border:1px solid var(--accent);color:var(--accent);border-radius:6px;padding:1px 8px;font-weight:600}.flx .flx-llm-cost{color:var(--muted);font-size:12px}
.flx .flx-llm button,.flx .flx-llm summary{min-height:24px;min-width:24px;cursor:pointer}.flx .flx-llm button:focus-visible,.flx .flx-llm summary:focus-visible,.flx .flx-llm textarea:focus-visible,.flx .flx-llm-grip:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
@media (pointer:coarse){.flx .flx-llm button,.flx .flx-llm summary{min-height:40px;min-width:40px}}
.flx .flx-llm-log{flex:1;overflow:auto;display:flex;flex-direction:column;gap:6px;min-height:120px}.flx .flx-llm-msg{border-radius:6px;padding:4px 8px;background:var(--soft);white-space:pre-wrap;overflow-wrap:anywhere}
.flx .flx-llm-msg.me{background:var(--sel);align-self:flex-end}.flx .flx-llm-msg.err{border:1px solid var(--bad);color:var(--bad)}
.flx .flx-llm-card{border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:6px;padding:6px 8px;display:flex;flex-direction:column;gap:4px;background:var(--panel)}
.flx .flx-llm-card.bad{border-left-color:var(--bad)}.flx .flx-llm-card ul{margin:0;padding-left:18px}.flx .flx-llm-card li.add::marker{color:var(--ok)}.flx .flx-llm-card li.remove::marker{color:var(--bad)}
.flx .flx-llm-card .row{display:flex;flex-wrap:wrap;gap:6px}.flx .flx-llm-done{opacity:.7}
.flx .flx-llm-reg{margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:2px;font-size:12px}.flx .flx-llm-reg li{display:flex;gap:6px;align-items:center;justify-content:space-between}
.flx .flx-llm pre{margin:0;max-height:160px;overflow:auto;font-size:11px;white-space:pre-wrap;overflow-wrap:anywhere;background:var(--soft);padding:4px;border-radius:4px}
.flx .flx-llm textarea{width:100%;min-height:48px;resize:vertical;background:var(--bg);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:4px 6px}
.flx .flx-llm-grip{display:flex;gap:4px;align-items:center;color:var(--muted);font-size:12px}
@media (max-width:900px){.flx .flx-body.flx-llm-on,.flx .flx-body.flx-adv.flx-llm-on{grid-template-columns:minmax(0,1fr)}}
@media (prefers-reduced-motion:reduce){.flx .flx-llm,.flx .flx-llm *{transition:none!important;animation:none!important;scroll-behavior:auto!important}}
`;
  const EDITS = new Set(["flow_add_step", "flow_set_param", "flow_remove_step", "flow_propose_plan"]);

  /** @param {string} text */
  const sha256 = async text => Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))).map(b => b.toString(16).padStart(2, "0")).join("");

  /** the button and the panel of the assistant, in the bar and beside the canvas; `act`: {submit, stop, close, toggle, resize}, filled by the mount @param {any} ctx @param {string} level @param {any} act */
  const buildPanel = (ctx, level, act) => {
    const { el, btn } = F;
    const log = el("div", { class: "flx-llm-log", id: "flx-llm-log", role: "log", "aria-live": "polite", "aria-label": "Conversation with the assistant" });
    const cost = el("span", { class: "flx-llm-cost", id: "flx-llm-cost", text: "Estimated cost: unknown" });
    const sent = el("pre", { id: "flx-llm-sent", tabindex: "0", "aria-label": "Text that will be sent" });
    const sentBox = el("details", { class: "flx-llm-sent" }, el("summary", { text: "What will be sent" }), sent);
    const regList = el("ul", { class: "flx-llm-reg", "aria-label": "Edits made by the assistant" });
    const input = /** @type {HTMLTextAreaElement} */ (el("textarea", { id: "flx-llm-input", "aria-label": "Message to the assistant", placeholder: "Ask for a plan or a change…", rows: "2" }));
    const send = btn("run", "Send the message", () => act.submit(), "Send");
    const stop = btn("stop", "Stop the assistant", () => { act.stop(); }, "Stop"); stop.id = "flx-llm-stop"; stop.hidden = true;
    const closeBtn = btn("x", "Close the assistant", () => act.close(), "Close");
    const grow = btn("plus", "Make the panel wider", () => act.resize(40), ""), shrink = btn("minus", "Make the panel narrower", () => act.resize(-40), "");
    const grip = el("div", { class: "flx-llm-grip", role: "group", "aria-label": "Width of the panel" }, shrink, grow, el("span", { text: "Width" }));
    const panel = el("aside", { class: "flx-llm", id: "flx-llm", "aria-label": "Assistant", hidden: true, tabindex: "-1" },
      el("div", { class: "flx-llm-head" }, el("h3", { text: "Assistant" }), el("span", { class: "flx-llm-level", id: "flx-llm-level", text: level, title: level === "L0" ? "L0: proposes, never edits" : level === "L1" ? "L1: asks before each edit" : "L2: edits by itself within limits" }), closeBtn), cost,
      log, regList, sentBox, input, el("div", { class: "row" }, send, stop), grip);
    const open = btn("chat", "Open the assistant", () => act.toggle(), "Assistant"); open.id = "flx-assistant"; open.setAttribute("aria-expanded", "false");
    ctx.bar.insertBefore(open, ctx.bar.querySelector(".flx-run"));
    ctx.body.append(panel);
    return { log, cost, sent, regList, input, send, stop, panel, open };
  };

  /** the messages and cards of the conversation log @param {HTMLElement} log */
  const logKit = log => {
    const { el, btn } = F;
    /** @param {string} text @param {string} [cls] */
    const say = (text, cls = "") => { const m = el("div", { class: "flx-llm-msg " + cls, text }); log.append(m); log.scrollTop = log.scrollHeight; return m; };
    /** @param {HTMLElement} card */
    const put = card => { log.append(card); log.scrollTop = log.scrollHeight; };
    /** a card with buttons that resolves with the label of the one pressed @param {string} title @param {string[]} lines @param {{label: string, name: string, icon: string}[]} buttons @param {string} [cls] @returns {Promise<string>} */
    const card = (title, lines, buttons, cls = "") => new Promise(done => {
      const ul = el("ul", {}, ...lines.map(t => { const kind = /^Add/.test(t) ? "add" : /^Remove/.test(t) ? "remove" : "change"; return el("li", { class: kind, text: t }); }));
      const row = el("div", { class: "row" });
      const c = el("div", { class: "flx-llm-card " + cls, role: "group", "aria-label": title }, el("strong", { text: title }), lines.length ? ul : null, row);
      for (const b of buttons) row.append(btn(b.icon, b.label, () => { row.remove(); c.classList.add("flx-llm-done"); c.append(el("em", { text: b.name + "." })); done(b.name); }, b.name));
      put(c);
      const first = row.querySelector("button"); if (first) /** @type {HTMLElement} */ (first).focus();
    });
    return { say, put, card };
  };

  /** the register of the edits made by the assistant and the proposal of a plan @param {any} ctx @param {any} kit @param {HTMLElement} regList @param {any[]} reg */
  const planKit = (ctx, kit, regList, reg) => {
    const { el, btn } = F, S = ctx.S, { card } = kit;
    const sync = () => { for (const r of reg) r.undo.disabled = !(S.history.length === r.depth && S.snap === r.snap); };
    ctx.root.addEventListener("flow:changed", sync);
    /** an edit of the plan made by the assistant: one entry of S.undo and a line of the register @param {string} label */
    const edited = label => {
      const r = /** @type {any} */ ({ label: label + " (by assistant)", depth: S.history.length, snap: S.snap });
      r.undo = btn("undo", "Undo: " + r.label, () => { S.undo(); r.row.classList.add("flx-llm-done"); r.undo.disabled = true; r.depth = -1; sync(); }, "Undo");
      r.row = el("li", {}, el("span", { text: r.label }), r.undo);
      reg.push(r); regList.append(r.row); sync();
    };

    /** the plan proposed by the model: the difference, then «Apply» (an entry of S.undo) or «Discard» @param {any} plan @param {string} note */
    const propose = async (plan, note) => {
      const diff = F.llmDiff(S.plan, plan);
      const pick = await card("Proposed plan" + (note ? ": " + note : ""), diff.length ? diff.map((/** @type {any} */ d) => d.text) : ["No change to the plan."], [{ label: "Apply the plan to the canvas", name: "Apply", icon: "ok" }, { label: "Discard the proposed plan", name: "Discard", icon: "x" }]);
      if (pick !== "Apply") return false;
      S.plan = plan; S.refresh(null); ctx.redraw(); edited("Applied plan");
      return true;
    };
    return { edited, propose };
  };

  /** what the model may know of the page, the reference of a run in `prov/1` and the estimated cost @param {any} S @param {any} runs @param {any} st @param {HTMLElement} cost */
  const dataKit = (S, runs, st, cost) => {
    const sources = () => ({
      profile: "ricerca", hide_names: !!(st.agent && st.agent.hideNames),
      plan: { sha256: S.snap || "", instances: S.plan.steps.map((/** @type {any} */ i) => ({ id: i.id, step: i.step, params: i.params })), problems: S.problems.map((/** @type {any} */ p) => ({ error_key: p.key, instance: p.instance, port: p.port })) },
      run: { steps: S.plan.steps.filter((/** @type {any} */ i) => runs.states[i.id]).map((/** @type {any} */ i) => ({ instance: i.id, step: i.step, status: runs.states[i.id] })) },
    });
    const context = async () => (st.agent ? st.agent.context(JSON.stringify(sources())) : "{}");
    /** what the charts can show: the page's collection of the open files and tables (`F.chartSources`, empty until the page has data) */
    const chartSources = () => JSON.stringify(typeof F.chartSources === "function" ? F.chartSources() : {});
    /** a specification of the model, checked by Rust: {ok, chart} or {ok: false, key, params} @param {any} spec */
    const chart = async spec => {
      try { return { ok: true, chart: await (await st.ensure()).chartMake(JSON.stringify(spec), chartSources()) }; } catch (e) { const x = /** @type {any} */ (e); return { ok: false, key: x && x.key || "err.llm.module", params: x && x.params || {} }; }
    };
    const chartViews = async () => (await st.ensure()).chartViews(chartSources());
    const chartPresets = async () => (await st.ensure()).chartPresets(chartSources());
    const llmRef = async () => ({ log_sha256: await sha256(st.agent ? await st.agent.log() : ""), turn: st.agent ? st.agent.turns : 0, provider: st.settings.provider, model: st.settings.model, level: st.level });

        const money = () => {
      const u = st.agent ? st.agent.usage : [0, 0];
      const tok = `${u[0]} in / ${u[1]} out tokens`;
      cost.textContent = st.price && st.price.in && st.price.out ? `Estimated cost: ${((u[0] * st.price.in + u[1] * st.price.out) / 1e6).toFixed(4)} ${st.price.currency || "USD"} (${tok})` : `Estimated cost: unknown (${tok})`;
    };
    return { context, llmRef, money, chart, chartViews, chartPresets };
  };

  /** what the page does for the machine of the agent: show text, call a tool, ask for a confirmation, close the turn @param {any} kit @param {any} ui @param {any} st */
  const makeHost = (kit, ui, st) => {
    const { el } = F;
    /** @type {any} */
    const host = {
      show: (/** @type {string} */ t) => { if (!host.cur) host.cur = kit.say("", ""); host.cur.textContent += t; ui.log.scrollTop = ui.log.scrollHeight; },
      log: (/** @type {any} */ line) => {
        let l = line; try { if (typeof l === "string") l = JSON.parse(l); } catch (_) { return; }
        if (l && l.ev === "decision" && l.decision === "deny") kit.say(`${l.tool} was refused (${l.key}).`, "err");
      },
      sending: (/** @type {any} */ r) => {
        host.cur = null;
        const body = r && r.body !== undefined ? r.body : r;
        ui.sent.textContent = `${r && r.url || ""}\n` + (typeof body === "string" ? body : JSON.stringify(body, null, 1)).slice(0, 8000);
        st.money();
      },
      call: async (/** @type {string} */ _id, /** @type {string} */ name, /** @type {any} */ args) => {
        host.cur = null;
        if (EDITS.has(name) && name !== "flow_propose_plan" && st.level === "L2" && ++st.edits > st.l2max) return { ok: false, content: { error_key: "err.llm.limit.edits", params: { max: st.l2max } } };
        const r = await st.tools.call(name, args);
        if (name === "flow_propose_plan" && !r.ok && r.content && r.content.problems) {
          kit.put(el("div", { class: "flx-llm-card bad", role: "group", "aria-label": "The proposed plan has problems" }, el("strong", { text: "The proposed plan has problems and cannot be applied" }),
            el("ul", {}, ...r.content.problems.map((/** @type {any} */ p) => el("li", { text: F.message({ key: p.error_key, params: p.params, instance: p.instance }) })))));
        }
        if (!r.ok && !(r.content && r.content.problems)) kit.say(`${name} failed: ${F.message({ key: r.content && r.content.error_key, params: r.content && r.content.params }) || r.content && r.content.error_key} ${JSON.stringify(r.content && r.content.params || {})}`, "err");
        return r;
      },
      ask: async (/** @type {string} */ _id, /** @type {string} */ summary, /** @type {string} */ name, /** @type {any} */ args) => {
        host.cur = null;
        const lines = [summary];
        if (args) lines.push(JSON.stringify(args).slice(0, 300));
        if (name === "flow_run" && F.runners.web && F.runners.web.available()) {
          try { const p = await F.runners.web.preflight(st.S); lines.push("Estimate: " + (p ? JSON.stringify(p).slice(0, 300) : "not available before the run")); } catch (_) { lines.push("Estimate: not available"); }
        }
        const pick = await kit.card("The assistant asks to " + (name || "act"), lines, [{ label: "Allow this action", name: "Allow", icon: "ok" }, { label: "Deny this action", name: "Deny", icon: "x" }]);
        return pick === "Allow";
      },
      done: (/** @type {string} */ outcome) => {
        host.cur = null; ui.stop.hidden = true; ui.send.disabled = false; ui.input.disabled = false; st.money();
        if (outcome.startsWith("error:")) kit.say(F.message({ key: outcome.slice(6), params: {} }) || "The assistant stopped: " + outcome.slice(6), "err");
        else if (outcome === "stopped") kit.say("Stopped.", "");
      },
    };
    return host;
  };

  /** mounts the button and the panel; `ctx`: {S, bar, body, say, redraw, select, setTab} */
  const llmMount = async ctx => {
    const S = ctx.S;
    // served from static/mzfinder/lazy/, the module from static/llm/ (a variable: the source tree has another layout)
    const where = "../../llm/llm-agente.js", mod = /** @type {any} */ (await import(where));
    if (!mod.assistantOn()) return null;
    const css = document.createElement("style"); css.textContent = CSS; document.head.append(css);
    const settings = JSON.parse(localStorage.getItem("mzlab.llm") || "{}");
    const level = ["L0", "L1", "L2"].includes(settings.level) ? settings.level : "L0";
    const l2max = Math.min(50, Math.max(1, Math.round(+settings.l2max || 5)));
    let width = 360;
    /** @type {any} */ const st = { S, settings, level, l2max, edits: 0, agent: null, price: null, tools: null, money: null };
    /** @type {{label: string, depth: number, snap: string, undo: HTMLButtonElement, row: HTMLElement}[]} */ const reg = [];
    const runs = { running: false, states: /** @type {Record<string, string>} */ ({}) };

    /** @type {any} */ const act = {};
    const { log, cost, sent, regList, input, send, stop, panel, open } = buildPanel(ctx, level, act);
    /** @param {number} d */
    const resize = d => { width = Math.max(260, Math.min(720, width + d)); ctx.body.style.setProperty("--llmw", width + "px"); };
    function show() { ctx.setTab("canvas"); panel.hidden = false; ctx.body.classList.add("flx-llm-on"); open.setAttribute("aria-expanded", "true"); input.focus(); }
    function close() { panel.hidden = true; ctx.body.classList.remove("flx-llm-on"); open.setAttribute("aria-expanded", "false"); open.focus(); }
    panel.addEventListener("keydown", e => {
      if (e.key === "Escape") { e.stopPropagation(); close(); }
      else if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); resize(40); } else if (e.altKey && e.key === "ArrowRight") { e.preventDefault(); resize(-40); }
    });
    input.addEventListener("keydown", e => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); submit(); } });
    Object.assign(act, { submit, resize, close, stop: () => { if (st.agent) st.agent.stop(); }, toggle: () => (panel.hidden ? show() : close()) });

    const kit = logKit(log); const { say, put, card } = kit;

    const { edited, propose } = planKit(ctx, kit, regList, reg);
    /** the rest of the work is the tools */
    const { context, llmRef, money, chart, chartViews, chartPresets } = dataKit(S, runs, st, cost); st.money = money;
    const showChart = (/** @type {any} */ c) => put(c.error === "no_view" ? F.llmChart.noView(c) : F.llmChart.figure(c));
    st.tools = F.llmTools({ S, runs, select: ctx.select, propose, edited, context: () => context(), llmRef: () => llmRef(), chart, chartViews, showChart });
    panel.insertBefore(F.llmChart.presetMenu(chartPresets, put), input);

    const host = makeHost(kit, { log, sent, stop, send, input }, st);

    /** the conversation (one worker), opened on first use by a message or by a chart @returns {Promise<any>} */
    st.ensure = async () => {
      if (st.agent) return st.agent;
      st.agent = new mod.AgentRun(host, {});
      await st.agent.open(true);
      try { const p = await (/** @type {any} */ (window).MZLAB_LLM && window.MZLAB_LLM.client ? window.MZLAB_LLM.client.providers() : []); const m = (p.find((/** @type {any} */ x) => x.id === settings.provider) || { models: [] }).models.find((/** @type {any} */ x) => x.id === settings.model); st.price = m && m.price || null; } catch (_) { /* no price: «unknown» */ }
      return st.agent;
    };

    async function submit() {
      const text = input.value.trim();
      if (!text || (st.agent && st.agent.running)) return;
      input.value = ""; say(text, "me"); st.edits = 0;
      stop.hidden = false; send.disabled = true;
      try {
        await st.ensure();
        await st.agent.user(text, await context());
      } catch (e) { host.done("error:" + (/** @type {any} */ (e) && /** @type {any} */ (e).key || "err.llm.module")); }
    }

    ctx.root.addEventListener("mzlab:llm-close", () => { if (st.agent) st.agent.close(); });
    return { open: show, close, panel, button: open };
  };
  F.llmMount = llmMount;
})();
