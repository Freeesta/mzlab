// @ts-check
// Results window of «Identify all the MS2 of the file…» (libreria.js identifyAll): ONE aligned table (fixed column widths, sticky header, numbers right-aligned,
// only the name wraps), the rows added in chunks while scrolling (hundreds of rows do not lag), and the window maximisable and resizable with its size remembered.
// Loaded by libreria.js on first use; the cells come ready from the caller, nothing is computed here.

const KEY = "mzlab.libid.size", CHUNK = 120;
const CSS = `#libid{width:min(1280px,96vw);max-width:98vw;height:min(80vh,900px);max-height:98vh;min-width:min(420px,96vw);min-height:260px;padding:12px 14px;box-sizing:border-box;overflow:hidden;resize:both}
#libid[open]{display:flex;flex-direction:column}#libid.max{width:98vw;height:96vh;resize:none}
#libid #li-body{flex:1 1 auto;min-height:0;display:flex;flex-direction:column;gap:6px}#libid .li-sc{flex:1 1 auto;min-height:0;overflow:auto;border:1px solid var(--line);border-radius:6px}
#libid .li-sc:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
#libid .li-t{table-layout:fixed;border-collapse:separate;border-spacing:0;width:100%;min-width:1300px}
#libid .li-t th{position:sticky;top:0;z-index:1;background:var(--soft);padding:4px 8px;font-size:12px;text-align:left;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;border-bottom:1px solid var(--line)}
#libid .li-t td{padding:3px 8px;font-size:12px;text-align:left;vertical-align:middle;border-bottom:1px solid var(--line);white-space:nowrap;overflow:hidden;text-overflow:clip}
#libid .li-t .num{text-align:right;font-variant-numeric:tabular-nums}#libid .li-t td.nm{white-space:normal;overflow-wrap:anywhere}
#libid .li-t tr.lr{cursor:pointer}#libid .li-t tr.lr:hover{background:var(--sel)}
#libid #li-body>label{display:flex;gap:6px;align-items:center;justify-content:flex-start;width:auto;align-self:flex-start}#libid .li-st{width:120px;height:90px;display:flex;align-items:center;justify-content:center;cursor:zoom-in;border-radius:4px}#libid .li-st:focus-visible{outline:2px solid var(--accent);outline-offset:1px}#libid .li-st svg{width:100%;height:100%}#libid .li-t td.ik{font-size:11px;overflow-wrap:anywhere;white-space:normal;user-select:all}
#li-big{max-width:min(520px,96vw)}#li-big .smi{font-size:11px;overflow-wrap:anywhere;user-select:all;margin:6px 0}#li-big svg{width:100%;height:auto;max-height:60vh}
#libid .top button.li-mx{min-width:24px;min-height:24px;margin-right:6px}`;
/** @typedef {{h: string, w?: string, num?: boolean, nm?: boolean, ik?: boolean}} Col */

/** @type {{max: string, restore: string, struct: string, close: string}} */ let TX;
const css = document.createElement("style"); css.textContent = CSS; document.head.appendChild(css);
const read = () => { try { return JSON.parse(localStorage.getItem(KEY) || "null"); } catch (_) { return null; } };
const save = (/** @type {number} */ w, /** @type {number} */ h) => { try { localStorage.setItem(KEY, JSON.stringify({ w, h })); } catch (_) { /* no storage */ } };

/** The window: remembered size, a maximise button in the title row (aria-pressed), the size saved when the user resizes it.
 * @param {HTMLDialogElement} d @param {{max: string, restore: string, struct: string, close: string}} tx */
function frame(d, tx) {
  TX = tx;
  const s = read(); if (s && s.w > 0 && s.h > 0) { d.style.width = Math.min(s.w, innerWidth * 0.98) + "px"; d.style.height = Math.min(s.h, innerHeight * 0.98) + "px"; }
  const top = d.querySelector(".top"), x = d.querySelector("#li-x"); if (!top || !x || d.querySelector(".li-mx")) return;
  const b = document.createElement("button"); b.type = "button"; b.className = "li-mx"; b.setAttribute("aria-pressed", "false"); b.textContent = "⤢"; b.title = b.ariaLabel = tx.max;
  b.onclick = () => { const on = d.classList.toggle("max"); b.setAttribute("aria-pressed", String(on)); b.title = b.ariaLabel = on ? tx.restore : tx.max; };
  top.insertBefore(b, x);
  let t = 0; FRAME.ro(d, () => { if (d.classList.contains("max") || !d.open) return; clearTimeout(t); t = setTimeout(() => save(d.offsetWidth, d.offsetHeight), 300); });
}

