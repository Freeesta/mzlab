// @ts-check
// Draw's side cards fold one by one: the chevron button in each header (disclosure: aria-expanded, aria-controls) shows or hides the body.
// Quick draw, Expected properties and Export canvas start open, the rest closed; the student's choice is kept in `qqq.disegno.schede`
// (the inline script after the aside in index.html applies it before the first paint). A card whose tool gives a result opens itself.

/** the cards that start open; every other card starts closed */
export const OPEN = ["quick-card", "prop-card", "exp-card"];
const KEY = typeof NOMI !== "undefined" ? NOMI.KEY("disegno.schede") : "qqq.disegno.schede";
const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);
const CHEV = '<svg class="rq-chev" width="12" height="12" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true" focusable="false"><path d="M1.5 3.5h7L5 7.5z"/></svg>';

/** the stored states, {card id: 1 open | 0 closed} */
function stored() {
  try { const s = JSON.parse(localStorage.getItem(KEY) || "{}"); return s && typeof s === "object" ? s : {}; } catch (_) { return {}; }
}
function remember(/** @type {string} */ id, /** @type {boolean} */ open) {
  try { const s = stored(); s[id] = open ? 1 : 0; localStorage.setItem(KEY, JSON.stringify(s)); } catch (_) { /* storage not available: the default stays */ }
}

const isOpen = (/** @type {Element} */ c) => c instanceof HTMLDetailsElement ? c.open : c.querySelector(".rq-tg")?.getAttribute("aria-expanded") === "true";

/** the accessible name of the toggle says what a press does, with the card title */
function label(/** @type {Element} */ c) {
  const b = c.querySelector(".rq-tg"), name = b?.querySelector("span")?.textContent || "";
  if (b) b.setAttribute("aria-label", t(isOpen(c) ? "draw.card.collapse" : "draw.card.expand", { name }));
}

/** shows or hides the body of card `c`; `keep` writes the choice (a press of the student, not a result that opened it) */
export function setOpen(/** @type {Element | null} */ c, /** @type {boolean} */ open, keep = false) {
  if (!c) return;
  if (c instanceof HTMLDetailsElement) c.open = open;
  else {
    const b = c.querySelector(".rq-tg"), body = b && document.getElementById(b.getAttribute("aria-controls") || "");
    if (!b || !body) return;
    b.setAttribute("aria-expanded", String(open)); body.hidden = !open; label(c);
  }
  if (keep) remember(c.id, open);
}

/** a card built by a module (h2 with data-i18n, an optional `.rq-d` description, then the controls) gets the same header and body */
function enhance(/** @type {HTMLElement} */ c) {
  if (c.querySelector(".rq-tg") || c instanceof HTMLDetailsElement || !c.id) return;
  const h = c.querySelector(":scope > h2");
  if (!h) return;
  const b = document.createElement("button"), s = document.createElement("span"), body = document.createElement("div");
  b.type = "button"; b.className = "rq-tg"; b.setAttribute("aria-controls", c.id + "-b");
  const k = h.getAttribute("data-i18n");
  if (k) { s.setAttribute("data-i18n", k); h.removeAttribute("data-i18n"); }
  s.textContent = h.textContent; h.textContent = ""; h.className = "rq-h";
  b.appendChild(s); b.insertAdjacentHTML("beforeend", CHEV); h.appendChild(b);
  // the explanations are no visible paragraphs: a tooltip on the header and, for assistive tech, the toggle's description
  const ds = [...c.querySelectorAll(":scope > .rq-d")];
  ds.forEach((d, n) => { d.id = d.id || c.id + "-d" + n; d.className = "sr-only"; });
  if (ds.length) b.setAttribute("aria-describedby", ds.map(d => d.id).join(" "));
  body.className = "rq-b"; body.id = c.id + "-b";
  [...c.children].filter(e => e !== h && !ds.includes(e) && e.tagName !== "STYLE").forEach(e => body.appendChild(e));
  c.appendChild(body);
  const st = stored()[c.id];
  setOpen(c, st == null ? OPEN.includes(c.id) || c.dataset.rqOpen === "1" : !!st);
}

/** the header tooltip repeats the hidden description, so it follows the catalog text (a literal title would be a long tooltip in the HTML) */
function tips(/** @type {Element} */ side) {
  side.querySelectorAll(".rq-tg[aria-describedby]").forEach((/** @type {Element} */ b) => {
    const h = b.closest(".rq-h"), v = (b.getAttribute("aria-describedby") || "").split(" ").map(id => document.getElementById(id)?.textContent || "").join(" ").trim();
    if (h instanceof HTMLElement && h.title !== v) h.title = v;
  });
}

/** a status line of a closed card that gets a text (a cut, a check, an error) opens the card */
function opener(/** @type {MutationRecord[]} */ ms, /** @type {MutationObserver} */ _o, side = document.querySelector("#v-draw aside")) {
  for (const m of ms) {
    const n = m.target instanceof Element ? m.target : m.target.parentElement;
    const st = n && n.closest("[role=status]"), c = st && st.closest(".card");
    if (c && !isOpen(c) && (st.textContent || "").trim()) setOpen(c, true);
    if (m.type === "childList") m.addedNodes.forEach(x => { if (x instanceof HTMLElement && x.classList.contains("card")) enhance(x); });
  }
  if (side) tips(side);
}

/** wires the static cards of index.html and the ones the modules add later */
export function mountCards() {
  const side = document.querySelector("#v-draw aside");
  if (!side || side.hasAttribute("data-rq")) return;
  side.setAttribute("data-rq", "1");
  side.querySelectorAll(":scope > .card").forEach((/** @type {Element} */ c) => { if (c instanceof HTMLElement) enhance(c); });
  side.addEventListener("click", (/** @type {Event} */ e) => {
    const b = e.target instanceof Element && e.target.closest(".rq-tg"), c = b && b.closest(".card");
    if (c) setOpen(c, !isOpen(c), true);
  });
  side.addEventListener("toggle", (/** @type {Event} */ e) => { const d = e.target; if (d instanceof HTMLDetailsElement && d.classList.contains("card") && d.dataset.rqUser) { remember(d.id, d.open); delete d.dataset.rqUser; } }, true);
  side.addEventListener("click", (/** @type {Event} */ e) => { const s = e.target instanceof Element && e.target.closest("summary"), d = s && s.parentElement; if (d instanceof HTMLDetailsElement) d.dataset.rqUser = "1"; }, true);
  side.querySelectorAll(".rq-tg").forEach((/** @type {Element} */ b) => { const c = b.closest(".card"); if (c) label(c); });
  tips(side);
  // a <details> card opened from storage before draw.js listened: its content comes with a toggle event
  side.querySelectorAll("details.card[open]").forEach((/** @type {Element} */ d) => d.dispatchEvent(new Event("toggle")));
  new MutationObserver(opener).observe(side, { childList: true, subtree: true, characterData: true });
  // a module that brings the student to its card (a canvas menu row) asks for it open
  document.addEventListener("mzlab-card-open", e => setOpen(/** @type {CustomEvent} */ (e).detail, true));
  document.addEventListener("mzlab-card-fold", e => setOpen(/** @type {CustomEvent} */ (e).detail, false));
}
