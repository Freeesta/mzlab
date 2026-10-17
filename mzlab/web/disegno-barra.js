// @ts-check
// Drop-down toolbar of the editor (Disegno). Ketcher has no API to rearrange its toolbars, so a group of its buttons is replaced by ONE proxy button
// that opens a small menu; choosing an item clicks the real (hidden) Ketcher button, and the proxy mirrors which one is selected.
// To add a group: one entry of GROUPS (the data-testid of the real buttons + a key of lang/ for each tooltip).
// Self-check: a group whose real buttons are not all found is left as Ketcher draws it; if the bar is removed from the page the native toolbars come back.

/** @typedef {{ id: string, tip: string }} Item */
/** @typedef {{ id: string, tool: string | null, tip: string, cols: number, items: Item[], hide: string[] }} Group */

/** @type {Group[]} */
export const GROUPS = [
  { id: "rings", tool: "rings", tip: "draw.bar.rings", cols: 3, hide: ['[class*="BottomToolbar-module_root"]'], items: [
    { id: "template-0", tip: "draw.bar.ring.benzene" }, { id: "template-1", tip: "draw.bar.ring.cyclopentadiene" }, { id: "template-2", tip: "draw.bar.ring.cyclohexane" },
    { id: "template-3", tip: "draw.bar.ring.cyclopentane" }, { id: "template-4", tip: "draw.bar.ring.cyclopropane" }, { id: "template-5", tip: "draw.bar.ring.cyclobutane" },
    { id: "template-6", tip: "draw.bar.ring.cycloheptane" }, { id: "template-7", tip: "draw.bar.ring.cyclooctane" }, { id: "template-lib", tip: "draw.bar.ring.library" }] },
  { id: "charge", tool: null, tip: "draw.bar.charge", cols: 2, hide: ['[data-testid="charge-plus"]', '[data-testid="charge-minus"]'], items: [
    { id: "charge-plus", tip: "draw.bar.charge.plus" }, { id: "charge-minus", tip: "draw.bar.charge.minus" }] },
];

const CSS = `:root{--tpb-bg:#fff;--tpb-line:#b8bcc4;--tpb-sel:#d6e6ff;--tpb-focus:#0b57d0}
[class*="LeftToolbar-module_root"]{overflow:visible!important}
#tp-bar{display:flex;flex-direction:column;align-items:center;gap:4px;padding:4px;margin-top:8px;position:relative;z-index:5;background:var(--tpb-bg);border-radius:6px;box-shadow:0 1px 4px rgba(0,0,0,.18)}
#tp-bar[hidden],#tp-bar .tpb-g[hidden]{display:none}
#tp-bar .tpb-g{position:relative}
#tp-bar button{cursor:pointer;display:flex;align-items:center;justify-content:center;gap:1px;min-width:32px;min-height:32px;padding:0;border:0;border-radius:4px;background:transparent;color:inherit}
#tp-bar button:hover{background:#eef1f5}
#tp-bar button.tp-sel{background:var(--tpb-sel)}
#tp-bar button:focus-visible{outline:2px solid var(--tpb-focus);outline-offset:1px}
#tp-bar button svg{width:24px;height:24px;flex:none}
#tp-bar .tpb-caret{width:6px!important;height:6px!important;opacity:.6}
#tp-bar [role=menu]{position:absolute;left:100%;top:0;margin-left:4px;display:grid;gap:2px;padding:4px;background:var(--tpb-bg);border:1px solid var(--tpb-line);border-radius:6px;box-shadow:0 2px 8px rgba(0,0,0,.2)}
#tp-bar [role=menu][hidden]{display:none}
@media (pointer:coarse){#tp-bar button{min-width:calc(40px / var(--ksz,1));min-height:calc(40px / var(--ksz,1))}}
@media (prefers-reduced-motion:no-preference){#tp-bar button{transition:background .1s}}`;

/** Builds the bar in the document `d` of Ketcher's iframe.
 * @param {Document} d
 * @param {(key: string) => string} t translator
 * @returns {{ update: (choice: Record<string, boolean>) => void, destroy: () => void, state: string } | null} null when nothing could be replaced */
