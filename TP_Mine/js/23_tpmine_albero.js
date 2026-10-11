// TPMINE-PRIVATE  mzFinder: fragmentation tree of the parent as an SVG (no libraries). Data: api frag_tree (LR "supposed" or HR MSn tree) and
// frag_tree_compare (nodes found in a product). Used by 20_tpmine.js (LR detail of the parent and of a product) and 21_tpmine_hr.js (results card).
(() => {
  "use strict";
  const CSS = `
.alb{--alb-1:#0072B2;--alb-2:#E69F00;--alb-3:#009E73;--alb-4:#CC79A7;--alb-5:#56B4E9;--alb-same:#009E73;--alb-shift:#D55E00}
.alb .alb-bar{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin:4px 0}.alb .alb-leg{font-size:11px;color:var(--muted)}
.alb .alb-wrap{position:relative;border:1px solid var(--line);border-radius:6px;background:var(--panel);overflow:hidden}
.alb svg.alb-svg{display:block;width:100%;height:min(62vh,520px);touch-action:none;cursor:grab}.alb svg.alb-svg:active{cursor:grabbing}
.alb .nd{cursor:pointer}.alb .nd:focus{outline:none}.alb .nd.on rect.f,.alb .nd:focus rect.f{stroke:var(--accent);stroke-width:3}
.alb .nd.dim{opacity:.4}
.alb .alb-pop{position:absolute;z-index:3;pointer-events:none;background:var(--panel);border:1px solid var(--line);border-radius:6px;padding:4px 6px;font-size:11px;box-shadow:0 2px 8px rgba(0,0,0,.18)}
.alb .alb-det{margin-top:8px}.alb .alb-det figure{display:inline-block;margin:0 10px 8px 0;vertical-align:top}.alb .alb-det figcaption{font-size:11px;color:var(--muted);max-width:300px;overflow-wrap:anywhere}
.alb .alb-det table{width:auto}.alb .alb-note{border-left:3px solid var(--alb-2);background:var(--sel);padding:4px 8px;font-size:12px;margin:4px 0}
@media (prefers-reduced-motion:no-preference){.alb .nd rect.b{transition:stroke-width .12s}}`;

  const NW = 118, NH = 50, HS = 138, VS = 96, PAD = 20;
  let OCLp = null, last = null;
  const ocl = () => (OCLp = OCLp || import(new URL("vendor/openchemlib.js", QTOOLS.ctx.base).href));
  const molSvg = new Map();
  const structure = async (smi, w, h) => {
    const k = smi + "|" + w + "x" + h;
    if (!molSvg.has(k)) molSvg.set(k, ocl().then(O => O.Molecule.fromSmiles(smi).toSVG(w, h)).catch(() => ""));
    return molSvg.get(k);
  };

  // ---- layout: depth from the root, leaves in order, every parent centred over its children
  function layout(data) {
    const by = new Map(data.nodes.map(n => [n.id, { ...n, kids: [], depth: 0, x: 0 }]));
    let root = null;
    for (const n of by.values()) { const p = n.parent != null && by.get(n.parent); if (p) p.kids.push(n); else root = root || n; }
    let leaf = 0, maxd = 0;
    const go = (n, d) => {
      n.depth = d; maxd = Math.max(maxd, d);
      n.kids.sort((a, b) => (b.mz ?? -1) - (a.mz ?? -1));
      if (!n.kids.length) { n.x = leaf++; return; }
      n.kids.forEach(k => go(k, d + 1));
      n.x = (n.kids[0].x + n.kids[n.kids.length - 1].x) / 2;
    };
    if (root) go(root, 0);
    const list = [...by.values()];
    list.forEach(n => { n.px = PAD + n.x * HS + NW / 2; n.py = PAD + n.depth * VS + NH / 2; });
    return { root, list, by, W: PAD * 2 + Math.max(leaf, 1) * HS - (HS - NW), H: PAD * 2 + (maxd + 1) * VS - (VS - NH) };
  }

  function mount(el, h, opts = {}) {
    if (!document.getElementById("tp-alb-css")) { const s = document.createElement("style"); s.id = "tp-alb-css"; s.textContent = CSS; document.head.appendChild(s); }
    const { esc, call } = h;
    el.classList.add("alb");
    el.innerHTML = `<div class="mut">${opts.live ? "Waiting for the first node..." : "Computing the fragmentation tree..."}</div>`;
    const inst = { marks: {}, compareId: null, compare: null };
    let L = null, data = null, sel = null, view = { k: 1, x: 0, y: 0 };

    const css = n => getComputedStyle(el).getPropertyValue(n).trim() || "#888";
    const lvCol = lv => css("--alb-" + Math.min(Math.max(lv, 1), 5));
    const fmtMz = v => v == null ? "?" : (+v).toFixed(4);
    const label = n => `MS${n.level}${n.kind === "frag" ? " fragment" : " precursor"} m/z ${fmtMz(n.mz)}${n.formula ? ", " + n.formula : ""}${n.rel != null ? ", " + n.rel + " percent" : ""}${n.loss ? ", loss of " + n.loss + " from the parent" : ""}${n.ghost ? ", ghost (precursor not isolated)" : ""}${inst.marks[n.id] ? ", " + (inst.marks[n.id].state === "same" ? "also in the product" : "in the product shifted by " + inst.marks[n.id].delta) : ""}`;

    function drawing() {
      const ink = css("--ink"), mut = css("--muted"), pan = css("--panel"), line = css("--line");
      const cmp = inst.compareId != null;
      const edges = L.list.filter(n => n.parent != null && L.by.get(n.parent)).map(n => {
        const p = L.by.get(n.parent), my = (p.py + NH / 2 + n.py - NH / 2) / 2;
        return `<path d="M${p.px} ${p.py + NH / 2}V${my}H${n.px}V${n.py - NH / 2}" fill="none" stroke="${mut}" stroke-width="1.2"/>`
          + (n.loss ? `<text x="${n.px}" y="${my - 3}" text-anchor="middle" font-size="11" fill="${ink}" stroke="${pan}" stroke-width="4" paint-order="stroke" aria-hidden="true">\u2212${esc(n.loss)}${n.alternatives && n.alternatives.length > 1 ? " +" + (n.alternatives.length - 1) : ""}</text>` : "");
      }).join("");
      const nodes = L.list.map(n => {
        const mk = inst.marks[n.id], col = lvCol(n.level), sw = (1.2 + 4 * Math.min(n.rel ?? 20, 100) / 100).toFixed(1);
        const fill = mk ? css(mk.state === "same" ? "--alb-same" : "--alb-shift") : pan;
        const glyph = mk ? (mk.state === "same" ? "\u2713" : "\u0394") : "";
        return `<g class="nd${cmp && !mk ? " dim" : ""}${sel === n.id ? " on" : ""}" data-id="${esc(n.id)}" role="treeitem" aria-level="${n.depth + 1}" tabindex="${sel === n.id ? 0 : -1}" aria-label="${esc(label(n))}" transform="translate(${n.px - NW / 2} ${n.py - NH / 2})">
          <rect class="f" width="${NW}" height="${NH}" rx="6" fill="${pan}" stroke="none"/>${mk ? `<rect width="${NW}" height="${NH}" rx="6" fill="${fill}" fill-opacity=".22" stroke="none"/>` : ""}
          <rect class="b" width="${NW}" height="${NH}" rx="6" fill="none" stroke="${col}" stroke-width="${sw}"${n.ghost ? ' stroke-dasharray="5 3"' : ""}/>
          <text x="${NW / 2}" y="16" text-anchor="middle" font-size="12" font-weight="700" fill="${ink}">${fmtMz(n.mz)}</text>
          <text x="${NW / 2}" y="30" text-anchor="middle" font-size="11" fill="${ink}">${esc(n.formula || "formula ?")}</text>
          <text x="${NW / 2}" y="44" text-anchor="middle" font-size="10" fill="${mut}">${n.rel != null ? n.rel + " %" : ""}${n.ghost ? " \u00b7 ghost" : ""}${n.kind === "ion" && n.level > 1 ? " \u00b7 MS" + n.level : ""}</text>
          ${glyph ? `<text x="${NW - 7}" y="13" text-anchor="end" font-size="13" font-weight="700" fill="${ink}" aria-hidden="true">${glyph}</text>` : ""}</g>`;
      }).join("");
      return `<g class="vp" transform="translate(${view.x} ${view.y}) scale(${view.k})">${edges}${nodes}</g>`;
    }
    function redraw() { const g = el.querySelector("svg.alb-svg"); if (g) { g.innerHTML = drawing(); bindNodes(); } }

    function bindNodes() {
      el.querySelectorAll(".nd").forEach(g => {
        const id = g.dataset.id;
        g.onclick = ev => { ev.stopPropagation(); choose(id, false); };
        g.onfocus = () => pop(id, g); g.onblur = () => pop(null);
        g.onmouseenter = () => pop(id, g); g.onmouseleave = () => pop(null);
      });
    }
    async function pop(id, g) {
      const P = el.querySelector(".alb-pop"); if (!P) return;
      if (id == null) { P.hidden = true; return; }
      const n = L.by.get(id), smi = n.smiles && n.smiles[0];
      if (!smi) { P.hidden = true; return; }
      const svg = await structure(smi.smiles, 150, 100);
      if (!svg) { P.hidden = true; return; }
      const r = g.getBoundingClientRect(), w = el.querySelector(".alb-wrap").getBoundingClientRect();
      P.innerHTML = `${svg}<div>${esc(n.formula || "")} \u00b7 weight ${smi.score}</div>`;
      P.style.left = Math.max(4, Math.min(w.width - 170, r.left - w.left + r.width / 2 - 80)) + "px";
      P.style.top = Math.max(4, r.bottom - w.top + 4 > w.height - 140 ? r.top - w.top - 140 : r.bottom - w.top + 4) + "px";
      P.hidden = false;
    }

    async function details(n) {
      const D = el.querySelector(".alb-det");
      const figs = await Promise.all((n.smiles || []).map(async s => `<figure>${await structure(s.smiles, 190, 130) || ""}<figcaption>weight ${s.score}<br><code>${esc(s.smiles)}</code></figcaption></figure>`));
      const mk = inst.marks[n.id];
      D.innerHTML = `<h4 style="margin:4px 0">${esc(n.formula || "formula ?")} <span class="mut">${n.kind === "frag" ? "fragment" : "isolated precursor"} (MS<sup>${n.level}</sup>) \u00b7 <i>m/z</i> ${fmtMz(n.mz)}</span></h4>
        <div class="kv"><b>Intensity</b><span>${n.rel != null ? n.rel + " % of the base peak" : "-"}</span>
        <b>Loss from the parent</b><span>${n.loss ? "\u2212" + esc(n.loss) + (n.loss_mass ? ` (${n.loss_mass} Da)` : "") : "-"}</span>
        ${n.ppm != null ? `<b>Error</b><span>${n.ppm} ppm</span>` : ""}${n.n_scans ? `<b>Scans</b><span>${n.n_scans}${n.ce != null ? " \u00b7 CE " + n.ce : ""}</span>` : ""}
        ${mk ? `<b>In the product</b><span>${mk.state === "same" ? "\u2713 the same fragment" : "\u0394 shifted by " + esc(mk.delta)}</span>` : ""}</div>
        ${n.alternatives && n.alternatives.length ? `<table style="margin-top:6px"><tr><th>Alternative formula</th><th>Loss</th><th>Error (Da)</th><th>From</th></tr>${n.alternatives.map(a => `<tr><td>${esc(a.formula || "-")}</td><td>\u2212${esc(a.loss)}</td><td class="n">${a.err}</td><td>${a.source === "loss" ? "common neutral loss" : "sub-formula"}</td></tr>`).join("")}</table>` : ""}
        <div style="margin-top:6px">${figs.join("") || '<span class="mut">No structure proposed for this node (the SMILES of the parent is needed).</span>'}</div>`;
    }
    function choose(id, focus) {
      sel = id; redraw();
      const n = L.by.get(id); details(n);
      const g = el.querySelector(`.nd[data-id="${CSS_escape(id)}"]`);
      if (g) { g.focus({ preventScroll: true }); if (focus) reveal(n); }
    }
    const CSS_escape = s => (window.CSS && window.CSS.escape) ? window.CSS.escape(s) : String(s).replace(/["\\#]/g, "\\$&");
    function reveal(n) {            // pan so that the node is inside the view
      const sv = el.querySelector("svg.alb-svg"), b = sv.getBoundingClientRect(), s = Math.min(b.width / L.W, b.height / L.H);
      const px = (n.px * view.k + view.x) * s, py = (n.py * view.k + view.y) * s;
      let dx = 0, dy = 0;
      if (px < 60) dx = (60 - px) / s; else if (px > b.width - 60) dx = (b.width - 60 - px) / s;
      if (py < 40) dy = (40 - py) / s; else if (py > b.height - 40) dy = (b.height - 40 - py) / s;
      if (dx || dy) { view.x += dx; view.y += dy; applyView(); }
    }
    function applyView() { const g = el.querySelector("g.vp"); if (g) g.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${view.k})`); }
    function zoom(f, cx, cy) {
      const k = Math.min(4, Math.max(0.3, view.k * f)), r = k / view.k;
      view.x = cx - (cx - view.x) * r; view.y = cy - (cy - view.y) * r; view.k = k; applyView();
    }

    function keys(ev) {
      const k = ev.key;
      if (k === "+" || k === "=") { zoom(1.25, L.W / 2, L.H / 2); return ev.preventDefault(); }
      if (k === "-") { zoom(0.8, L.W / 2, L.H / 2); return ev.preventDefault(); }
      if (k === "0") { view = { k: 1, x: 0, y: 0 }; applyView(); return ev.preventDefault(); }
      if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", " "].includes(k)) return;
      ev.preventDefault(); ev.stopPropagation();          // the page moves between candidates with the same arrow keys
      const n = L.by.get(sel || (L.root && L.root.id)); if (!n) return;
      if (k === "Enter" || k === " ") return details(n);
      let to = null;
      if (k === "ArrowUp") to = n.parent != null && L.by.get(n.parent);
      else if (k === "ArrowDown") to = n.kids[0];
      else { const row = L.list.filter(m => m.depth === n.depth).sort((a, b) => a.x - b.x), i = row.indexOf(n); to = row[i + (k === "ArrowRight" ? 1 : -1)]; }
      if (to) choose(to.id, true);
    }

    function download(name, blob) { const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000); }
    function svgText() {
      const g = el.querySelector("svg.alb-svg").cloneNode(true);
      g.querySelectorAll(".nd").forEach(x => { x.removeAttribute("tabindex"); x.removeAttribute("class"); });
      g.querySelector("g.vp").setAttribute("transform", "");
      g.setAttribute("xmlns", "http://www.w3.org/2000/svg"); g.setAttribute("viewBox", `0 0 ${L.W} ${L.H}`); g.setAttribute("width", L.W); g.setAttribute("height", L.H);
      g.setAttribute("font-family", "system-ui,Arial,sans-serif"); g.removeAttribute("style"); g.removeAttribute("class");
      const bg = document.createElementNS("http://www.w3.org/2000/svg", "rect"); bg.setAttribute("width", L.W); bg.setAttribute("height", L.H); bg.setAttribute("fill", css("--panel"));
      g.insertBefore(bg, g.firstChild);
      return new XMLSerializer().serializeToString(g);
    }
    function exportSvg() { download("mzFinder_fragmentation_tree.svg", new Blob([svgText()], { type: "image/svg+xml" })); }
    function exportPng() {
      const url = URL.createObjectURL(new Blob([svgText()], { type: "image/svg+xml" })), im = new Image();
      im.onload = () => {
        const c = document.createElement("canvas"), s = 2; c.width = L.W * s; c.height = L.H * s;
        const x = c.getContext("2d"); x.scale(s, s); x.drawImage(im, 0, 0); URL.revokeObjectURL(url);
        c.toBlob(b => download("mzFinder_fragmentation_tree.png", b), "image/png");
      };
      im.src = url;
    }

    function shell() {
      const hasCmp = !!opts.compareId;
      el.innerHTML = `<div class="alb-bar"><b>Fragmentation tree of the parent</b>
        <button class="alb-b" data-a="in" aria-label="Zoom in" title="Zoom in (+)">+</button><button class="alb-b" data-a="out" aria-label="Zoom out" title="Zoom out (-)">\u2212</button><button class="alb-b" data-a="fit" aria-label="Fit the tree to the window" title="Fit (0)">Fit</button>
        <button class="alb-b" data-a="svg" aria-label="Export the tree as SVG">SVG</button><button class="alb-b" data-a="png" aria-label="Export the tree as PNG">PNG</button>
        ${hasCmp ? `<button class="alb-b" id="alb-cmp" data-a="cmp" aria-pressed="false">Compare with parent tree</button>` : ""}
        <span class="alb-leg" id="alb-leg"></span></div>
        ${data.supposed ? `<div class="alb-note">${esc(data.note)}: every formula and loss within \u00b10.5 Da is kept as an alternative (see the details of a node).</div>` : ""}
        <div class="alb-wrap"><svg class="alb-svg" viewBox="0 0 ${L.W} ${L.H}" role="tree" aria-label="Fragmentation tree of the parent: ${data.nodes.length} nodes. Arrow keys move between nodes, Enter shows the details, plus and minus zoom."><title>Fragmentation tree of the parent</title>${drawing()}</svg><div class="alb-pop" hidden></div></div>
        <div class="alb-det" aria-live="polite"><span class="mut">Click a node (or move with the arrow keys and press Enter) for the structures, the alternatives and the loss.</span></div>`;
      const sv = el.querySelector("svg.alb-svg");
      bindNodes();
      sv.addEventListener("keydown", keys);
      sv.addEventListener("wheel", ev => { ev.preventDefault(); const b = sv.getBoundingClientRect(), s = Math.min(b.width / L.W, b.height / L.H); zoom(ev.deltaY < 0 ? 1.15 : 1 / 1.15, (ev.clientX - b.left) / s, (ev.clientY - b.top) / s); }, { passive: false });
      let drag = null;
      sv.addEventListener("pointerdown", ev => { if (ev.target.closest(".nd")) return; drag = { x: ev.clientX, y: ev.clientY, vx: view.x, vy: view.y }; sv.setPointerCapture(ev.pointerId); });
      sv.addEventListener("pointermove", ev => { if (!drag) return; const b = sv.getBoundingClientRect(), s = Math.min(b.width / L.W, b.height / L.H); view.x = drag.vx + (ev.clientX - drag.x) / s; view.y = drag.vy + (ev.clientY - drag.y) / s; applyView(); });
      sv.addEventListener("pointerup", () => { drag = null; });
      el.querySelectorAll(".alb-b").forEach(b => b.onclick = () => {
        const a = b.dataset.a;
        if (a === "in") zoom(1.25, L.W / 2, L.H / 2); else if (a === "out") zoom(0.8, L.W / 2, L.H / 2);
        else if (a === "fit") { view = { k: 1, x: 0, y: 0 }; applyView(); } else if (a === "svg") exportSvg(); else if (a === "png") exportPng();
        else if (a === "cmp") inst.compare(inst.compareId != null ? null : (typeof opts.compareId === "function" ? opts.compareId() : opts.compareId));
      });
    }

    inst.compare = async cid => {
      if (cid == null && inst.compareId == null && el.querySelector("#alb-cmp")) { const l = el.querySelector("#alb-leg"); if (l) l.textContent = "Select a candidate with MS2 first."; return; }
      const leg = el.querySelector("#alb-leg"), btn = el.querySelector("#alb-cmp");
      if (cid == null) { inst.marks = {}; inst.compareId = null; if (leg) leg.textContent = ""; if (btn) { btn.setAttribute("aria-pressed", "false"); btn.textContent = "Compare with parent tree"; } redraw(); return; }
      if (leg) leg.textContent = "Comparing...";
      try {
        const c = await call("frag_tree_compare", cid);
        if (!c.ok) { if (leg) leg.textContent = c.note; return; }
        inst.marks = c.marks; inst.compareId = cid;
        if (btn) { btn.setAttribute("aria-pressed", "true"); btn.textContent = "Clear the comparison"; }
        if (leg) leg.textContent = `\u2713 same as in the parent: ${c.n_same} \u00b7 \u0394 shifted by the change: ${c.n_shifted} \u00b7 dimmed: not found in the product (of ${c.n_nodes} nodes)`;
        redraw();
        if (sel) choose(sel, false);
      } catch (e) { if (leg) leg.textContent = String(e.message || e); }
    };

    // live mode (opts.live): no request; the page gives the nodes as they are built (24_tpmine_diretta.js) and the tree grows
    inst.live = d => {
      data = d; L = layout(d); sel = sel && L.by.get(sel) ? sel : (L.root && L.root.id);
      const sv = el.querySelector("svg.alb-svg");
      if (!sv) shell(); else { sv.setAttribute("viewBox", `0 0 ${L.W} ${L.H}`); redraw(); }
    };
    inst.ready = opts.live ? Promise.resolve(inst) : (async () => {
      try { data = await call("frag_tree"); } catch (e) { el.innerHTML = `<div class="err">${esc(e.message || e)}</div>`; return inst; }
      if (!data.ok) { el.innerHTML = `<div class="mut">Fragmentation tree: ${esc(data.note)}.</div>`; return inst; }
      L = layout(data);
      sel = L.root && L.root.id;
      shell();
      
      return inst;
    })();
    if (!opts.live) last = inst;
    return inst;
  }

  window.TPALBERO = { mount, compare: id => last && last.compare && last.compare(id), last: () => last };
})();