/** OpenChemLib, once (the same lazy file as the fragmentation graph); null when it cannot be loaded. @type {any} */
let OCL;
const ocl = async () => { if (OCL === undefined) { try { const m = await import("./vendor/openchemlib.js"); OCL = m.default || m; } catch (_) { OCL = null; } } return OCL; };
/** @type {Map<string, string>} */
const pics = new Map();
/** The themed SVG of a SMILES, cached; "" when it cannot be drawn. */
async function draw(/** @type {string} */ smi, /** @type {number} */ w, /** @type {number} */ hh) {
  const key = `${smi}|${w}`; if (pics.has(key)) return /** @type {string} */ (pics.get(key));
  const o = await ocl(); let out = "";
  try { if (o) { out = o.Molecule.fromSmiles(smi).toSVG(w, hh, "li" + pics.size, window.MOLUI ? MOLUI.SVG_OPTS : undefined); if (window.MOLUI) out = MOLUI.svg(out); } } catch (_) { out = ""; }
  pics.set(key, out); return out;
}
/** The cell of the structure: empty until it scrolls into view (watch), a dash when the entry has no SMILES.
 * @param {string} smi @param {string} aria */
const cell = (smi, aria) => smi ? h`<div class="li-st" data-smi="${smi}" role="button" tabindex="0" title="${smi}" aria-label="${aria}"></div>` : "–";
/** The enlarged structure with its SMILES (select to copy). @param {string} smi @param {string} title @param {string} close */
async function big(smi, title, close) {
  let d = /** @type {HTMLDialogElement | null} */ (document.getElementById("li-big"));
  if (!d) { d = document.createElement("dialog"); d.id = "li-big"; document.body.appendChild(d); }
  const svg = await draw(smi, 420, 315);
  d.innerHTML = h`<div style="display:flex;align-items:center"><h3 style="margin:0;flex:1;font-size:14px">${title}</h3><button type="button" class="x" aria-label="${close}" title="${close}">&times;</button></div>${raw(svg)}<div class="smi">${smi}</div>`;
  const dd = d; /** @type {HTMLElement} */ (d.querySelector(".x")).onclick = () => dd.close(); if (!d.open) d.showModal();
}
/** Starts watching the structure cells under `from`: they are drawn when they scroll into view, and only then. @param {ParentNode} from @param {IntersectionObserver} io */
function watch(from, io) { from.querySelectorAll(".li-st:not([data-done])").forEach(c => io.observe(c)); }

/** The table in `host`: the first rows now, the next ones when the scroll gets near the end. `rows` = [{i, cells}] (cells = HTML of each cell in the order of `cols`).
 * @param {HTMLElement} host @param {Col[]} cols @param {{i: number, cells: string[]}[]} rows @param {string} empty @param {string} tip title of every row */
function fill(host, cols, rows, empty, tip) {
  const th = cols.map(c => h`<th scope="col" class="${c.num ? "num" : ""}">${raw(c.h)}</th>`).join("");
  const cg = cols.map(c => h`<col style="${c.w ? "width:" + c.w : ""}">`).join("");
  host.innerHTML = h`<div class="li-sc" tabindex="0"><table class="li-t"><colgroup>${raw(cg)}</colgroup><thead><tr>${raw(th)}</tr></thead><tbody></tbody></table></div>`;
  const sc = /** @type {HTMLElement} */ (host.firstElementChild), tb = /** @type {HTMLElement} */ (host.querySelector("tbody")); let n = 0;
  const more = () => {
    if (!rows.length) { tb.innerHTML = h`<tr><td colspan="${cols.length}" class="muted">${raw(empty)}</td></tr>`; return; }
    const html = rows.slice(n, n + CHUNK).map(r => h`<tr class="lr" data-i="${r.i}" title="${tip}">${raw(r.cells.map((c, j) => h`<td class="${cols[j].num ? "num" : cols[j].nm ? "nm" : cols[j].ik ? "ik" : ""}">${raw(c)}</td>`).join(""))}</tr>`).join("");
    tb.insertAdjacentHTML("beforeend", h`${raw(html)}`); n += CHUNK; watch(tb, io);
  };
  const io = new IntersectionObserver(es => es.forEach(e => {
    if (!e.isIntersecting) return; const c = /** @type {HTMLElement} */ (e.target); io.unobserve(c); c.dataset.done = "1";
    draw(c.dataset.smi || "", 120, 90).then(svg => { c.innerHTML = svg ? h`${raw(svg)}` : "–"; });
  }), { root: sc, rootMargin: "200px" });
  const open = (/** @type {Element} */ t) => { const c = /** @type {HTMLElement | null} */ (t.closest(".li-st")); if (c) big(c.dataset.smi || "", TX.struct, TX.close); return !!c; };
  host.addEventListener("click", e => open(/** @type {Element} */ (e.target)));
  host.addEventListener("keydown", e => { if ((e.key === "Enter" || e.key === " ") && open(/** @type {Element} */ (e.target))) e.preventDefault(); });
  more();
  sc.onscroll = () => { if (n < rows.length && sc.scrollTop + sc.clientHeight > sc.scrollHeight - 200) more(); };
  return { more, rendered: () => Math.min(n, rows.length) };
}

window.LIBR = { frame, fill, cell };
