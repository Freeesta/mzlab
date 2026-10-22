// @ts-check
// The styles of the «Fragmentation graph» (class ig-*): added once to the page by the tab and by the Flow view of the same graph.

const CSS = `.ig-field{align-items:flex-end;gap:8px}.ig-field input[type=text]{min-width:14ch}.ig-set summary{cursor:pointer;min-height:24px}.ig-set input[type=number]{width:7ch}.ig-bar{display:flex;gap:4px;align-items:center;flex-wrap:wrap;margin:6px 0}.ig-bar .bt{min-width:28px;min-height:28px}.ig-cut{min-height:28px}.ig-cuts{border-collapse:collapse}.ig-cuts th,.ig-cuts td{padding:2px 6px;text-align:left}
@media (pointer:coarse){.ig-bar .bt,.ig-use,.ig-cut{min-width:40px;min-height:40px}}
.ig-wrap{overflow:auto;border:1px solid var(--line);border-radius:6px;max-height:min(60vh,520px);background:var(--bg)}.ig-svg{display:block}
.ig-box{fill:var(--panel);stroke:var(--line);stroke-width:1.5}.ig-node{cursor:pointer;outline:none}.ig-node:focus-visible .ig-box,.ig-node.sel .ig-box{stroke:var(--accent);stroke-width:3}.ig-node.hot .ig-box{stroke:var(--accent2)}
.ig-node.sel .ig-box{fill:var(--sel)}.ig-mz{font:700 12px sans-serif;fill:var(--ink)}.ig-cand{font:11px sans-serif;fill:var(--muted)}.ig-cand.on{fill:var(--ink);font-weight:700}
.ig-arc{fill:none;stroke:var(--muted);opacity:.6}.ig-arc.on{stroke:var(--accent2);opacity:1}.ig-arc.iso{stroke-dasharray:0}.ig-headp{fill:var(--muted)}.ig-loss{font:10px sans-serif;fill:var(--muted)}.ig-amb{fill:var(--warn);stroke:var(--ink)}
.ig-cands{border-collapse:collapse;width:100%;font-size:12px}.ig-cands th,.ig-cands td{border-bottom:1px solid var(--line);padding:2px 6px;text-align:left}.ig-cands tr.on{background:var(--sel)}.ig-cands .num{text-align:right}
.ig-rule{font:10px sans-serif;fill:var(--ink)}.ig-weak{border:1px solid var(--line);border-radius:4px;padding:0 4px;font-weight:700}.ig-cal{color:var(--warn)}.ig-rl{margin-top:8px}.ig-src{display:inline-block;min-height:24px;min-width:24px}.ig-flt{display:inline-flex;align-items:center;gap:4px;min-height:24px}
.ig-warn{color:var(--warn)}.ig-unexp{color:var(--muted)}.ig-draw{border:1px solid var(--line);border-radius:6px;padding:6px;margin-top:8px;max-width:340px}.ig-pic svg{max-width:100%;height:auto}`;

/** Adds the styles to the page (once). */
export function addStyle() {
  if (document.getElementById("ig-css")) return;
  const st = document.createElement("style"); st.id = "ig-css"; st.textContent = CSS; document.head.appendChild(st);
}