export function mountBar(d, t) {
  const root = d.querySelector('[class*="LeftToolbar-module_root"]');
  const found = (/** @type {string} */ id) => /** @type {HTMLElement | null} */ (d.querySelector(`[data-testid="${id}"]`));
  const usable = GROUPS.filter(g => g.items.every(i => found(i.id) && found(i.id)?.querySelector("svg")));
  const missing = GROUPS.filter(g => !usable.includes(g)).map(g => g.id);
  if (missing.length) console.warn("Ketcher toolbar: native buttons kept for", missing.join(", "), "(a button was not found)");
  if (!root || !usable.length) { d.documentElement.dataset.tpBar = "native"; return null; }

  const style = d.createElement("style"); style.id = "tp-bar-css";
  style.textContent = CSS + usable.flatMap(g => g.hide).join(",") + "{display:none!important}";
  const bar = d.createElement("div"); bar.id = "tp-bar"; bar.setAttribute("role", "toolbar"); bar.setAttribute("aria-label", t("draw.bar.label"));
  // Ketcher marks the active tool with a "selected" class
  const isSel = (/** @type {Element} */ e) => [...e.classList].some(c => /selected/i.test(c));

  /** @type {{ g: Group, box: HTMLElement, btn: HTMLButtonElement, menu: HTMLElement, items: HTMLButtonElement[], cur: number }[]} */
  const views = [];
  const closeAll = (/** @type {boolean} */ focus = false) => views.forEach(v => { if (!v.menu.hidden) { v.menu.hidden = true; v.btn.setAttribute("aria-expanded", "false"); if (focus) v.btn.focus(); } });
  const icon = (/** @type {Item} */ it) => /** @type {SVGElement} */ (found(it.id)?.querySelector("svg")?.cloneNode(true));

  for (const g of usable) {
    const box = d.createElement("div"); box.className = "tpb-g";
    const btn = d.createElement("button"); btn.type = "button"; btn.setAttribute("aria-haspopup", "menu"); btn.setAttribute("aria-expanded", "false");
    btn.setAttribute("aria-label", t(g.tip)); btn.title = t(g.tip); btn.dataset.tp = g.id;
    const menu = d.createElement("div"); menu.setAttribute("role", "menu"); menu.setAttribute("aria-label", t(g.tip)); menu.hidden = true;
    menu.style.gridTemplateColumns = `repeat(${g.cols}, auto)`;
    const items = g.items.map(it => {
      const b = d.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitem"); b.tabIndex = -1;
      b.setAttribute("aria-label", t(it.tip)); b.title = t(it.tip); b.appendChild(icon(it));
      b.onclick = () => { found(it.id)?.click(); v.cur = g.items.indexOf(it); closeAll(true); sync(); };
      menu.appendChild(b); return b;
    });
    const v = { g, box, btn, menu, items, cur: 0 };
    // opening a group also selects its current tool (the one drawn on the button), so one click is enough to start drawing; the menu then offers the variants
    const open = (/** @type {number} */ at) => { closeAll(); found(g.items[at].id)?.click(); menu.hidden = false; btn.setAttribute("aria-expanded", "true"); items[at].focus(); sync(); };
    btn.onclick = () => { if (menu.hidden) open(v.cur); else closeAll(); };
    btn.addEventListener("keydown", e => { if (e.key === "ArrowDown" || e.key === "ArrowRight") { e.preventDefault(); open(v.cur); } });
    menu.addEventListener("keydown", e => {
      const i = items.indexOf(/** @type {HTMLButtonElement} */ (d.activeElement)), n = items.length;
      const go = (/** @type {number} */ k) => { e.preventDefault(); items[(k + n) % n].focus(); };
      if (e.key === "ArrowDown" || e.key === "ArrowRight") go(i + 1);
      else if (e.key === "ArrowUp" || e.key === "ArrowLeft") go(i - 1);
      else if (e.key === "Home") go(0); else if (e.key === "End") go(n - 1);
      else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); closeAll(true); }
      else if (e.key === "Tab") closeAll();
    });
    box.append(btn, menu); bar.appendChild(box); views.push(v);
  }
  d.addEventListener("pointerdown", e => { if (!bar.contains(/** @type {Node} */ (e.target))) closeAll(); });

  // the proxy shows the icon of the item last chosen (or the selected one) and looks selected while one of its real buttons is
  function sync() {
    if (!bar.isConnected) { destroy(); return; }
    for (const v of views) {
      const sel = v.g.items.findIndex(it => { const e = found(it.id); return !!e && isSel(e); });
      if (sel >= 0) v.cur = sel;
      v.btn.replaceChildren(icon(v.g.items[v.cur]));
      const caret = d.createElementNS("http://www.w3.org/2000/svg", "svg"); caret.setAttribute("viewBox", "0 0 6 6"); caret.setAttribute("class", "tpb-caret"); caret.setAttribute("aria-hidden", "true");
      caret.innerHTML = '<path d="M0 1l3 4 3-4z" fill="currentColor"/>'; v.btn.appendChild(caret);
      v.btn.classList.toggle("tp-sel", sel >= 0); v.btn.setAttribute("aria-pressed", String(sel >= 0));
      v.items.forEach((b, i) => b.classList.toggle("tp-sel", i === sel));
    }
  }
  const obs = new MutationObserver(sync), gone = new MutationObserver(() => { if (!bar.isConnected) destroy(); });
  function destroy() { obs.disconnect(); gone.disconnect(); bar.remove(); style.remove(); d.documentElement.dataset.tpBar = "native"; }

  d.head.appendChild(style); root.appendChild(bar);
  const app = d.querySelector('[class*="App-module_app"]') || d.body;
  obs.observe(app, { subtree: true, attributes: true, attributeFilter: ["class"] }); gone.observe(root, { childList: true });
  d.documentElement.dataset.tpBar = "menu"; sync();
  return {
    state: "menu", destroy,
    /** The tool groups that the student switched off hide their proxy too. */
    update(choice) { for (const v of views) v.box.hidden = !!v.g.tool && choice[v.g.tool] === false; bar.hidden = views.every(v => v.box.hidden); if (bar.hidden) closeAll(); },
  };
}
