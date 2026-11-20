// mzDraw page: forwards pointer, wheel and key events to the editor in the mzdraw .wasm and paints the display list it returns.
// No chemistry and no decision here: what a click does is decided in Rust (crates/mzdraw).
import { featManifest } from "../feat-manifesto.js";

const I = /** @type {any} */ (window).I18N, t = (k, p = {}) => I.t(k, p);
const $ = id => /** @type {HTMLElement} */ (document.getElementById(id));
const REPAINT = 1, CHANGED = 2, TOOL = 4, HANDLED = 8;
// toolbar: [command or tool name, key of the short label, key of the tooltip]; "|" is a separator
const TOOLS = ["select", "erase", "|", "bond:1", "bond:2", "bond:3", "|", "atom:C", "atom:N", "atom:O", "atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "|",
  "ring:6a", "ring:6", "ring:5", "|", "charge:+", "charge:-", "|", "undo", "redo", "|", "zoom-in", "zoom-out", "fit", "|", "clear"];
const COLOURS = ["--d2-ink", "--d2-select", "--d2-hover", "--d2-warn", "--d2-preview", "--d2-n", "--d2-o", "--d2-s", "--d2-x", "--d2-p"];

/** @type {any} */ let ed = null;
const cv = /** @type {HTMLCanvasElement} */ ($("d2-canvas")), g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d"));

async function load() {
  const base = new URL("../wasm-feat/", import.meta.url), e = (await featManifest(base)).mzdraw;
  if (!e) throw new Error("manifest");
  const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
  const sha = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(x => x.toString(16).padStart(2, "0")).join("");
  if (sha !== e.sha256) throw new Error("sha");
  const mod = await import(new URL(e.js, base).href);
  await mod.default({ module_or_path: bytes });
  if (mod.version() !== "mzdraw/1") throw new Error("version");
  return mod;
}

function paint() {
  const d = ed.scene(), s = ed.strings(), css = getComputedStyle(document.documentElement), col = COLOURS.map(c => css.getPropertyValue(c).trim());
  const r = devicePixelRatio || 1;
  g.setTransform(r, 0, 0, r, 0, 0);
  g.clearRect(0, 0, cv.clientWidth, cv.clientHeight);
  g.lineCap = "round"; g.textBaseline = "middle";
  for (let i = 0; i < d.length;) {
    const op = d[i];
    if (op === 1) { g.strokeStyle = col[d[i + 6]]; g.lineWidth = d[i + 5]; g.beginPath(); g.moveTo(d[i + 1], d[i + 2]); g.lineTo(d[i + 3], d[i + 4]); g.stroke(); i += 7; }
    else if (op === 2) { g.fillStyle = col[d[i + 4]]; g.font = d[i + 3] + "px Arial, Helvetica, sans-serif"; g.textAlign = d[i + 6] ? "left" : "center"; g.fillText(s[d[i + 5]], d[i + 1], d[i + 2]); i += 7; }
    else if (op === 3) { g.strokeStyle = col[d[i + 5]]; g.lineWidth = 1; g.setLineDash([4, 3]); g.strokeRect(d[i + 1], d[i + 2], d[i + 3], d[i + 4]); g.setLineDash([]); i += 6; }
    else { g.beginPath(); g.arc(d[i + 1], d[i + 2], d[i + 3], 0, 2 * Math.PI); if (op === 4) { g.fillStyle = col[d[i + 4]]; g.fill(); } else { g.strokeStyle = col[d[i + 4]]; g.lineWidth = 2; g.stroke(); } i += 5; }
  }
}

function size() {
  const r = devicePixelRatio || 1;
  cv.width = Math.round(cv.clientWidth * r); cv.height = Math.round(cv.clientHeight * r);
  paint();
}

/** the text of one check/1 warning, from the Drawing check catalog @param {any} w */
function warningText(w) {
  const allowed = (w.allowed || []).join(", ");
  return t(`draw.check.${w.code}.what`, { el: w.el || "", valence: w.valence ?? "", allowed, q: w.q ?? "", fix: w.fix ?? "", n: w.n ?? "", iso: w.iso ?? "", list: allowed });
}

function refresh() {
  const st = JSON.parse(ed.state());
  for (const b of $("d2-tools").querySelectorAll("button")) {
    const c = /** @type {HTMLElement} */ (b).dataset.cmd || "";
    if (c.includes(":") || c === "select" || c === "erase") b.setAttribute("aria-pressed", String(c === st.tool));
    if (c === "undo") /** @type {HTMLButtonElement} */ (b).disabled = !st.undo;
    if (c === "redo") /** @type {HTMLButtonElement} */ (b).disabled = !st.redo;
  }
  $("d2-status").textContent = t("draw2.count", { atoms: st.atoms, bonds: st.bonds });
  const list = $("d2-warnings"), ws = (st.check && st.check.warnings) || [];
  list.replaceChildren(...ws.map((/** @type {any} */ w) => {
    const li = document.createElement("li"), b = document.createElement("span");
    b.className = "lvl"; b.textContent = t(`draw.check.level.${w.level}`) + ": ";
    li.append(b, warningText(w)); li.dataset.code = w.code;
    return li;
  }));
  $("d2-summary").textContent = !st.atoms ? t("draw.check.empty") : ws.length ? t("draw.check.found", { n: ws.length }) : t("draw.check.ok");
  if (st.error) $("d2-live").textContent = I.err({ error_key: st.error, params: {} });
  else if (st.say.key) $("d2-live").textContent = t(`draw2.say.${st.say.key.slice(10)}`, { what: st.say.what });
  document.documentElement.dataset.d2 = JSON.stringify({ atoms: st.atoms, bonds: st.bonds, tool: st.tool, warnings: ws.length, selected: st.selected });
}

