// @ts-nocheck
// Bar handles of the integration windows (see below). Classic script, loaded on demand (carica) by scroll.js; cursorLine() calls BARRE.sync(p).
const BARRE = (() => {
// the two bars of every integration window: a 44 px target over the bar's top handle for finger, pen and keyboard (mouse keeps dragging the bar itself);
// ← → move the border one sample (Shift: five). The handle drawn on the canvas stays the visible dot.
function sync(/** @type {Bag} */ p) {
  const a = p._a, old = /** @type {HTMLElement[]} */ (p._bh || []); p._bh = [];
  const list = a && a.sr && !a.cas && a.X && !p._exp && p.type !== "spec" && p.type !== "map" ? p.ints.map((/** @type {Bag} */ it) => ({ it, s: a.sr.find((/** @type {Bag} */ q) => q.key === it.key) })).filter((/** @type {Bag} */ q) => q.s) : [];
  list.forEach((/** @type {Bag} */ q) => ["a", "b"].forEach(e => {
    const it = q.it, s = q.s, b = /** @type {HTMLElement} */ (old.pop() || (() => {
      const n = document.createElement("button"); n.type = "button"; n.className = "ibh"; n.setAttribute("role", "slider"); p.el.appendChild(n);
      const at = (/** @type {PointerEvent} */ ev) => p._bar.xd(ev.clientX - p.cv.getBoundingClientRect().left);
      n.addEventListener("pointerdown", ev => { ev.preventDefault(); ev.stopPropagation(); try { n.setPointerCapture(ev.pointerId); } catch (_) { /* events still arrive */ } setActive(p); p._bar.begin(n._it, n._e); n.classList.add("on"); });
      n.addEventListener("pointermove", ev => { if (n.classList.contains("on")) p._bar.move(at(ev)); });
      const end = (/** @type {PointerEvent} */ ev) => { if (!n.classList.contains("on")) return; n.classList.remove("on"); if (ev.type === "pointerup") p._bar.move(at(ev)); p._bar.end(); };
      n.addEventListener("pointerup", end); n.addEventListener("pointercancel", end);
      n.addEventListener("keydown", ev => {
        const d = ev.key === "ArrowLeft" || ev.key === "ArrowDown" ? -1 : ev.key === "ArrowRight" || ev.key === "ArrowUp" ? 1 : 0;
        if (!d || ev.ctrlKey || ev.metaKey || ev.altKey) return;
        ev.preventDefault(); ev.stopPropagation(); const sx = n._s.x, i = nearIdx(sx, n._it[n._e]) + d * (ev.shiftKey ? 5 : 1);
        setActive(p); p._bar.begin(n._it, n._e); p._bar.move(sx[Math.max(0, Math.min(sx.length - 1, i))]); p._bar.end();
      });
      return n;
    })());
    b._it = it; b._e = e; b._s = s; p._bh.push(b);
    const x = it.zero && typeof INTZERO !== "undefined" ? INTZERO.clamp(s, it[e]) : s.x[nearIdx(s.x, it[e])];
    b.style.left = p.cv.offsetLeft + a.X(x) + "px"; b.style.top = p.cv.offsetTop + M.t + 6 + "px"; b.hidden = false;
    const t = I18N.t(e === "a" ? "int.bar.a" : "int.bar.b", { rt: x.toFixed(2) }); b.setAttribute("aria-label", t); b.title = t;
    b.setAttribute("aria-valuenow", x.toFixed(2)); b.setAttribute("aria-valuetext", I18N.t("cursor.handle.value", { rt: x.toFixed(2) }));
    if (a.full) { b.setAttribute("aria-valuemin", a.full[0].toFixed(2)); b.setAttribute("aria-valuemax", a.full[1].toFixed(2)); }
  }));
  old.forEach(n => n.remove()); if (typeof INTBASE !== "undefined") INTBASE.sync(p);          // the points of a custom baseline (integra-base.js)
}
  return { sync };
})();
