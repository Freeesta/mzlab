// @ts-check
// Flow (administrator mode): the Review view of a campaign folder. It walks the features that wait for a decision (2b, 3a, 3b) one at a time, shows what the engine proposes with the
// main proofs in words and tables, and saves a decision in one click: Accept (A), Skip (S), Reject the identity (R); arrows move, E opens the full Bench 2b. The engine is the one of
// the Banco 2b (`pool.attributioReview` / `pool.attributioDecision`); the program proposes and never decides: nothing is pre-ticked, a note is asked only when the level, the name or the
// identity changes. Saved decisions replace `feature_decisions.csv` in memory; "Download decisions" gives the file. English only, no keys of lang/.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const WAITING = ["2b", "3a", "3b"];
  const KEY = () => (typeof NOMI !== "undefined" ? NOMI.KEY("attributio.reviewer") : "qqq.attributio.reviewer");
  const num = (/** @type {any} */ v, n = 3) => (v == null || !isFinite(v) ? "–" : String(+Number(v).toFixed(n)));

  const css = () => {
    if (document.getElementById("flw-review-css")) return;
    const s = document.createElement("style"); s.id = "flw-review-css";
    s.textContent = ".flw-rv{border:1px solid var(--line);border-radius:8px;padding:12px;margin:10px 0;background:var(--panel);color:var(--ink)}.flw-rv h3{margin:0 0 6px}"
      + ".flw-rv-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin:8px 0}.flw-rv progress{width:160px;height:10px;accent-color:var(--accent)}"
      + ".flw-rv-acts button{min-height:40px;min-width:40px;padding:6px 14px;font:inherit}.flw-rv kbd{border:1px solid var(--line);border-radius:3px;padding:0 4px;font-size:11px;margin-left:6px}"
      + ".flw-rv-proofs{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:8px;margin:8px 0}.flw-rv-proofs div{border:1px solid var(--line);border-radius:6px;padding:6px 8px}"
      + ".flw-rv-proofs b{display:block;font-size:12px;color:var(--muted)}.flw-rv-note{width:100%;box-sizing:border-box}.flw-rv .err{color:var(--bad)}.flw-rv-card{opacity:1}"
      + "@media (prefers-reduced-motion:no-preference){.flw-rv-card{animation:flw-rv-in .18s ease-out}@keyframes flw-rv-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}}";
    document.head.appendChild(s);
  };

  /** the level to write when the engine's proposal is accepted: "1" is no decision (the cascade's level stays) @param {string} proposed */
  F.reviewLevel = proposed => (proposed === "1" ? "" : proposed);

  /** what a decision changes, in the words of the engine's rules (a copy of decision_changes; the engine checks again) @param {any} cur @param {string} level @param {string} group */
  F.reviewChanges = (cur, level, group) => {
    const to = /** @type {Record<string, string>} */ ({ "2b": "2b", "3": "3", "3a": "3", "3b": "3", "2": "2a", "2a": "2a" })[level], out = [];
    if (to && to !== cur.level) out.push(`level ${cur.level} -> ${to}`);
    if (group === "ambiguous" && (cur.group || "") !== "ambiguous") out.push("identity rejected");
    return out;
  };

  /** the rows of the campaign table that wait for a decision, as the list the engine reviews @param {{columns: string[], rows: any[][]}} data */
  F.reviewQueue = data => data.rows.filter(r => WAITING.includes(String(r[4]) + (r[5] || ""))).map(r => ({ pol: String(r[0]), id: Number(r[1]) }));

  /** the decision the engine reads for one feature @param {any} rev @param {{level: string, group: string, note: string, by: string}} d */
  F.reviewDecision = (rev, d) => {
    const ev = rev.proposal.evidence, t = new Date(), day = `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}-${String(t.getDate()).padStart(2, "0")}`, frag = (ev.n_explained || 0) > 0;
    return {
      pol: rev.pol, id: rev.id, level: d.level, name: "", identity: "", group: d.group, justification: d.note, decided_on: day, decided_by: d.by, rt_check: "",
      evidence: { type: frag ? "fragments" : "", library: "", entry: "", cosine: frag ? ev.frac_explained ?? "" : "", matched: frag ? ev.n_explained ?? "" : "", delta_mz: "", delta_formula: "", note: "" }, checklist: [],
    };
  };

  /** the main proofs of a review as [title, words] @param {any} rev */
  const proofsOf = rev => {
    const ev = rev.proposal.evidence, alt = (rev.alternatives || [])[0], rt = rev.rtlogp;
    return [
      ["Fragments", ev.n_peaks ? `${ev.n_explained ?? 0} of ${ev.n_peaks} peaks explained` : "no MS2 for this feature"],
      ["Library", alt ? `${alt.name} (${alt.score == null ? "no score" : "cosine " + num(alt.score)})` : "no match in the libraries"],
      ["RT", rt && rt.ok != null ? (rt.ok ? "inside the window predicted from logP" : "outside the window predicted from logP") : "no prediction"],
      ["Purity", (ev.purity && ev.purity.badge) || "–"],
    ];
  };

  /** the explained fragments of a review as table rows @param {any} rev */
  const fragRows = rev => ((rev.proposal.evidence.view && rev.proposal.evidence.view.peaks) || []).filter((/** @type {any} */ x) => x.frag).map((/** @type {any} */ x) => [num(x.mz, 4), num(x.rel, 1), x.frag.formula, num(x.frag.ppm, 2), x.frag.loss]);

  /** an action button with its key shown @param {any} icon @param {string} name @param {string} k @param {() => void} fn @param {string} text */
  const act = (icon, name, k, fn, text) => { const b = F.btn(icon, `${name} (${k})`, fn, text); b.append(F.el("kbd", { text: k })); return b; };

  /** the review screen in `host` for the campaign `c`; `hooks.onSaved(text, path)` takes the new decisions file, `hooks.expert(list, index)` opens the Bench 2b, `hooks.close(changed)` goes back @param {HTMLElement} host @param {any} c @param {any} hooks */
  F.reviewMount = (host, c, hooks) => {
    css();
    const { el } = F, list = F.reviewQueue(c.data), done = /** @type {Set<string>} */ (new Set()), st = { at: 0, rev: /** @type {any} */ (null), busy: false, text: "", path: "", wrote: false };
    const root = el("section", { class: "flw-rv", "aria-label": "Review", tabindex: -1 }), key = (/** @type {any} */ f) => f.pol + f.id;
    host.replaceChildren(root);
    const download = () => G.ATTRIBUTIO_CORE.download(st.path.split("/").pop() || "feature_decisions.csv", st.text, "text/csv;charset=utf-8");
    const back = () => F.btn("flow", "Back to the table", () => hooks.close(!!st.text), "Back to the table");
    const saved = () => (!st.text ? null : st.wrote ? F.el("span", { class: "mut", role: "status", text: `Saved in ${st.path}.` }) : F.btn("export", "Download the decisions file", download, "Download decisions"));
    // the next feature without a decision in the direction d, or -1
    const next = (/** @type {number} */ d) => { for (let k = 1; k <= list.length; k++) { const i = (st.at + d * k + list.length * k) % list.length; if (!done.has(key(list[i]))) return i; } return -1; };
    const go = (/** @type {number} */ d) => { const n = next(d); if (n >= 0 && n !== st.at) show(n); };
    const finish = () => root.replaceChildren(el("h3", { text: "Review" }), el("p", { role: "status", text: list.length ? `All ${list.length} features have a decision.` : "Nothing to review: no feature is at level 2b, 3a or 3b." }), el("div", { class: "flw-rv-bar" }, saved(), back()));

    /** saves one decision, then shows the next feature @param {any} rev @param {string} level @param {string} group @param {{by: any, note: HTMLTextAreaElement, msg: HTMLElement}} f */
    const decide = async (rev, level, group, f) => {
      if (st.busy) return;
      const changes = F.reviewChanges(rev.current, level, group), needNote = changes.length > 0 || level === "2b", bad = (/** @type {string} */ m) => { f.msg.textContent = m; f.msg.className = "err"; };
      if (!f.by.value.trim()) { bad("Write the reviewer first."); f.by.focus(); return; }
      if (needNote && !f.note.value.trim()) { f.note.hidden = false; f.note.focus(); bad(`A note is required: ${changes.join(", ") || "a 2b row"}. Write it and press the button again.`); return; }
      st.busy = true; f.msg.textContent = "Saving…"; f.msg.className = "mut";
      try { localStorage.setItem(KEY(), f.by.value.trim()); } catch (_) { /* optional */ }
      try {
        const d = F.reviewDecision(rev, { level, group, note: f.note.value.trim(), by: f.by.value.trim() });
        const r = await (await G.ATTRIBUTIO_CORE.pool()).attributioDecision(c.files, c.root, JSON.stringify(d), "");
        st.text = r.text; st.path = r.summary.path;
        // a folder chosen with the browser's picker takes the decisions file in place; else it stays in memory and the button downloads it
        if (c.dir) st.wrote = !!(r.summary.written || await G.ATTRIBUTIO_BANCO.writeInFolder(c.dir, st.path, r.text));
        hooks.onSaved(r.text, r.summary.path);
        if (G.WORKFLOW && G.WORKFLOW.logOp) G.WORKFLOW.logOp("attributio", { attributio: { decision: { pol: d.pol, id: d.id, level: d.level, by: d.decided_by, on: d.decided_on }, feature_decisions: r.text } });
        done.add(key(list[st.at])); st.busy = false;
        const n = next(1); if (n < 0) finish(); else show(n);
      } catch (e) { const x = /** @type {any} */ (e); st.busy = false; bad("Not saved: " + ((x && x.params && x.params.detail) || (x && x.message) || String(e))); }
    };

    /** the card of the feature in `st.rev` */
    const draw = () => {
      const rev = st.rev, p = rev.proposal, cur = rev.current, by = el("input", { type: "text", "aria-label": "Reviewer", autocomplete: "off", placeholder: "Reviewer" }), msg = el("p", { role: "status", class: "mut" });
      const note = /** @type {HTMLTextAreaElement} */ (el("textarea", { class: "flw-rv-note", rows: 2, hidden: true, "aria-label": "Why? (required: the level or the identity changes)", placeholder: "Why? (required: the level or the identity changes)" }));
      try { by.value = localStorage.getItem(KEY()) || ""; } catch (_) { /* no storage: the reviewer is typed every time */ }
      const f = { by, note, msg }, accept = () => decide(rev, F.reviewLevel(p.proposed), "", f), reject = () => decide(rev, "", "ambiguous", f), expert = () => hooks.expert(list, st.at);
      const card = el("div", { class: "flw-rv-card" },
        el("h3", { tabindex: -1, id: "flw-rv-h", text: `Review · ${rev.pol} ${rev.id} · m/z ${num(rev.mz, 4)} · RT ${num(rev.rt, 2)}` }),
        el("progress", { max: list.length, value: done.size, "aria-label": `${done.size} of ${list.length} decided` }), el("span", { class: "mut", text: ` ${done.size} of ${list.length} decided · feature ${st.at + 1}` }),
        el("p", {}, el("strong", { text: `Now L${cur.level}${cur.sub && cur.sub !== "a" ? cur.sub : ""}` }), ` ${cur.name || cur.identity || "no name"}. The program proposes: ${p.class} (level ${p.proposed}). The decision is yours.`),
        el("ul", {}, ...(p.reasons || []).slice(0, 4).map((/** @type {string} */ r) => el("li", { text: r }))),
        el("div", { class: "flw-rv-proofs" }, ...proofsOf(rev).map(([t, v]) => el("div", {}, el("b", { text: t }), v))),
        el("div", { class: "flw-rv-bar flw-rv-acts" }, act("ok", "Accept the proposal", "A", accept, "Accept"), act("run", "Skip this feature", "S", () => go(1), "Skip"),
          act("x", "Reject the identity", "R", reject, "Reject identity"), act("search", "Open the full Bench 2b", "E", expert, "Expert view"), back()),
        note, el("div", { class: "flw-rv-bar" }, el("label", {}, "Reviewer ", by), msg), el("div", { class: "flw-rv-bar" }, saved()));
      const frags = fragRows(rev);
      if (frags.length) F.drawTable(card, { caption: "Fragments explained", head: ["m/z", "%", "Fragment ion formula", "ppm", "Loss"], rows: frags, label: "Fragments explained" });
      root.replaceChildren(card);
      /** @type {HTMLElement} */ (root.querySelector("#flw-rv-h")).focus();
      root.onkeydown = e => {
        const t = /** @type {HTMLElement} */ (e.target), k = e.key.toLowerCase();
        if (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || e.altKey || e.ctrlKey || e.metaKey) return;
        if (k === "a") accept(); else if (k === "s" || k === "arrowright") go(1); else if (k === "r") reject(); else if (k === "e") expert(); else if (k === "arrowleft") go(-1); else return;
        e.preventDefault();
      };
    };

    /** the proofs of the feature `i` from the engine, then its card @param {number} i */
    const show = async i => {
      st.at = i; st.rev = null;
      const f = list[i], title = el("h3", { text: `Review · ${f.pol} ${f.id}` });
      root.replaceChildren(title, el("p", { role: "status", text: "Loading the proofs…" }));
      try { st.rev = await (await G.ATTRIBUTIO_CORE.pool()).attributioReview(c.files, c.root, f.pol, f.id); } catch (e) {
        const x = /** @type {any} */ (e);
        root.replaceChildren(title, el("p", { class: "err", role: "alert", text: (x && x.params && x.params.detail) || (x && x.message) || String(e) }), F.btn("run", "Skip this feature", () => go(1), "Skip"), back()); return;
      }
      draw();
    };
    if (!list.length) finish(); else show(0);
  };
})();
