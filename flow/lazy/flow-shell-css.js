// @ts-check
// Flow (administrator mode): the stylesheet of the canvas shell. Colours only from the page variables (plus the category colour passed as --cc); motion through the
// shared tokens, so it stops with prefers-reduced-motion and the accessibility setting.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.flw.flx{display:flex;flex-direction:column;gap:8px;min-width:0;padding:0}.flx *{box-sizing:border-box}
.flx .flx-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}.flx .flx-bar h2{margin:0;font-size:16px;margin-right:4px}
.flx .flx-seg{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden}.flx .flx-seg button{border:0;background:var(--panel);color:var(--ink);min-height:28px;padding:2px 12px;cursor:pointer}
.flx .flx-bar .fl-st{margin-left:auto}.flx .flx-bar .fl-b{min-height:28px;min-width:28px}.flx .flx-bar .fl-st button{min-height:24px}.flx .flx-live-ctl[hidden],.flx .flx-live-ctl button[hidden]{display:none}.flx .flx-seg button[aria-pressed=true]{background:var(--accent);color:var(--on-accent)}.flx .flx-seg button:focus-visible{outline:2px solid var(--accent);outline-offset:-3px}
.flx button[aria-busy=true]{cursor:progress;opacity:.6}@media (prefers-reduced-motion:no-preference){.flx button[aria-busy=true]{animation:flBusy 1s ease-in-out infinite alternate}}@keyframes flBusy{to{opacity:.35}}
.flx .flx-run{background:var(--accent);color:var(--on-accent);border-color:var(--accent)}.flx .flx-run:hover:not(:disabled){background:var(--accent);filter:brightness(1.1)}
.flx .flx-body{--insp:50%;width:100%;align-self:stretch;display:grid;grid-template-columns:minmax(0,1fr) 24px var(--insp);gap:4px;align-items:start}.flx .flx-body.flx-adv{grid-template-columns:200px minmax(0,1fr) 24px var(--insp)}
.flx .flx-split{align-self:stretch;min-height:120px;cursor:col-resize;background:linear-gradient(var(--line),var(--line)) center/4px 100% no-repeat;touch-action:none}.flx .flx-split:hover,.flx .flx-split:focus-visible,.flx .flx-split.drag{background:linear-gradient(var(--accent),var(--accent)) center/4px 100% no-repeat;outline:none}
.flx .fl-cv{position:relative;border:1px solid var(--line);border-radius:10px;background-color:var(--panel);background-image:radial-gradient(color-mix(in srgb,var(--line) 45%,transparent) 1px,transparent 1.5px);background-size:20px 20px;height:min(70vh,640px);min-height:340px;overflow:hidden}
.flx .fl-view{position:absolute;inset:0;overflow:hidden;cursor:grab;touch-action:none}.flx .fl-view.fl-pan{cursor:grabbing}.flx .fl-view:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.flx .fl-stage{position:absolute;left:0;top:0;transform-origin:0 0}.flx .fl-stage.fl-soft{transition:transform var(--t-med,180ms) var(--ease-out,ease-out)}
.flx .fl-lines{position:absolute;left:0;top:0;pointer-events:none;overflow:visible}.flx .fle{fill:none;stroke:var(--muted);stroke-width:2}.flx .fle.hot{stroke:var(--accent);stroke-width:3}
.flx .fln{position:absolute;width:220px;height:80px;display:flex;flex-direction:column;justify-content:center;gap:4px;padding:6px 8px 6px 10px;border:1px solid var(--line);border-left:4px solid var(--cc);border-radius:10px;background:var(--panel);color:var(--ink);cursor:pointer;transition:border-color var(--t-fast,120ms)}
.flx .fln:hover{border-color:color-mix(in srgb,var(--accent) 60%,var(--line));border-left-color:var(--cc)}.flx .fln:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.flx .fln.fl-new{animation:flIn var(--t-med,180ms) var(--ease-out,ease-out) both}.flx .fln.sel{outline:2px solid var(--accent);box-shadow:inset 0 0 0 2px color-mix(in srgb,var(--accent) 18%,transparent)}.flx .fln.bad{border-color:var(--bad);border-left-color:var(--cc)}
.flx .fln-top{display:flex;align-items:center;gap:8px;min-width:0}.flx .fln-ic{flex:none;color:var(--cc);display:inline-flex}.flx .fln-tx{display:flex;flex-direction:column;min-width:0;flex:1}.flx .fln-t{font-weight:700;font-size:14px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.flx .fln-s{font-size:12px;color:var(--muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.flx .fln-chip{position:absolute;top:-9px;right:8px;display:inline-flex;gap:3px;align-items:center;font-size:12px;padding:0 6px;border-radius:10px;border:1px solid var(--line);background:var(--panel)}
.flx .fln-chip.ok{color:var(--ok)}.flx .fln-chip.bad{color:var(--bad);border-color:var(--bad)}.flx .fln-chip.warn{color:var(--warn);border-color:var(--warn)}
.flx .fln-next{position:absolute;right:-14px;top:50%;margin-top:-12px;width:24px;height:24px;border-radius:50%;border:1px solid var(--line);background:var(--panel);color:var(--ink);display:none;place-items:center;cursor:pointer;padding:0}
.flx .fln:hover .fln-next,.flx .fln.sel .fln-next,.flx .fln:focus-within .fln-next{display:grid}.flx .fln-next:hover{border-color:var(--accent);color:var(--accent)}.flx .fln-next:focus-visible{outline:2px solid var(--accent)}
.flx .fl-zoom{position:absolute;left:8px;bottom:8px;display:flex;gap:4px}
.flx .fl-insp,.flx .fl-insp-empty,.flx .flx-pal{border:1px solid var(--line);border-radius:10px;background:var(--panel);padding:10px;min-width:0}.flx .fl-insp-empty{color:var(--muted)}
.flx .fl-insp{border-left:4px solid var(--cc);max-height:70vh;overflow:auto;animation:flIn var(--t-med,180ms) var(--ease-out,ease-out) both}
.flx .fl-insp-hd{display:flex;gap:8px;align-items:center}.flx .fl-insp-hd h3{font-size:16px;flex:1}.flx .fl-insp-why{margin:6px 0}.flx .fl-ready{color:var(--ok);display:flex;gap:4px;align-items:center;margin:4px 0}
.flx .fl-needs,.flx .flx-needs{list-style:none;margin:6px 0;padding:0;display:flex;flex-direction:column;gap:6px}.flx .fl-needs li,.flx .flx-needs li{display:flex;flex-wrap:wrap;gap:6px;align-items:center;color:var(--ink);padding:6px 8px;border-radius:8px;background:color-mix(in srgb,var(--warn,#E69F00) 18%,var(--panel));border:1px solid var(--warn,#E69F00)}
.flx .fl-link{cursor:pointer;color:var(--accent)}.flx h4{margin:10px 0 2px;font-size:13px}
.flx .flx-pop{position:absolute;z-index:20;min-width:260px;max-width:340px;border:1px solid var(--line);border-radius:10px;background:var(--panel);box-shadow:0 6px 24px color-mix(in srgb,var(--ink) 25%,transparent);padding:4px;animation:flIn var(--t-fast,120ms) var(--ease-out,ease-out) both}
.flx .flx-pop button{display:flex;flex-direction:column;align-items:flex-start;width:100%;text-align:left;min-height:32px;padding:6px 8px;border:0;border-radius:6px;background:none;color:var(--ink);cursor:pointer}
.flx .flx-pop button:hover,.flx .flx-pop button:focus-visible{background:var(--sel);outline:none}.flx .flx-pop small{color:var(--muted)}.flx .flx-pop .on{opacity:.6}
.flx .flx-tpl{position:absolute;left:50%;bottom:12px;transform:translateX(-50%);width:min(92%,640px);max-height:64%;overflow:auto;border:1px solid var(--line);border-radius:12px;background:var(--panel);padding:12px;z-index:5;display:none}
.flx .flx-tpl.show{display:block}.flx .flx-tpl h3{margin:0 0 8px}.flx .flx-tpls{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:8px}
.flx .flx-tc{display:flex;flex-direction:column;gap:4px;text-align:left;border:1px solid var(--line);border-radius:10px;background:var(--bg);color:var(--ink);padding:8px;cursor:pointer}.flx .flx-tc:hover{border-color:var(--accent)}.flx .flx-tc:focus-visible{outline:2px solid var(--accent)}
.flx .flx-tc .flx-pre{display:flex;gap:3px;align-items:center}.flx .flx-pre i{display:block;width:16px;height:8px;border-radius:3px;background:var(--cc)}.flx .flx-pre b{display:block;width:8px;height:2px;background:var(--muted)}
.flx .flx-more{margin-top:8px}.flx .flx-more summary{cursor:pointer;min-height:40px;display:flex;align-items:center;color:var(--ink)}.flx .flx-more summary:focus-visible{outline:2px solid var(--accent)}.flx .flx-more .flx-tpls{margin-top:4px}
.flx .flx-tc small{color:var(--muted)}.flx .flx-tc .flx-help{display:block;margin-top:2px}.flx .flx-toast{display:flex;gap:8px;align-items:center;min-height:28px;color:var(--muted)}.flx .flx-plan{white-space:pre-wrap;font-family:monospace;font-size:12px;background:var(--soft);padding:8px;border-radius:8px;max-height:70vh;overflow:auto}
.flx.flx-simple .flx-adv-only{display:none}
.flx tr.flx-sel>td,.flx tr.flx-sel>th{outline:3px solid var(--accent);outline-offset:-3px}.flx tr.flx-member>td,.flx tr.flx-member>th{outline:2px dashed var(--accent);outline-offset:-2px}.flx tr[data-id][tabindex]{cursor:pointer}.flx tr[data-id][tabindex]:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}.flx .flx-smp{display:flex;flex-direction:column;gap:6px}.flx .flx-smp.flx-parent-box input[type=text],.flx .flx-smp.flx-parent-box select{max-width:none}.flx .flx-parent-box .molc{max-width:none}.flx .flx-smp input[type=text],.flx .flx-smp input[type=number],.flx .flx-smp select{min-height:28px;max-width:160px}.flx .flx-issues{margin:8px 0;padding:8px;border:1px solid var(--line);border-radius:8px;background:var(--panel)}.flx .flx-issues h3{margin:0 0 4px;font-size:14px}.flx .flx-q{display:inline-flex;align-items:center;justify-content:center;min-width:24px;min-height:24px;border-radius:50%;text-decoration:none;font-weight:700}.flx .flx-leg{display:flex;flex-wrap:wrap;gap:6px 14px;align-items:center;margin-top:8px;font-size:12px;color:var(--muted)}.flx .flx-leg-lv{display:inline-flex;flex-wrap:wrap;gap:4px;align-items:center}.flx .flx-leg .lvb.flx-auto{--lvc:var(--muted);background:none;border-style:dashed}
.flx .fl-results{display:flex;flex-direction:column;gap:6px;margin-top:10px}.flx .fl-results h4{margin:0}.flx .flx-res{margin-top:10px;border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:8px}
.flx .flx-res h3{flex:1;margin:0;overflow-wrap:anywhere}.flx .flx-tablewrap{max-height:50vh;overflow:auto;margin-top:6px}.flx .flx-table{border-collapse:collapse;font-size:12px}
.flx .flx-sticks{width:100%;max-width:640px;height:auto;display:block}.flx .flx-stick{stroke:var(--accent);stroke-width:2}.flx .flx-prec{stroke:var(--muted);stroke-width:1;stroke-dasharray:4 3}.flx .flx-ax{stroke:var(--line);stroke-width:1}.flx .flx-tick{fill:var(--muted);font-size:11px}.flx{--gv1:#0072B2;--gv2:#E69F00;--gv3:#009E73;--gv4:#D55E00;--gv5:#CC79A7;--gv6:#56B4E9;--gv7:#F0E442;--gv8:var(--ink)}.flx .flx-one{margin-top:6px}
.flx .flx-qc{margin-top:6px}.flx .flx-qc summary{cursor:pointer;min-height:24px}.flx .flx-table th[scope=row]{background:none;position:static;font-weight:600}.flx .flx-table caption{text-align:left;color:var(--muted);padding:4px 0}.flx .flx-table th,.flx .flx-table td{border:1px solid var(--line);padding:3px 8px;text-align:left;white-space:nowrap}.flx .flx-table th{position:sticky;top:0;background:var(--soft)}
.flx .fln-live{display:flex;flex-wrap:wrap;align-items:center;gap:0 6px;min-height:20px;font-size:12px;color:var(--muted)}.flx .fln-live[hidden],.flx .fln-bar[hidden],.flx .fln-files[hidden]{display:none}
.flx .fln-lm{display:inline-flex;flex:none}.flx .fln-lw{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.flx .fln-live[data-tone=ok]{color:var(--ok)}.flx .fln-live[data-tone=run]{color:var(--accent)}.flx .fln-live[data-tone=warn]{color:var(--warn)}.flx .fln-live[data-tone=bad]{color:var(--bad)}
.flx .fln-bar{flex-basis:100%;height:4px;border-radius:2px;background:var(--line);overflow:hidden}.flx .fln-bar i{display:block;height:100%;width:var(--p,0%);background:var(--accent)}
.flx .fln-bar.indet i{width:40%;background:repeating-linear-gradient(90deg,var(--accent) 0 6px,transparent 6px 10px)}
.flx .fln[data-live=stale]{border-style:dashed}.flx .fln[data-live=error],.flx .fln[data-live=refused]{border-color:var(--bad);border-left-color:var(--cc)}
.flx .fln-files{margin-top:8px}.flx .fln-files .flx-tablewrap{max-height:240px}
@media (prefers-reduced-motion:no-preference){.flx .fln-bar.indet i{animation:flSlide 1.2s linear infinite}}@keyframes flSlide{from{margin-left:-40%}to{margin-left:100%}}
.flx .flx-live-ctl{display:inline-flex;gap:4px;align-items:center}.flx .flx-live-ctl button{min-height:28px;min-width:24px}.flx .flx-live-ctl button[aria-pressed=true]{background:var(--accent);color:var(--on-accent)}
@media (max-width:900px){.flx .flx-body,.flx .flx-body.flx-adv{grid-template-columns:minmax(0,1fr)}.flx .flx-split{display:none}.flx .fl-insp{max-height:none}}
@media (pointer:coarse){.flx .flx-live-ctl button,.flx .flx-bar .fl-b{min-height:40px;min-width:40px}.flx .fln-next{width:40px;height:40px;right:-20px;margin-top:-20px}.flx .flx-seg button{min-height:40px}.flx .flx-split{min-width:24px}}
@media (prefers-reduced-motion:reduce){.flx .fln,.flx .fl-insp,.flx .flx-pop{animation:none}.flx .fl-stage.fl-soft,.flx .fln{transition:none}.flx .fln-bar.indet i{animation:none}}
:root[data-a11y-reduce] .flx .fln,:root[data-a11y-reduce] .flx .fl-insp,:root[data-a11y-reduce] .flx .flx-pop{animation:none}:root[data-a11y-reduce] .flx .fl-stage.fl-soft,:root[data-a11y-reduce] .flx .fln{transition:none}:root[data-a11y-reduce] .flx .fln-bar.indet i{animation:none}`;
  F.injectShellCss = () => {
    if (document.getElementById("flx-css")) return;
    const s = document.createElement("style"); s.id = "flx-css"; s.textContent = CSS; document.head.append(s);
  };
})();
