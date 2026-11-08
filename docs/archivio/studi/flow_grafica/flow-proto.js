// Flow prototype, canvas half: DOM nodes on a transformed layer, SVG wires, typed ports, click-or-drag linking, proposed adapters,
// wire previews, a fake run with per-node progress and cache, save/load of mzflow/2 + ui. Study probe of docs/studi/FLOW_GRAFICA.md.
// All rules come from FLOW (flow-proto-model.js); this file only draws, listens and measures.
"use strict";
(() => {
  const $ = q => document.querySelector(q);
  const view = $("#view"), world = $("#world"), svg = $("#wires"), layer = $("#nodes"), msg = $("#msg"), pop = $("#pop"), insp = $("#insp");
  const TXT = {
    "err.flow.type": p => `Not compatible: ${p.from} cannot feed ${p.to}.`,
    "err.flow.cycle": () => "Not allowed: this link would make a loop.",
    "err.flow.cardinality": () => "Not compatible: many files cannot feed a single input.",
    "err.flow.missing": p => `Missing input “${p.port}” in ${p.title}.`,
    "err.flow.port": () => "Unknown port.",
    "err.flow.format": () => "This file is not an mzflow/2 workflow.",
    "err.flow.unavailable": () => "This workflow uses a step that is not available here.",
  };
  const say = t => { msg.textContent = t; };
  const reduce = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
  let G = { nodes: [], edges: [] }, V = { x: 0, y: 0, k: 1 }, seq = 0;
  const els = new Map(), ports = new Map(), wires = new Map(), status = new Map(), cache = new Map();
  let armed = null, temp = null, pending = new Set(), raf = 0;

  // ---- drawing ----
  const SHAPES = {
    square: '<rect x="2" y="2" width="10" height="10" rx="1"/>', circle: '<circle cx="7" cy="7" r="5.5"/>',
    diamond: '<path d="M7 1 13 7 7 13 1 7Z"/>', triangle: '<path d="M2 2 12 7 2 12Z"/>', hexagon: '<path d="M4 1.5h6L13 7l-3 5.5H4L1 7Z"/>',
  };
  const glyph = t => `<svg class="dot t-${t}" viewBox="0 0 14 14" aria-hidden="true">${SHAPES[FLOW.TYPES[t].shape]}</svg>`;
  function portBtn(n, p, dir) {
    const T = FLOW.TYPES[p.type], s = FLOW.step(n);
    const lab = `${dir === "in" ? "Input" : "Output"} ${p.id}, ${T.title}${p.many ? ", one per file" : ""}${p.optional ? ", optional" : ""}, of ${s.title}`;
    return `<button class="port ${dir}" data-dir="${dir}" data-port="${p.id}" data-type="${p.type}" aria-label="${lab}" title="${T.title}">` +
      (dir === "in" ? `${glyph(p.type)}<span>${p.id}</span>` : `<span>${p.id}</span>${glyph(p.type)}`) + "</button>";
  }
  function nodeEl(n) {
    const s = FLOW.step(n), d = document.createElement("div");
    d.className = "node"; d.tabIndex = 0; d.dataset.id = n.id;
    d.setAttribute("role", "group"); d.setAttribute("aria-label", `${s.title} (${n.id})`);
    d.innerHTML = `<div class="hd"><span class="ic" aria-hidden="true">${s.icon}</span><span class="tt">${s.title}</span>` +
      `<span class="st" data-st="idle"><span class="sr">idle</span></span></div>` +
      `<div class="ports"><div class="ins">${(s.in || []).map(p => portBtn(n, p, "in")).join("")}</div>` +
      `<div class="outs">${(s.out || []).map(p => portBtn(n, p, "out")).join("")}</div></div><div class="pg" aria-hidden="true"><i></i></div>`;
    d.style.transform = `translate(${n.x}px,${n.y}px)`;
    return d;
  }
  /** Port centres relative to their node, measured once (unscaled), so moving a node never re-measures the DOM. */
  function measure(n) {
    const d = els.get(n.id), r0 = d.getBoundingClientRect();
    for (const b of d.querySelectorAll(".port")) {
      const r = b.querySelector(".dot").getBoundingClientRect();
      ports.set(`${n.id}:${b.dataset.dir}:${b.dataset.port}`, { dx: (r.left + r.width / 2 - r0.left) / V.k, dy: (r.top + r.height / 2 - r0.top) / V.k });
    }
  }
  const ekey = e => `${e.from}.${e.fp}>${e.to}.${e.tp}`;
  const at = (id, dir, p) => { const n = G.nodes.find(x => x.id === id), o = ports.get(`${id}:${dir}:${p}`) || { dx: 0, dy: 0 }; return [n.x + o.dx, n.y + o.dy]; };
  function curve(a, b) {
    const dx = Math.max(40, Math.abs(b[0] - a[0]) / 2);
    return `M${a[0]},${a[1]} C${a[0] + dx},${a[1]} ${b[0] - dx},${b[1]} ${b[0]},${b[1]}`;
  }
  function wireEl(e) {
    const p = document.createElementNS("http://www.w3.org/2000/svg", "path"), src = G.nodes.find(n => n.id === e.from);
    const t = FLOW.portOf(src, "out", e.fp).type;
    p.setAttribute("class", `wire t-${t}`); p.setAttribute("tabindex", "0"); p.setAttribute("role", "button");
    p.setAttribute("aria-label", `Link ${FLOW.step(src).title} ${e.fp} to ${FLOW.step(G.nodes.find(n => n.id === e.to)).title} ${e.tp}, ${FLOW.TYPES[t].title}`);
    p.dataset.k = ekey(e);
    return p;
  }
  function drawWire(e) { wires.get(ekey(e))?.setAttribute("d", curve(at(e.from, "out", e.fp), at(e.to, "in", e.tp))); }
  function render() {
    layer.textContent = ""; svg.textContent = ""; els.clear(); wires.clear(); ports.clear();
    const frag = document.createDocumentFragment();
    for (const n of G.nodes) { const d = nodeEl(n); els.set(n.id, d); frag.append(d); }
    layer.append(frag);
    // All reads first, then all writes: one layout instead of one per node.
    for (const n of G.nodes) measure(n);
    for (const n of G.nodes) paintStatus(n.id);
    for (const e of G.edges) { const w = wireEl(e); wires.set(ekey(e), w); svg.append(w); drawWire(e); }
    applyView();
  }
  function applyView() { world.style.transform = `translate(${V.x}px,${V.y}px) scale(${V.k})`; }
  /** Moves are batched to one frame: only the moved nodes and their wires are touched. */
  function moved(id) {
    pending.add(id);
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      for (const k of pending) {
        const n = G.nodes.find(x => x.id === k);
        els.get(k).style.transform = `translate(${n.x}px,${n.y}px)`;
        for (const e of G.edges) if (e.from === k || e.to === k) drawWire(e);
      }
      pending.clear();
    });
  }

  // ---- status, cache, run ----
  const LABEL = { idle: "idle", stale: "needs run", queued: "queued", running: "running", done: "done", cached: "from cache", error: "error" };
  function paintStatus(id) {
    const s = status.get(id) || { st: "idle", f: 0 }, d = els.get(id);
    if (!d) return;
    const b = d.querySelector(".st");
    b.dataset.st = s.st; b.querySelector(".sr").textContent = LABEL[s.st];
    b.title = LABEL[s.st];
    d.querySelector(".pg i").style.transform = `scaleX(${s.f})`;
    d.classList.toggle("err", s.st === "error");
  }
  function setStatus(id, st, f = 0) { status.set(id, { st, f }); paintStatus(id); }
  function downstream(id) {
    const out = new Set([id]);
    let grow = true;
    while (grow) { grow = false; for (const e of G.edges) if (out.has(e.from) && !out.has(e.to)) { out.add(e.to); grow = true; } }
    return out;
  }
  function invalidate(id) { for (const k of downstream(id)) if ((status.get(k) || {}).st !== "idle") setStatus(k, "stale"); }
  function progress(id, ms) {
    return new Promise(res => {
      const t0 = performance.now();
      const tick = now => {
        const f = Math.min(1, (now - t0) / ms);
        setStatus(id, "running", f);
        f < 1 ? requestAnimationFrame(tick) : res();
      };
      requestAnimationFrame(tick);
    });
  }
  async function run() {
    const probs = FLOW.problems(G);
    if (probs.length) {
      for (const p of probs) setStatus(p.instance, "error");
      const p = probs[0], n = G.nodes.find(x => x.id === p.instance);
      say(TXT[p.error_key]({ port: p.port, title: FLOW.step(n).title }) + (probs.length > 1 ? ` (+${probs.length - 1} more)` : ""));
      els.get(p.instance).focus();
      return;
    }
    const plan = FLOW.order(G), memo = new Map();
    for (const id of plan) setStatus(id, "queued");
    let ran = 0;
    for (const id of plan) {
      const k = FLOW.key(G, id, memo), n = G.nodes.find(x => x.id === id);
      if (cache.get(id) === k) { setStatus(id, "cached", 1); continue; }
      live(id, true);
      await progress(id, FLOW.step(n).ms);
      live(id, false);
      cache.set(id, k); setStatus(id, "done", 1); ran++;
    }
    say(`Done: ${ran} step(s) run, ${plan.length - ran} from cache.`);
  }
  function live(id, on) { for (const e of G.edges) if (e.to === id) wires.get(ekey(e))?.classList.toggle("live", on && !reduce()); }

  // ---- linking ----
  function arm(id, port) {
    armed = { id, port };
    let ok = 0, ad = 0;
    for (const d of els.values()) {
      let any = false;
      for (const b of d.querySelectorAll(".port.in")) {
        const r = FLOW.check(G, G.nodes.find(n => n.id === armed.id), port, G.nodes.find(n => n.id === d.dataset.id), b.dataset.port);
        b.classList.remove("ok", "adapt", "no");
        b.classList.add(r.ok ? "ok" : r.adapt ? "adapt" : "no");
        b.dataset.hint = r.ok ? "Compatible" : r.adapt ? `Needs: ${r.adapt.map(s => FLOW.STEPS[s].title).join(" → ")}` : TXT[r.error_key](r.params || {});
        if (r.ok) ok++; if (r.adapt) ad++;
        any = any || !!(r.ok || r.adapt);
      }
      d.classList.toggle("dim", !any && d.dataset.id !== id);
    }
    view.classList.add("linking");
    say(`Pick an input: ${ok} compatible, ${ad} through a conversion. Escape cancels.`);
  }
  function disarm() {
    armed = null; temp?.remove(); temp = null;
    view.classList.remove("linking");
    for (const d of els.values()) { d.classList.remove("dim"); d.querySelectorAll(".port.in").forEach(b => b.classList.remove("ok", "adapt", "no")); }
  }
  function connect(src, sp, dst, dp) {
    const S = G.nodes.find(n => n.id === src), D = G.nodes.find(n => n.id === dst), r = FLOW.check(G, S, sp, D, dp);
    if (r.adapt) return propose(S, sp, D, dp, r.adapt);
    if (!r.ok) return say(TXT[r.error_key](r.params || {}));
    if (!FLOW.portOf(D, "in", dp).many) G.edges = G.edges.filter(e => !(e.to === dst && e.tp === dp));
    G.edges.push({ from: src, fp: sp, to: dst, tp: dp });
    invalidate(dst); render(); say(`Linked ${FLOW.step(S).title} → ${FLOW.step(D).title}.`);
  }
  /** The missing step is proposed, never inserted silently (COMUNICAZIONE.md §2): one click on Insert. */
  function propose(S, sp, D, dp, path) {
    const names = path.map(s => FLOW.STEPS[s].title).join(" → ");
    const b = els.get(D.id).querySelector(`.port.in[data-port="${dp}"]`).getBoundingClientRect();
    showPop(b.left, b.bottom + 6, `<p>${FLOW.TYPES[FLOW.portOf(S, "out", sp).type].title} cannot go straight into ${FLOW.TYPES[FLOW.portOf(D, "in", dp).type].title}.</p>` +
      `<p><b>Insert: ${names}</b></p><div class="row"><button data-a="ins" class="pri">Insert</button><button data-a="no">Cancel</button></div>`, a => {
      if (a !== "ins") return;
      let from = S, fp = sp;
      path.forEach((name, i) => {
        const n = add(name, S.x + (D.x - S.x) * (i + 1) / (path.length + 1), (S.y + D.y) / 2 + 40);
        G.edges.push({ from: from.id, fp, to: n.id, tp: FLOW.STEPS[name].in[0].id });
        from = n; fp = FLOW.STEPS[name].out[0].id;
      });
      G.edges.push({ from: from.id, fp, to: D.id, tp: dp });
      invalidate(D.id); render(); say(`Inserted ${names}.`);
    });
  }
  function showPop(x, y, html, done) {
    pop.innerHTML = html; pop.hidden = false;
    pop.style.left = `${Math.min(x, innerWidth - 300)}px`; pop.style.top = `${Math.min(y, innerHeight - 160)}px`;
    pop.onclick = ev => { const a = ev.target.closest("[data-a]")?.dataset.a; if (!a) return; pop.hidden = true; done(a); };
    pop.querySelector("button")?.focus();
  }
  function add(name, x, y) { const n = { id: `n${++seq}`, step: name, params: {}, x, y }; G.nodes.push(n); return n; }

  // ---- wire preview ----
  const FAKE = { raw: "3 files · 41.2 MB · HR, positive", mzml: "3 files · 63.0 MB", mzidx: "3 indexes · 2 410 scans", peaks: "3 files · 1 902 / 1 877 / 1 940 features",
    table: "2 115 rows × 3 samples", structure: "C13H24N4O3S · 1 stereocentre (unknown)", tp: "14 candidate rows", report: "CSV · 2 115 rows" };
  function preview(path) {
    const e = G.edges.find(x => ekey(x) === path.dataset.k), src = G.nodes.find(n => n.id === e.from), t = FLOW.portOf(src, "out", e.fp).type;
    const st = (status.get(src.id) || {}).st, ready = st === "done" || st === "cached";
    const r = path.getBoundingClientRect();
    showPop(r.left + r.width / 2, r.top + r.height / 2, `<p>${glyph(t)} <b>${FLOW.TYPES[t].title}</b></p>` +
      `<p>${ready ? FAKE[t] + `<br><small>key ${cache.get(src.id)}</small>` : "Not computed yet: Run to see what flows here."}</p>` +
      `<div class="row"><button data-a="del">Remove link</button><button data-a="no">Close</button></div>`, a => {
      if (a === "del") { G.edges = G.edges.filter(x => x !== e); invalidate(e.to); render(); say("Link removed."); }
    });
  }

  // ---- pointer: drag nodes, link ports, pan, pinch ----
  const touches = new Map();
  let drag = null;
  view.addEventListener("pointerdown", ev => {
    const port = ev.target.closest(".port"), hd = ev.target.closest(".hd"), wire = ev.target.closest(".wire");
    pop.hidden = true;
    if (wire) return preview(wire);
    view.setPointerCapture(ev.pointerId);
    touches.set(ev.pointerId, [ev.clientX, ev.clientY]);
    if (port) { drag = { kind: "port", port, x0: ev.clientX, y0: ev.clientY }; return; }
    if (hd) { const id = hd.parentElement.dataset.id; drag = { kind: "node", id, x: ev.clientX, y: ev.clientY }; return; }
    drag = touches.size === 2 ? { kind: "pinch", d0: dist(), k0: V.k } : { kind: "pan", x: ev.clientX, y: ev.clientY };
  });
  const dist = () => { const [a, b] = [...touches.values()]; return Math.hypot(a[0] - b[0], a[1] - b[1]) || 1; };
  view.addEventListener("pointermove", ev => {
    if (!drag) return;
    touches.set(ev.pointerId, [ev.clientX, ev.clientY]);
    if (drag.kind === "node") {
      const n = G.nodes.find(x => x.id === drag.id);
      n.x += (ev.clientX - drag.x) / V.k; n.y += (ev.clientY - drag.y) / V.k; drag.x = ev.clientX; drag.y = ev.clientY;
      moved(n.id);
    } else if (drag.kind === "pan") {
      V.x += ev.clientX - drag.x; V.y += ev.clientY - drag.y; drag.x = ev.clientX; drag.y = ev.clientY; applyView();
    } else if (drag.kind === "pinch" && touches.size === 2) {
      V.k = Math.max(0.2, Math.min(2.5, drag.k0 * dist() / drag.d0)); applyView();
    } else if (drag.kind === "port" && drag.port.dataset.dir === "out") {
      if (!armed) arm(drag.port.closest(".node").dataset.id, drag.port.dataset.port);
      if (!temp) { temp = document.createElementNS("http://www.w3.org/2000/svg", "path"); temp.setAttribute("class", `wire temp t-${drag.port.dataset.type}`); svg.append(temp); }
      const r = world.getBoundingClientRect();
      temp.setAttribute("d", curve(at(armed.id, "out", armed.port), [(ev.clientX - r.left) / V.k, (ev.clientY - r.top) / V.k]));
    }
  });
  view.addEventListener("pointerup", ev => {
    const d = drag;
    drag = null; touches.delete(ev.pointerId);
    if (!d || d.kind !== "port") return;
    const far = Math.hypot(ev.clientX - d.x0, ev.clientY - d.y0) > 8;
    const hit = document.elementFromPoint(ev.clientX, ev.clientY)?.closest(".port.in");
    if (far && hit) { const a = armed; disarm(); return connect(a.id, a.port, hit.closest(".node").dataset.id, hit.dataset.port); }
    if (far) { const a = armed; disarm(); return openPalette(a); }
    portClick(d.port);
  });
  /** A click (or Enter) on a port: the same linking without any dragging (WCAG 2.5.7). */
  function portClick(b) {
    const id = b.closest(".node").dataset.id;
    if (b.dataset.dir === "out") { if (armed) disarm(); arm(id, b.dataset.port); b.focus(); return; }
    if (armed) { const a = armed; disarm(); connect(a.id, a.port, id, b.dataset.port); }
    else say("Pick an output first (the ports on the right).");
  }
  view.addEventListener("wheel", ev => {
    ev.preventDefault();
    if (ev.ctrlKey) zoomAt(ev.clientX, ev.clientY, Math.exp(-ev.deltaY / 300));
    else { V.x -= ev.deltaX; V.y -= ev.deltaY; applyView(); }
  }, { passive: false });
  function zoomAt(cx, cy, f) {
    const r = view.getBoundingClientRect(), k = Math.max(0.2, Math.min(2.5, V.k * f));
    V.x = cx - r.left - (cx - r.left - V.x) * k / V.k; V.y = cy - r.top - (cy - r.top - V.y) * k / V.k; V.k = k; applyView();
  }
  function fit() {
    if (!G.nodes.length) return;
    const xs = G.nodes.map(n => n.x), ys = G.nodes.map(n => n.y), r = view.getBoundingClientRect();
    const w = Math.max(...xs) - Math.min(...xs) + 220, h = Math.max(...ys) - Math.min(...ys) + 160;
    V.k = Math.max(0.2, Math.min(1.2, Math.min(r.width / w, r.height / h)));
    V.x = (r.width - w * V.k) / 2 - Math.min(...xs) * V.k; V.y = (r.height - h * V.k) / 2 - Math.min(...ys) * V.k; applyView();
  }

  // ---- keyboard ----
  view.addEventListener("keydown", ev => {
    const t = ev.target;
    if (ev.key === "Escape") { disarm(); pop.hidden = true; insp.hidden = true; return say("Cancelled."); }
    if (t.classList?.contains("port") && (ev.key === "Enter" || ev.key === " ")) { ev.preventDefault(); return portClick(t); }
    if (t.classList?.contains("wire") && (ev.key === "Enter" || ev.key === " ")) { ev.preventDefault(); return preview(t); }
    if (t.classList?.contains("wire") && (ev.key === "Delete" || ev.key === "Backspace")) {
      G.edges = G.edges.filter(x => ekey(x) !== t.dataset.k); render(); return say("Link removed.");
    }
    if (!t.classList?.contains("node")) return;
    const step = ev.shiftKey ? 64 : 16, n = G.nodes.find(x => x.id === t.dataset.id);
    const dx = { ArrowLeft: -step, ArrowRight: step }[ev.key] || 0, dy = { ArrowUp: -step, ArrowDown: step }[ev.key] || 0;
    if (dx || dy) { ev.preventDefault(); n.x += dx; n.y += dy; moved(n.id); return; }
    if (ev.key === "Enter") { ev.preventDefault(); inspect(n); }
    if (ev.key === "Delete") { G.nodes = G.nodes.filter(x => x !== n); G.edges = G.edges.filter(e => e.from !== n.id && e.to !== n.id); render(); say(`${FLOW.step(n).title} removed.`); }
  });
  view.addEventListener("dblclick", ev => { const d = ev.target.closest(".node"); if (d) inspect(G.nodes.find(x => x.id === d.dataset.id)); });
  document.addEventListener("keydown", ev => {
    if (ev.target.matches("input,select,textarea") || ev.ctrlKey || ev.metaKey) return;
    if (ev.key === "a" || ev.key === "A") { ev.preventDefault(); openPalette(null); }
    if (ev.key === "r" || ev.key === "R") run();
  });

  // ---- inspector: the form is generated from the params schema ----
  function inspect(n) {
    const s = FLOW.step(n), keys = Object.keys(s.params);
    insp.innerHTML = `<h2>${s.icon} ${s.title}</h2>` + (keys.length ? keys.map(k => {
      const p = s.params[k], v = n.params[k], auto = p.estimate && v === undefined;
      return `<label>${p.title} <small>(${p.unit})</small><input type="number" data-k="${k}" value="${v ?? p.default}" ${p.minimum !== undefined ? `min="${p.minimum}"` : ""} ${p.maximum !== undefined ? `max="${p.maximum}"` : ""} ${auto ? "disabled" : ""}></label>` +
        (p.estimate ? `<label class="chk"><input type="checkbox" data-auto="${k}" ${auto ? "checked" : ""}> Auto (estimated from the data, with its proof)</label>` : "");
    }).join("") : "<p>No settings.</p>") + `<button data-a="close">Close</button>`;
    insp.hidden = false;
    insp.onchange = ev => {
      const k = ev.target.dataset.k, a = ev.target.dataset.auto;
      if (k) n.params[k] = Number(ev.target.value);
      if (a) { if (ev.target.checked) delete n.params[a]; else n.params[a] = s.params[a].default; inspect(n); }
      invalidate(n.id); say(`${s.title}: settings changed, this step and the ones after it need a run.`);
    };
    insp.onclick = ev => { if (ev.target.dataset.a === "close") { insp.hidden = true; els.get(n.id).focus(); } };
    insp.querySelector("input,button").focus();
  }

  // ---- palette: add a step, filtered by what the armed output can feed ----
  const pal = $("#palette"), palQ = $("#pal-q"), palL = $("#pal-list");
  let palFrom = null;
  function openPalette(from) { palFrom = from; palQ.value = ""; fillPalette(); pal.showModal(); palQ.focus(); }
  function fillPalette() {
    const q = palQ.value.toLowerCase(), src = palFrom && G.nodes.find(n => n.id === palFrom.id);
    const t = src && FLOW.portOf(src, "out", palFrom.port).type;
    const fits = name => !t || (FLOW.STEPS[name].in || []).some(p => p.type === t || (p.accepts || []).includes(t) || (FLOW.convPath(t, p.type) || []).length);
    palL.innerHTML = Object.entries(FLOW.STEPS).filter(([n, s]) => fits(n) && s.title.toLowerCase().includes(q))
      .map(([n, s]) => `<li role="option" tabindex="0" data-n="${n}">${s.icon} ${s.title}</li>`).join("");
  }
  palQ.addEventListener("input", fillPalette);
  function pick(li) {
    pal.close();
    const r = view.getBoundingClientRect(), n = add(li.dataset.n, (r.width / 2 - V.x) / V.k, (r.height / 2 - V.y) / V.k);
    render();
    if (palFrom) { const p = (FLOW.STEPS[n.step].in || [])[0]; if (p) connect(palFrom.id, palFrom.port, n.id, p.id); }
    els.get(n.id).focus();
  }
  palL.addEventListener("click", ev => { const li = ev.target.closest("li"); if (li) pick(li); });
  palL.addEventListener("keydown", ev => { if (ev.key === "Enter" && ev.target.closest("li")) pick(ev.target.closest("li")); });
  palQ.addEventListener("keydown", ev => { if (ev.key === "Enter" && palL.firstElementChild) pick(palL.firstElementChild); });

  // ---- toolbar, save/load, stress ----
  $("#b-add").onclick = () => openPalette(null);
  $("#b-run").onclick = run;
  $("#b-zin").onclick = () => { const r = view.getBoundingClientRect(); zoomAt(r.left + r.width / 2, r.top + r.height / 2, 1.2); };
  $("#b-zout").onclick = () => { const r = view.getBoundingClientRect(); zoomAt(r.left + r.width / 2, r.top + r.height / 2, 1 / 1.2); };
  $("#b-fit").onclick = fit;
  $("#b-save").onclick = () => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([JSON.stringify(FLOW.save(G, V), null, 1)], { type: "application/json" }));
    a.download = "workflow.mzflow.json"; a.click(); URL.revokeObjectURL(a.href); say("Saved.");
  };
  $("#b-open").onchange = async ev => {
    try { openDoc(JSON.parse(await ev.target.files[0].text())); } catch (e) { say((TXT[e.message] || (() => "This file cannot be read."))()); }
  };
  function openDoc(doc) {
    const r = FLOW.load(doc);
    G = r.g; seq = G.nodes.length + 100; status.clear(); cache.clear(); render();
    if (r.view) { V = r.view; applyView(); } else fit();
    say(`Opened: ${G.nodes.length} steps.`);
  }
  const CHAIN = ["import.raw", "index.mzidx", "features.detect", "features.align", "export.csv"];
  function chain(x0, y0) {
    const ns = CHAIN.map((s, i) => add(s, x0 + 230 * i, y0 + (i % 2) * 30));
    for (let i = 1; i < ns.length; i++) G.edges.push({ from: ns[i - 1].id, fp: FLOW.STEPS[CHAIN[i - 1]].out[0].id, to: ns[i].id, tp: FLOW.STEPS[CHAIN[i]].in[0].id });
  }
  function stress(n) {
    G = { nodes: [], edges: [] }; seq = 0; status.clear(); cache.clear();
    start();
    for (let i = 0; i < Math.ceil(n / CHAIN.length); i++) chain(40 + (i % 4) * 1200, 420 + Math.floor(i / 4) * 170);
    render(); fit();
  }
  $("#b-stress").onchange = ev => stress(Number(ev.target.value));
  function start() {
    const im = add("import.raw", 40, 120), ix = add("features.detect", 560, 140), al = add("features.align", 820, 110), ex = add("export.csv", 1080, 130);
    const mol = add("structure.draw", 820, 300), tp = add("tp.link", 1080, 280);
    G.edges.push({ from: ix.id, fp: "peaks", to: al.id, tp: "peaks" }, { from: al.id, fp: "table", to: ex.id, tp: "table" },
      { from: al.id, fp: "table", to: tp.id, tp: "table" }, { from: mol.id, fp: "mol", to: tp.id, tp: "parent" });
    return im;
  }

  // Frame recorder for docs/studi/flow_grafica/misura.py: intervals between animation frames while a gesture runs.
  const rec = { on: false, t: [], last: 0 };
  function loop(now) { if (rec.on) { rec.t.push(now - rec.last); rec.last = now; requestAnimationFrame(loop); } }
  window.FLOWPROTO = {
    stress, run, openDoc, save: () => FLOW.save(G, V), count: () => ({ nodes: G.nodes.length, edges: G.edges.length }),
    recStart() { rec.on = true; rec.t = []; rec.last = performance.now(); requestAnimationFrame(loop); },
    recStop() { rec.on = false; const t = rec.t.slice(1).sort((a, b) => a - b), q = f => t[Math.min(t.length - 1, Math.floor(f * t.length))] || 0;
      return { frames: t.length, p50: q(0.5), p95: q(0.95), max: t[t.length - 1] || 0, over50: t.filter(x => x > 50).length }; },
  };
  start(); render(); fit();
  say("Import is not linked: drag from its “raw” output to the “index” input of Detect features.");
})();
