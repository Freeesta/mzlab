// @ts-check
// Flow (administrator mode): small helpers of the page: element builder, icons, the marks of the port types (colour + shape + letter), the texts of the err.flow.*
// keys and the stylesheet. English only (Flow has no catalogue in lang/). Colours are CSS variables (Okabe-Ito); motion is off with prefers-reduced-motion.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** builds an element: attrs may hold on* handlers, `class`, `text`, dataset keys as `data-x`; children are nodes or strings
   * @param {string} tag @param {Record<string, any> | null} attrs @param {...(Node | string | null | false | undefined)} kids @returns {any} */
  F.el = (tag, attrs, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (v == null || v === false) continue;
      if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else if (k === "text") e.textContent = v;
      else if (k in e && !k.includes("-") && k !== "list") /** @type {any} */ (e)[k] = v;
      else e.setAttribute(k, v === true ? "" : String(v));
    }
    for (const c of kids) if (c) e.append(c);
    return e;
  };
  const PATHS = {
    plus: "M12 5v14M5 12h14", up: "M6 15l6-6 6 6", down: "M6 9l6 6 6-6", x: "M6 6l12 12M18 6L6 18", compare: "M8 4v16M16 4v16M3 8l5-4 5 4M11 16l5 4 5-4",
    export: "M12 4v11M7 10l5 5 5-5M5 20h14", import: "M12 15V4M7 9l5-5 5 5M5 20h14", run: "M7 4l12 8-12 8z", undo: "M9 7L4 12l5 5M4 12h10a6 6 0 0 1 0 8", warn: "M12 4l9 16H3zM12 10v4M12 17v.01",
    ok: "M5 12l5 5 9-10", file: "M7 3h7l4 4v14H7zM14 3v5h5", flow: "M4 6h6v4H4zM14 14h6v4h-6zM10 8h2a2 2 0 0 1 2 2v4", copy: "M9 9h10v11H9zM5 15V4h10", search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM16 16l5 5",
    minus: "M5 12h14", fit: "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5", one: "M9 8l3-2v12", reset: "M4 12a8 8 0 1 0 2.6-5.9M4 4v5h5", info: "M12 8v.01M12 11v6M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
  };
  /** an inline icon (decorative: the button around it has the name) @param {keyof typeof PATHS} name @param {number} [px] */
  F.icon = (name, px = 16) => {
    const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    for (const [k, v] of Object.entries({ viewBox: "0 0 24 24", width: px, height: px, fill: "none", stroke: "currentColor", "stroke-width": 2, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true" })) s.setAttribute(k, String(v));
    const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
    p.setAttribute("d", PATHS[name]); s.append(p);
    return s;
  };
  /** an icon button with an accessible name and a tooltip; the click handler is `fn` @param {keyof typeof PATHS} icon @param {string} name @param {(e: Event) => void} fn @param {string} [text] */
  F.btn = (icon, name, fn, text) => F.el("button", { type: "button", class: "fl-b", title: name, "aria-label": name, onclick: fn }, F.icon(icon), text ? F.el("span", { text }) : null);

  /** the mark of a data type: colour, shape (by family) and the letter; the name is in the title and the accessible text @param {string} type */
  F.mark = type => {
    const t = F.TYPES[type] || { family: "end", letter: "?" };
    return F.el("span", { class: `fl-mk fl-f-${t.family}`, style: `--c:var(--t-${type})`, title: type, role: "img", "aria-label": `type ${type}` }, F.el("span", { "aria-hidden": "true", text: t.letter }));
  };

  /** the text of a problem; the keys are the err.flow.* of the Rust checker (docs/agenti/rust-flow.md) @param {any} p {instance, port, key, params} */
  F.message = p => {
    const q = p.params || {};
    /** @type {Record<string, string>} */ const M = {
      "err.flow.step.unknown": `Unknown step ${q.step}.`,
      "err.flow.step.version": `Step ${q.step} has version ${q.available}, not ${q.version}.`,
      "err.flow.instance.duplicate": "Two steps have the same name.",
      "err.flow.unavailable": `${q.step} cannot run on ${q.platform}.`,
      "err.flow.algorithm.unknown": `Unknown algorithm ${q.algorithm}.`,
      "err.flow.algorithm.unavailable": `Algorithm ${q.algorithm} is planned, not available yet.`,
      "err.flow.preset.unknown": `Unknown preset ${q.preset}.`,
      "err.flow.param.unknown": `Unknown parameter ${q.param}.`,
      "err.flow.param.type": `Parameter ${q.param} has the wrong type.`,
      "err.flow.param.range": `Parameter ${q.param} is out of range.`,
      "err.flow.port.unknown": `Unknown input or output ${q.port || q.ref}.`,
      "err.flow.port.missing": `Input ${q.port} is not connected.`,
      "err.flow.input.unknown": `The plan has no input file ${q.ref}.`,
      "err.flow.ref.unknown": `No step ${q.ref}.`,
      "err.flow.type": `A ${q.from} cannot go into a ${q.to} input.`,
      "err.flow.cardinality": q.count === "many" ? `Input ${q.port} takes one item, the source gives a list.` : `Input ${q.port} takes one source, ${q.count} are connected.`,
      "err.flow.facet": `The data do not have the ${q.facet} this step needs.`,
      "err.flow.cycle": "The steps depend on each other in a circle.",
      "err.flow.json": "The file is not valid JSON.",
      "err.flow.format": `Unknown file format ${q.format}; Flow reads mzflow/2.`,
      "err.flow.step": `The step ${q.step} must be written id@version.`,
      "err.flow.ref": `The reference ${q.ref} must be file:NAME or step.output.`,
      "err.flow.instance": `A step has no ${q.field}.`,
      "err.flow.input": `An input file has no ${q.field}.`,
      "err.flow.compare.sameVariant": "Two variants of the comparison are identical.",
      "err.flow.input.missing": `Open the file ${q.input} in the Data tab first, then run again.`,
      "err.flow.check": `The plan has problems: ${q.problems}.`,
      "err.flow.read.big": `${q.name} is too big to show here.`,
      "err.flow.run": "The run could not start.",
      "err.flow.compare.few": "A comparison needs at least two variants.",
    };
    return M[p.key] || p.key;
  };

  const CSS = `
.fl{--t-raw:#E69F00;--t-mzml:#F0B84A;--t-mzidx:#0072B2;--t-xic:#56B4E9;--t-peaks:#009E73;--t-spectra:#5DBF9C;--t-rtmap:#F0E442;--t-table:#D55E00;--t-annot:#E88A4C;--t-tp:#CC79A7;--t-library:#B4531E;--t-candidates:#C96A3A;--t-formulas:#DE9468;--t-structure:#CC79A7;--t-transform:#D9A0C0;--t-samples:#8A8F98;--t-qc:#8A8F98;--t-report:#8A8F98;--t-comparison:#8A8F98;display:grid;grid-template-columns:minmax(220px,300px) minmax(0,1fr);gap:12px;padding:10px 12px;align-items:start}
.fl *{box-sizing:border-box}.fl h2,.fl h3{margin:0;font-size:14px}.fl .fl-top{grid-column:1/-1;display:flex;flex-wrap:wrap;gap:8px;align-items:center}.fl .fl-top h2{font-size:16px;margin-right:6px}
.fl .fl-b{display:inline-flex;align-items:center;justify-content:center;gap:5px;min-width:28px;min-height:28px;padding:2px 8px;border:1px solid var(--line);border-radius:6px;background:var(--panel);color:var(--ink);cursor:pointer;font:inherit;transition:background .12s,opacity .12s}
.fl .fl-b:hover:not(:disabled){background:var(--sel)}.fl .fl-b:disabled{opacity:.4;cursor:default}.fl .fl-b:focus-visible,.fl select:focus-visible,.fl input:focus-visible,.fl summary:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.fl .fl-st{margin-left:auto;display:inline-flex;gap:6px;align-items:center;padding:3px 10px;border-radius:14px;background:var(--sel);font-weight:600}.fl .fl-st.ok{color:var(--ok)}.fl .fl-st.bad{color:var(--bad)}
.fl .fl-pal{border:1px solid var(--line);border-radius:8px;background:var(--panel);padding:8px;max-height:78vh;overflow:auto;position:sticky;top:6px}
.fl .fl-pal input[type=search]{width:100%;margin:6px 0;padding:4px 8px;min-height:28px;border:1px solid var(--line);border-radius:6px;background:var(--bg);color:var(--ink)}
.fl .fl-pal details{margin:4px 0}.fl .fl-pal summary{cursor:pointer;font-weight:600;padding:3px 0}.fl .fl-pal h4{margin:6px 0 2px;font-size:11px;text-transform:uppercase;letter-spacing:.04em;color:var(--muted)}
.fl .fl-it{display:flex;width:100%;gap:6px;align-items:center;text-align:left;min-height:28px;padding:3px 6px;border:1px solid transparent;border-radius:6px;background:transparent;color:var(--ink);cursor:pointer;font:inherit}
.fl .fl-it:hover{background:var(--sel);border-color:var(--line)}.fl .fl-it:focus-visible{outline:2px solid var(--accent)}.fl .fl-it.fl-dim{color:var(--muted)}.fl .fl-it .fl-ti{flex:1;min-width:0}.fl .fl-it small{color:var(--muted)}
.fl .fl-mk{display:inline-grid;place-items:center;flex:none;width:18px;height:18px;background:var(--c);color:#000;font-size:10px;font-weight:700;line-height:1;border:1px solid var(--ink)}
.fl .fl-f-file{border-radius:2px}.fl .fl-f-trace{border-radius:50%}.fl .fl-f-peak{border-radius:2px;transform:rotate(45deg) scale(.78)}.fl .fl-f-peak span{transform:rotate(-45deg)}.fl .fl-f-table{clip-path:polygon(50% 0,100% 100%,0 100%);border:0;padding-top:5px}.fl .fl-f-chem{clip-path:polygon(25% 0,75% 0,100% 50%,75% 100%,25% 100%,0 50%);border:0}.fl .fl-f-end{border-radius:50%;background:transparent;color:var(--ink);border:2px solid var(--c)}
.fl .fl-main{display:flex;flex-direction:column;gap:8px;min-width:0}.fl .fl-inputs{display:flex;flex-wrap:wrap;gap:6px;align-items:center;padding:6px 8px;border:1px dashed var(--line);border-radius:8px}
.fl .fl-chip{display:inline-flex;gap:5px;align-items:center;padding:2px 4px 2px 6px;border:1px solid var(--line);border-radius:14px;background:var(--panel)}
.fl ol.fl-chain{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:8px}.fl .fl-card{border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:8px;background:var(--panel);padding:8px 10px;animation:flIn .16s ease-out}
.fl .fl-card.bad{border-left-color:var(--bad)}.fl .fl-card.ok{border-left-color:var(--ok)}.fl .fl-hd{display:flex;gap:6px;align-items:center;flex-wrap:wrap}.fl .fl-hd .fl-ti{font-weight:700}.fl .fl-hd .fl-sp{flex:1}
.fl .fl-pk{font-size:11px;padding:0 6px;border-radius:10px;background:var(--sel);color:var(--muted)}.fl .fl-ports{display:grid;grid-template-columns:minmax(120px,max-content) minmax(0,1fr);gap:4px 10px;margin:6px 0;align-items:center}
.fl .fl-ports label,.fl .fl-ports .fl-pn{display:flex;gap:5px;align-items:center}.fl select,.fl input[type=number],.fl input[type=text]{min-height:28px;padding:2px 6px;border:1px solid var(--line);border-radius:6px;background:var(--bg);color:var(--ink);font:inherit;max-width:100%}
.fl .fl-out{display:flex;flex-wrap:wrap;gap:6px;color:var(--muted);font-size:12px}.fl .fl-probs{margin:6px 0 0;padding:0;list-style:none;color:var(--bad)}.fl .fl-probs li{display:flex;gap:5px;align-items:flex-start}.fl .fl-probs svg{flex:none;margin-top:2px}
.fl .fl-note{color:var(--muted)}.fl .fl-goal{padding:6px 10px;border-radius:8px;background:var(--sel)}.fl .fl-goal.ok{color:var(--ok)}.fl .fl-empty{padding:20px;text-align:center;color:var(--muted);border:1px dashed var(--line);border-radius:8px}
.fl .fl-form{display:grid;grid-template-columns:minmax(130px,max-content) minmax(0,1fr);gap:4px 10px;margin:6px 0;align-items:center}.fl .fl-form select,.fl .fl-form input[type=number],.fl .fl-form input[type=text]{max-width:420px}.fl .fl-form .fl-ds{grid-column:2;font-size:12px;color:var(--muted)}.fl .fl-form input[type=checkbox]{width:24px;height:24px}
.fl .fl-cli{border:1px solid var(--accent);border-radius:8px;padding:8px 10px;background:var(--panel)}.fl .fl-cli code{display:block;white-space:pre-wrap;word-break:break-all;background:var(--soft);padding:6px;border-radius:6px;margin:6px 0}
.fl details>summary{cursor:pointer;color:var(--accent);margin:4px 0}.fl .fl-meta{display:flex;flex-wrap:wrap;gap:6px;align-items:center}
@keyframes flIn{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:none}}
@media (max-width:760px){.fl{grid-template-columns:minmax(0,1fr)}.fl .fl-pal{position:static;max-height:40vh}}
@media (pointer:coarse){.fl .fl-b,.fl .fl-it,.fl select,.fl input{min-height:40px;min-width:40px}}
.fl .fl-var{display:flex;gap:6px;align-items:center;margin:4px 0}.fl .fl-tols{display:flex;gap:12px;flex-wrap:wrap;margin-top:6px}.fl .fl-tol{display:flex;flex-direction:column;font-size:12px;color:var(--muted)}
@media (prefers-reduced-motion:reduce){.fl .fl-card{animation:none}.fl .fl-b{transition:none}}:root[data-a11y-reduce] .fl .fl-card{animation:none}:root[data-a11y-reduce] .fl .fl-b{transition:none}`;
  F.injectCss = () => {
    if (document.getElementById("fl-css")) return;
    const s = document.createElement("style"); s.id = "fl-css"; s.textContent = CSS; document.head.append(s);
  };
  /** the command that runs an exported plan with the native runner @param {string} file */
  F.cliCommand = file => `cargo run --release -p mzlab-cli --example flow -- ${file} workdir 4`;
})();
