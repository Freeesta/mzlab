// @ts-strict
// Keyboard drag of a panel (lazy, loaded by arrange.js at the first Space on a handle): slots between rows or beside a row, the live preview of arrSlide, an announcement per step.
// Classic script, shares the globals of arrange.js / pannelli.js; the pointer drag stays in arrange.js.
/** Where the keys can put `p`: slots between the rows of the others (reading order, `n` = after the last row), or beside a row. */
function arrKeySlots(/** @type {any} */ p) {
  const order = arrStack(), rows = LAYOUT.righeDi(order.filter(q => q !== p)), grp = blockOf(p).grp.map((/** @type {any} */ q) => q.id), ids0 = order.map(q => q.id).join();
  const i0 = rows.filter(r => order.indexOf(r[0]) < order.indexOf(p)).length;
  const fit = (/** @type {any} */ d) => { const wide = d.k !== "sx" && d.k !== "dx", res = arrRedo(arrKeys(order), p.id, wide && !d.solo ? grp : [p.id], d, hostWidth()); return res ? { d, res } : null; };
  const slot = (/** @type {number} */ i) => {
    const t = rows[i] ? rows[i][0] : null, d = t ? { k: "su", t: t.id, solo: grp.includes(t.id) || undefined } : { k: "fine" }, a = fit(d);
    // moving the block past its own spectrum changes nothing: the panel goes alone then
    return a && i !== i0 && a.res.map((/** @type {any} */ q) => q.id).join() === ids0 ? fit({ ...d, solo: true }) : a;
  };
  return { rows, i0, slot, side: (/** @type {number} */ j, /** @type {string} */ k) => fit({ k, t: rows[j][0].id }) };
}
/** Keyboard drag: Space picks the panel up, arrows move it, Space drops it, Escape (or leaving the handle) cancels; every step is announced. */
function arrKeyDrag(/** @type {any} */ p, /** @type {any} */ g) {
  const K = arrKeySlots(p), order = arrStack(), n = K.rows.length;
  let i = K.i0, j = 0, side = "", cur = /** @type {any} */ (null);
  const z0 = p.el.style.zIndex; p.el.classList.add("arrange-kb"); p.el.style.zIndex = "5000";
  const show = (/** @type {any} */ a) => {
    const r = arrSlide(order, a.res, p, true), me = a.res.find((/** @type {any} */ q) => q.id === p.id); cur = a;
    arrShowPrev(r, arrLabel(a.d)); me.num = p.num; arrSay(arrWhere(me, a.res));
  };
  const end = (/** @type {boolean} */ commit) => {
    g.removeEventListener("keydown", key, true); g.removeEventListener("blur", blur);
    arrShowPrev(null, ""); p.el.classList.remove("arrange-kb");
    const d = commit && cur && (side || i !== K.i0) ? cur.d : null;
    if (d) { arrLiveEnd(true); arrRun(p, d, d.k !== "sx" && d.k !== "dx"); } else { p.el.style.zIndex = z0; arrLiveEnd(false); arrSay(ARR_T(commit ? "arrange.drop.same" : "arrange.cancel")); }
    g.focus();
  };
  const key = (/** @type {KeyboardEvent} */ e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    if (e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); end(false); return; }
    if (e.key === " " || e.key === "Enter") { e.preventDefault(); e.stopImmediatePropagation(); g._space = true; end(true); return; }
    const v = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0, h = e.key === "ArrowLeft" ? "sx" : e.key === "ArrowRight" ? "dx" : "";
    if (!v && !h) return;
    e.preventDefault(); e.stopImmediatePropagation();
    let a = null, ni = i, nj = j;
    if (v) { ni = side ? (v < 0 ? j : j + 1) : Math.max(0, Math.min(n, i + v)); a = K.slot(ni); }
    else if (n) { nj = side ? j : Math.min(i, n - 1); a = K.side(nj, h); }
    if (!a) { arrSay(ARR_T("arrange.zone.nospace")); return; }
    if (v) { i = ni; side = ""; } else { j = nj; side = h; }
    show(a);
  };
  const blur = () => end(false);
  g.addEventListener("keydown", key, true); g.addEventListener("blur", blur);
  arrSay(ARR_T("arrange.pick", { n: p.num ?? "" }));
}

window.ARRKEY = { drag: arrKeyDrag };