/** acts on the flags an editor call returned @param {number} f */
function after(f) {
  if (f & REPAINT) paint();
  if (f & (CHANGED | TOOL)) refresh();
  return f;
}

/** a toolbar command: view commands are handled here, the rest is the editor's @param {string} c */
function run(c) {
  const w = cv.clientWidth, h = cv.clientHeight;
  if (c === "zoom-in") after(ed.zoom(w / 2, h / 2, 1.25));
  else if (c === "zoom-out") after(ed.zoom(w / 2, h / 2, 0.8));
  else if (c === "fit") after(ed.fit(w, h));
  else after(ed.command(c) | CHANGED);
}

function toolbar() {
  const bar = $("d2-tools");
  for (const c of TOOLS) {
    if (c === "|") { const s = document.createElement("span"); s.className = "sep"; s.setAttribute("role", "separator"); bar.append(s); continue; }
    const b = document.createElement("button"), k = c.replace(":", ".").replace("+", "plus").replace("charge.-", "charge.minus").replace("zoom-in", "zoomIn").replace("zoom-out", "zoomOut");
    b.type = "button"; b.dataset.cmd = c; b.dataset.testid = "d2-" + c;
    b.textContent = c.startsWith("atom:") ? c.slice(5) : t(`draw2.tool.${k}`);
    b.title = t(`draw2.tool.${k}.title`); b.setAttribute("aria-label", b.title);
    b.addEventListener("click", () => { run(c); });
    bar.append(b);
  }
}

const mods = (/** @type {MouseEvent | KeyboardEvent} */ e) => (e.shiftKey ? 1 : 0) | (e.ctrlKey || e.metaKey ? 2 : 0);
const kind = (/** @type {PointerEvent} */ e) => (e.pointerType === "pen" ? 1 : e.pointerType === "touch" ? 2 : 0);

function events() {
  /** @type {{x: number, y: number} | null} */ let panning = null;
  const at = (/** @type {PointerEvent | WheelEvent} */ e) => { const r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  cv.addEventListener("pointerdown", e => {
    cv.focus(); cv.setPointerCapture(e.pointerId);
    if (e.button === 1 || e.button === 2) { panning = { x: e.clientX, y: e.clientY }; e.preventDefault(); return; }
    const [x, y] = at(e); after(ed.pointer(0, x, y, kind(e), mods(e)));
  });
  cv.addEventListener("pointermove", e => {
    if (panning) { after(ed.pan(e.clientX - panning.x, e.clientY - panning.y)); panning = { x: e.clientX, y: e.clientY }; return; }
    const [x, y] = at(e); after(ed.pointer(1, x, y, kind(e), mods(e)));
  });
  const up = (/** @type {PointerEvent} */ e) => {
    if (panning) { panning = null; return; }
    const [x, y] = at(e); after(ed.pointer(2, x, y, kind(e), mods(e)) | CHANGED);          // a release may have changed the selection alone
  };
  cv.addEventListener("pointerup", up); cv.addEventListener("pointercancel", up);
  cv.addEventListener("contextmenu", e => e.preventDefault());
  cv.addEventListener("wheel", e => { e.preventDefault(); const [x, y] = at(e); after(ed.zoom(x, y, e.deltaY < 0 ? 1.1 : 1 / 1.1)); }, { passive: false });
  cv.addEventListener("keydown", e => { const f = ed.key(e.key, mods(e)); if (after(f ? f | CHANGED : 0) & HANDLED) e.preventDefault(); });
  addEventListener("resize", size);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
}

/** adds a structure text to the drawing and says how it went @param {string} text */
function add(text) {
  const note = $("d2-note");
  try {
    const info = JSON.parse(ed.add(text));
    note.textContent = info.placeholder ? t("draw2.import.placeholder") : "";
    after(ed.fit(cv.clientWidth, cv.clientHeight) | CHANGED);
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))); }
}

/** downloads a text as a file @param {string} what @param {string} name @param {string} type */
function save(what, name, type) {
  const a = document.createElement("a"), url = URL.createObjectURL(new Blob([ed.save(what)], { type }));
  a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  URL.revokeObjectURL(url);
}

function panels() {
  $("d2-add").addEventListener("click", () => add(/** @type {HTMLTextAreaElement} */ ($("d2-text")).value));
  $("d2-open").addEventListener("click", () => $("d2-file").click());
  $("d2-file").addEventListener("change", async e => { const f = /** @type {HTMLInputElement} */ (e.target).files?.[0]; if (f) add(await f.text()); });
  $("d2-ex-molfile").addEventListener("click", () => save("molfile", "drawing.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-svg").addEventListener("click", () => save("svg", "drawing.svg", "image/svg+xml"));
  $("d2-ex-mzdraw").addEventListener("click", () => save("mzdraw", "drawing.mzdraw.json", "application/json"));
}

I.apply(document);
document.title = t("draw2.name");
try {
  ed = await load();
  /** @type {any} */ (window).MZDRAW = ed;
  toolbar(); events(); panels(); size(); refresh();
  document.documentElement.dataset.d2Ready = "1";
} catch (e) {
  $("d2-status").textContent = t("draw2.unavailable");
  document.documentElement.dataset.d2Ready = "0";
}
