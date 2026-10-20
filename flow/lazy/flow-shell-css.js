// @ts-check
// Flow (administrator mode): the stylesheet of the canvas shell. Colours only from the page variables (plus the category colour passed as --cc); motion through the
// shared tokens, so it stops with prefers-reduced-motion and the accessibility setting.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.fl.flx{display:flex;flex-direction:column;gap:8px;min-width:0;padding:0}.flx *{box-sizing:border-box}
.flx .flx-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}.flx .flx-bar h2{margin:0;font-size:16px;margin-right:4px}
.flx .flx-seg{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden}.flx .flx-seg button{border:0;background:var(--panel);color:var(--ink);min-height:28px;padding:2px 12px;cursor:pointer}
.flx .flx-seg button[aria-pressed=true]{background:var(--accent);color:var(--bg)}.flx .flx-seg button:focus-visible{outline:2px solid var(--accent);outline-offset:-3px}
.flx .flx-run{background:var(--accent);color:var(--bg);border-color:var(--accent)}.flx .flx-run:hover:not(:disabled){background:var(--accent);filter:brightness(1.1)}
.flx .flx-tabs{display:flex;gap:4px;border-bottom:1px solid var(--line)}.flx .flx-tabs button{border:0;border-bottom:2px solid transparent;background:none;color:var(--muted);min-height:28px;padding:2px 12px;cursor:pointer}
.flx .flx-tabs button[aria-selected=true]{color:var(--ink);border-bottom-color:var(--accent);font-weight:600}.flx .flx-tabs button:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.flx .flx-body{width:100%;align-self:stretch;display:grid;grid-template-columns:minmax(0,1fr) 340px;gap:8px;align-items:start}.flx .flx-body.flx-adv{grid-template-columns:200px minmax(0,1fr) 340px}
.flx .fl-cv{position:relative;border:1px solid var(--line);border-radius:10px;background-color:var(--panel);background-image:radial-gradient(color-mix(in srgb,var(--line) 45%,transparent) 1px,transparent 1.5px);background-size:20px 20px;height:min(70vh,640px);min-height:340px;overflow:hidden}
.flx .fl-view{position:absolute;inset:0;overflow:hidden;cursor:grab;touch-action:none}.flx .fl-view.fl-pan{cursor:grabbing}.flx .fl-view:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.flx .fl-stage{position:absolute;left:0;top:0;transform-origin:0 0}.flx .fl-stage.fl-soft{transition:transform var(--t-med,180ms) var(--ease-out,ease-out)}
.flx .fl-lines{position:absolute;left:0;top:0;pointer-events:none;overflow:visible}.flx .fle{fill:none;stroke:var(--muted);stroke-width:2}.flx .fle.hot{stroke:var(--accent);stroke-width:3}
.flx .fln{position:absolute;width:220px;height:64px;display:flex;align-items:center;gap:8px;padding:6px 8px 6px 10px;border:1px solid var(--line);border-left:4px solid var(--cc);border-radius:10px;background:var(--panel);color:var(--ink);cursor:pointer;transition:border-color var(--t-fast,120ms)}
.flx .fln:hover{border-color:color-mix(in srgb,var(--accent) 60%,var(--line));border-left-color:var(--cc)}.flx .fln:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.flx .fln.fl-new{animation:flIn var(--t-med,180ms) var(--ease-out,ease-out) both}.flx .fln.sel{outline:2px solid var(--accent);box-shadow:inset 0 0 0 2px color-mix(in srgb,var(--accent) 18%,transparent)}.flx .fln.bad{border-color:var(--bad);border-left-color:var(--cc)}
.flx .fln-ic{flex:none;color:var(--cc);display:inline-flex}.flx .fln-tx{display:flex;flex-direction:column;min-width:0;flex:1}.flx .fln-t{font-weight:700;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.flx .fln-s{font-size:12px;color:var(--muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.flx .fln-chip{position:absolute;top:-9px;right:8px;display:inline-flex;gap:3px;align-items:center;font-size:12px;padding:0 6px;border-radius:10px;border:1px solid var(--line);background:var(--panel)}
.flx .fln-chip.ok{color:var(--ok)}.flx .fln-chip.bad{color:var(--bad);border-color:var(--bad)}
.flx .fln-next{position:absolute;right:-14px;top:50%;margin-top:-12px;width:24px;height:24px;border-radius:50%;border:1px solid var(--line);background:var(--panel);color:var(--ink);display:none;place-items:center;cursor:pointer;padding:0}
.flx .fln:hover .fln-next,.flx .fln.sel .fln-next,.flx .fln:focus-within .fln-next{display:grid}.flx .fln-next:hover{border-color:var(--accent);color:var(--accent)}.flx .fln-next:focus-visible{outline:2px solid var(--accent)}
.flx .fl-zoom{position:absolute;left:8px;bottom:8px;display:flex;gap:4px}
.flx .fl-insp,.flx .fl-insp-empty,.flx .flx-pal{border:1px solid var(--line);border-radius:10px;background:var(--panel);padding:10px;min-width:0}.flx .fl-insp-empty{color:var(--muted)}
.flx .fl-insp{border-left:4px solid var(--cc);max-height:70vh;overflow:auto;animation:flIn var(--t-med,180ms) var(--ease-out,ease-out) both}
.flx .fl-insp-hd{display:flex;gap:8px;align-items:center}.flx .fl-insp-hd h3{font-size:16px;flex:1}.flx .fl-insp-why{margin:6px 0}.flx .fl-ready{color:var(--ok);display:flex;gap:4px;align-items:center;margin:4px 0}
.flx .fl-needs{list-style:none;margin:6px 0;padding:0;display:flex;flex-direction:column;gap:6px}.flx .fl-needs li{display:flex;flex-wrap:wrap;gap:6px;align-items:center;color:var(--ink);padding:6px 8px;border-radius:8px;background:color-mix(in srgb,var(--warn,#E69F00) 18%,var(--panel));border:1px solid var(--warn,#E69F00)}
.flx .fl-link{cursor:pointer;color:var(--accent)}.flx h4{margin:10px 0 2px;font-size:13px}
.flx .flx-pop{position:absolute;z-index:20;min-width:260px;max-width:340px;border:1px solid var(--line);border-radius:10px;background:var(--panel);box-shadow:0 6px 24px color-mix(in srgb,var(--ink) 25%,transparent);padding:4px;animation:flIn var(--t-fast,120ms) var(--ease-out,ease-out) both}
.flx .flx-pop button{display:flex;flex-direction:column;align-items:flex-start;width:100%;text-align:left;min-height:32px;padding:6px 8px;border:0;border-radius:6px;background:none;color:var(--ink);cursor:pointer}
.flx .flx-pop button:hover,.flx .flx-pop button:focus-visible{background:var(--sel);outline:none}.flx .flx-pop small{color:var(--muted)}.flx .flx-pop .on{opacity:.6}
.flx .flx-tpl{position:absolute;left:50%;bottom:12px;transform:translateX(-50%);width:min(92%,640px);max-height:64%;overflow:auto;border:1px solid var(--line);border-radius:12px;background:var(--panel);padding:12px;z-index:5;display:none}
.flx .flx-tpl.show{display:block}.flx .flx-tpl h3{margin:0 0 8px}.flx .flx-tpls{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:8px}
.flx .flx-tc{display:flex;flex-direction:column;gap:4px;text-align:left;border:1px solid var(--line);border-radius:10px;background:var(--bg);color:var(--ink);padding:8px;cursor:pointer}.flx .flx-tc:hover{border-color:var(--accent)}.flx .flx-tc:focus-visible{outline:2px solid var(--accent)}
.flx .flx-tc .flx-pre{display:flex;gap:3px;align-items:center}.flx .flx-pre i{display:block;width:16px;height:8px;border-radius:3px;background:var(--cc)}.flx .flx-pre b{display:block;width:8px;height:2px;background:var(--muted)}
.flx .flx-tc small{color:var(--muted)}.flx .flx-toast{display:flex;gap:8px;align-items:center;min-height:28px;color:var(--muted)}.flx .flx-plan{white-space:pre-wrap;font-family:monospace;font-size:12px;background:var(--soft);padding:8px;border-radius:8px;max-height:70vh;overflow:auto}
.flx.flx-simple .flx-adv-only{display:none}
@media (max-width:900px){.flx .flx-body,.flx .flx-body.flx-adv{grid-template-columns:minmax(0,1fr)}.flx .fl-insp{max-height:none}}
@media (pointer:coarse){.flx .fln-next{width:40px;height:40px;right:-20px;margin-top:-20px}.flx .flx-seg button,.flx .flx-tabs button{min-height:40px}}
@media (prefers-reduced-motion:reduce){.flx .fln,.flx .fl-insp,.flx .flx-pop{animation:none}.flx .fl-stage.fl-soft,.flx .fln{transition:none}}
:root[data-a11y-reduce] .flx .fln,:root[data-a11y-reduce] .flx .fl-insp,:root[data-a11y-reduce] .flx .flx-pop{animation:none}:root[data-a11y-reduce] .flx .fl-stage.fl-soft,:root[data-a11y-reduce] .flx .fln{transition:none}`;
  F.injectShellCss = () => {
    if (document.getElementById("flx-css")) return;
    const s = document.createElement("style"); s.id = "flx-css"; s.textContent = CSS; document.head.append(s);
  };
})();
