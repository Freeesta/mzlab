// Smooth layout changes without redrawing the plots on every frame: FLIP with transform only (Web Animations API).
// The layout jumps to its final state at once (one refit + redraw), then the elements slide and stretch from where they were.
// Tokens (--t-med, --ease-out) come from transizioni.css; duration 0 (reduced motion) or the first paint = no animation.
const R = document.documentElement;
const Q1 = s => document.querySelector(s);
const cs = () => getComputedStyle(R);
const ms = () => parseFloat(cs().getPropertyValue("--t-med")) || 0;
const ease = () => cs().getPropertyValue("--ease-out").trim() || "ease-out";
const still = () => R.classList.contains("no-tr") || !ms() || !Element.prototype.animate;

let pressed = false, suppress = false;                // the user is dragging or resizing: the layout follows the pointer at once
["pointerdown", "mousedown"].forEach(t => addEventListener(t, () => { pressed = true; }, true));
["pointerup", "pointercancel", "mouseup", "dragend", "blur"].forEach(t => addEventListener(t, () => { pressed = false; }, true));

const rect = el => { const r = el.getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; };
function play(el, a, b, keepSize) {                     // a = where it was, b = where it is: slide (and stretch) back to b
  const dx = a.x - b.x, dy = a.y - b.y, sx = !keepSize && b.w && Math.abs(a.w - b.w) > 1 ? a.w / b.w : 1, sy = !keepSize && b.h && Math.abs(a.h - b.h) > 1 ? a.h / b.h : 1;
  if (Math.abs(dx) < 1 && Math.abs(dy) < 1 && sx === 1 && sy === 1) return;
  el.animate([{ transformOrigin: "0 0", transform: `translate(${dx}px,${dy}px) scale(${sx},${sy})` }, { transformOrigin: "0 0", transform: "none" }], { duration: ms(), easing: ease() });
}
const cancelAll = el => el.getAnimations().forEach(x => x.cancel());
const panels = () => [...document.querySelectorAll("#dpanels>.pnl")].filter(p => p.offsetWidth > 0 && !p.classList.contains("max"));

// opts: { aside, was, shown, moved } ; run() applies the final layout and does the single refit
function layout(opts, run) {
  const { aside, was, shown, moved } = opts;
  if (still()) { run(); return; }
  const ps = panels(), first = ps.map(rect), aFirst = aside && was ? rect(aside) : null;
  suppress = true;
  run();
  queueMicrotask(() => { suppress = false; });
  ps.forEach(cancelAll);
  ps.forEach((p, i) => play(p, first[i], rect(p)));
  if (!aside) return;
  const dir = aside.closest(".sb-right") ? 1 : -1;
  if (shown && !was) aside.animate([{ opacity: 0, transform: `translateX(${dir * 14}px)` }, { opacity: 1, transform: "none" }], { duration: ms(), easing: ease() });
  else if (shown && was && moved && aFirst) { cancelAll(aside); play(aside, aFirst, rect(aside), true); }
  else if (!shown && was && aFirst) {                   // keep the side panel on screen while it fades out
    aside.classList.add("sb-leaving");
    Object.assign(aside.style, { left: aFirst.x + "px", top: aFirst.y + "px", width: aFirst.w + "px", height: aFirst.h + "px" });
    const done = () => { aside.classList.remove("sb-leaving"); ["left", "top", "width", "height"].forEach(k => aside.style.removeProperty(k)); };
    const an = aside.animate([{ opacity: 1, transform: "none" }, { opacity: 0, transform: `translateX(${dir * -14}px)` }], { duration: ms(), easing: ease() });
    an.onfinish = an.oncancel = done;
  }
}

// panels of the work area that change place or size by themselves (neighbours closing a gap, a plot growing, a new grid)
const num = (s, k) => { const m = new RegExp("(?:^|[;\\s])" + k + ":\\s*(-?[\\d.]+)px").exec(s || ""); return m ? +m[1] : null; };
const geom = s => ({ x: num(s, "left"), y: num(s, "top"), w: num(s, "width"), h: num(s, "height") });
let mo = null;
function watchPanels() {
  const dp = Q1("#dpanels");
  if (!dp || typeof MutationObserver === "undefined") return;
  const born = new WeakSet();
  const opt = { attributes: true, attributeFilter: ["style"], attributeOldValue: true };
  mo = new MutationObserver(recs => {
    const skip = suppress || pressed || still(), seen = new Set(), jobs = [];
    for (const r of recs) {
      if (r.type === "childList") { r.addedNodes.forEach(n => { if (n.classList && n.classList.contains("pnl")) { born.add(n); mo.observe(n, opt); } }); continue; }
      const el = r.target;
      if (skip || seen.has(el) || el.classList.contains("max")) continue;
      seen.add(el);
      const a = geom(r.oldValue), b = geom(el.getAttribute("style"));
      if ([a.x, a.y, b.x, b.y].some(v => v === null)) continue;
      jobs.push([el, { x: a.x, y: a.y, w: a.w ?? b.w, h: a.h ?? b.h }, b]);
    }
    if (!jobs.length || !jobs[0][0].parentNode) return;
    jobs.forEach(([el, a, b]) => { if (el.offsetWidth > 0) { cancelAll(el); play(el, a, b); } });
  });
  mo.observe(dp, { childList: true });
  dp.querySelectorAll(":scope>.pnl").forEach(p => mo.observe(p, opt));
}
watchPanels();
window.TRANS = { layout, still };
