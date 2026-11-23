// @ts-strict
// Side-by-side panels of the Dati view: the arrGrip in the header (drag with zones, arrMenu, Alt+arrows), the dividers between neighbours, undo (Ctrl/Cmd+Z).
// Classic script, loaded after pannelli.js (shared globals). The rows themselves (riga, quota, geometry) are in layout.js; here only the gestures.
const ARR_T = (/** @type {string} */ k, /** @type {any} */ v = undefined) => I18N.t(k, v);
const arrCss = document.createElement("style");
arrCss.textContent = ".arrange-grip{flex:none;min-width:24px;min-height:24px;padding:0 2px;margin-right:2px;border:0;background:transparent;color:var(--muted);cursor:grab;touch-action:none;opacity:0;line-height:1;font-size:14px}" +
  ".pnl:hover .arrange-grip,.arrange-grip:focus-visible,.arrange-grip[aria-expanded=true]{opacity:1}.arrange-grip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.arrange-grip{opacity:1;min-width:44px;min-height:44px}}" +
  ".pnl.arrange-drag{opacity:.45}.arrange-prev{position:absolute;z-index:5001;box-sizing:border-box;pointer-events:none;border:2px dashed var(--accent);background:var(--sel);opacity:.8;display:flex;align-items:center;justify-content:center;color:var(--ink);font-size:13px;font-weight:600}" +
  ".arrange-prev.no{border-color:var(--muted);background:var(--soft);color:var(--muted)}" +
  ".arrange-menu{position:fixed;z-index:10005;min-width:200px;max-width:min(320px,94vw);max-height:70vh;overflow:auto;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);padding:4px}" +
  ".arrange-menu button{display:block;width:100%;min-height:28px;text-align:left;border:0;background:transparent;color:var(--ink);padding:3px 8px;border-radius:4px}.arrange-menu button:hover,.arrange-menu button:focus-visible{background:var(--sel);outline:2px solid var(--accent);outline-offset:-2px}.arrange-menu button[aria-disabled=true]{color:var(--muted);cursor:default}" +
  ".arrange-menu .aff-lbl{display:block;padding:4px 8px 2px;font-size:11px;color:var(--muted)}" +
  ".arrange-sep{position:absolute;z-index:6;width:24px;cursor:col-resize;touch-action:none;display:flex;justify-content:center;align-items:stretch;border:0;padding:0;background:transparent}.arrange-sep::after{content:'';width:4px;border-radius:2px;background:var(--line)}" +
  ".arrange-sep:hover::after,.arrange-sep:focus-visible::after,.arrange-sep.on::after{background:var(--accent)}.arrange-sep:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.arrange-sep{width:40px}}" +
  "#arrange-bar{position:fixed;z-index:100001;bottom:20px;left:50%;transform:translateX(-50%);display:flex;gap:10px;align-items:center;background:var(--ink);color:var(--panel);padding:6px 12px;border-radius:6px;box-shadow:0 3px 12px rgba(0,0,0,.25)}#arrange-bar button{min-height:24px;padding:0 8px}" +
  "#arrange-live{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}";
document.head.appendChild(arrCss);

const arrEligible = (/** @type {any} */ p) => !!(p.full && p.el && !dockedP(p) && !hasDocked(p) && !p.el.classList.contains("max"));
const arrKeys = (/** @type {any[]} */ l) => l.map(q => ({ id: q.id, x: q.x, y: q.y, w: q.w, h: q.h, riga: q.riga, quota: q.quota, tab: q.tab, full: q.full }));
const arrEntra = (/** @type {any[]} */ row, /** @type {number} */ W) => row.length <= LAYOUT.MAX_ROW && row.every(q => minW(q) * row.length + pnlGap() * (row.length - 1) <= W);

/**
 * The move `d` applied to an ordered arrStack (real panels or copies of them). `grp` = ids that travel together (arrVertical moves).
 * d.k: sx / dx (beside d.t), su / giu (own row before / after the row of d.t), fine (own row at the end), uguali (equal widths of the row of p).
 * @returns {any[] | null} the new order, or null when it does not fit (nothing is touched then)
 */
function arrRedo(/** @type {any[]} */ order, /** @type {number} */ pid, /** @type {number[]} */ grp, /** @type {any} */ d, /** @type {number} */ W) {
  const p = order.find(q => q.id === pid), rest = order.filter(q => !grp.includes(q.id)), mates = (/** @type {any} */ q) => rest.filter(x => x.riga === q.riga);
  if (!p) return null;
  if (d.k === "uguali") { order.filter(q => q.riga === p.riga).forEach(q => { q.quota = null; }); return order; }
  const t = d.t != null ? rest.find(q => q.id === d.t) : null;
  if (d.k !== "fine" && (!t || t === p)) return null;
  const side = d.k === "sx" || d.k === "dx";
  if (side && !arrEntra([...mates(t), p], W)) return null;
  const old = order.filter(q => q !== p && q.riga === p.riga && !grp.includes(q.id));
  // left alone: back to the full width
  if (old.length === 1) old[0].quota = 1;
  const out = rest.slice();
  if (side) {
    p.riga = t.riga; mates(t).forEach(q => { q.quota = null; }); p.quota = null;
    out.splice(out.indexOf(t) + (d.k === "dx" ? 1 : 0), 0, p);
  } else {
    const base = order.reduce((m, q) => Math.max(m, q.riga ?? -1), -1) + 1, blk = order.filter(q => grp.includes(q.id));
    blk.forEach((q, i) => { q.riga = base + i; q.quota = null; q.x = 0; q.w = W; });
    const row = t ? mates(t) : [], at = d.k === "su" ? out.indexOf(row[0]) : d.k === "giu" ? out.indexOf(row[row.length - 1]) + 1 : out.length;
    out.splice(at, 0, ...blk);
  }
  return out;
}

/** The arrStack in reading order; a panel that has no row yet (just opened) gets its own. */
const arrStack = () => { const o = stackOrder(); let n = o.reduce((m, q) => Math.max(m, q.riga ?? -1), -1) + 1; o.forEach(q => { if (q.riga == null) q.riga = n++; }); return o; };

/** @type {{ n: number, s: any[] }[]} */ const arrHist = [];
const arrSnap = () => { arrHist.push({ n: ++HSEQ, s: arrKeys(tabPanels().filter(q => q.full)) }); if (arrHist.length > 20) arrHist.shift(); };
const arrRelay = (/** @type {any} */ keep) => { placeRows(arrStack()); E.panels.sort(LAYOUT.ordine); if (window.DDA) DDA.sync(); fitHost(); if (keep && keep.el) front(keep.el); uiSave(); };

const arrLive = document.createElement("div"); arrLive.id = "arrange-live"; arrLive.setAttribute("role", "status"); arrLive.setAttribute("aria-live", "polite"); document.body.appendChild(arrLive);
const arrSay = (/** @type {string} */ m) => { arrLive.textContent = ""; setTimeout(() => { arrLive.textContent = m; }, 30); };
function arrWhere(/** @type {any} */ p) {
  const rows = LAYOUT.righeDi(arrStack()), r = rows.find(x => x.includes(p));
  return r ? ARR_T("arrange.where", { n: p.num ?? "", r: rows.indexOf(r) + 1, t: rows.length, i: r.indexOf(p) + 1, m: r.length }) : "";
}

/** Do a move on the real panels; returns false (and says so) when the move does not fit. */
function arrRun(/** @type {any} */ p, /** @type {any} */ d, /** @type {boolean} */ wide) {
  const before = arrKeys(tabPanels().filter(q => q.full)), grp = wide && !d.solo ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id];
  const order = arrStack(), res = arrRedo(order, p.id, grp, d, hostWidth());
  if (!res) { arrSay(ARR_T("arrange.zone.nospace")); return false; }
  arrHist.push({ n: ++HSEQ, s: before }); if (arrHist.length > 20) arrHist.shift();
  placeRows(res); arrRelay(p); numberPanels(); arrSay(ARR_T("arrange.done") + ". " + arrWhere(p));
  return true;
}
function arrUndo() {
  const h = arrHist.pop(); if (!h) return;
  h.s.forEach(s => { const q = E.panels.find(x => x.id === s.id); if (q) { q.x = s.x; q.y = s.y; q.w = s.w; q.h = s.h; q.riga = s.riga; q.quota = s.quota; } });
  arrRelay(null); numberPanels(); arrSay(ARR_T("arrange.undone"));
}
const arrLast = () => arrHist.length ? arrHist[arrHist.length - 1].n : 0;

// ---- destination under the pointer (host coordinates) -> move: a cross on the target panel, above / below in its top and bottom 30 %, left / right in the middle band
function arrZona(/** @type {any[]} */ order, /** @type {number[]} */ grp, /** @type {number} */ px, /** @type {number} */ py) {
  const rest = order.filter(q => !grp.includes(q.id)), t = rest.find(q => px >= q.x && px < q.x + q.w && py >= q.y && py < q.y + q.h);
  if (t) { const fx = (px - t.x) / t.w, fy = (py - t.y) / t.h; return fy < 0.3 ? { k: "su", t: t.id } : fy > 0.7 ? { k: "giu", t: t.id } : { k: fx < 0.5 ? "sx" : "dx", t: t.id }; }
  const next = rest.find(q => q.y > py);
  return next ? { k: "su", t: next.id } : { k: "fine" };
}
/** @type {HTMLElement | null} */ let arrPrev = null;
const arrShowPrev = (/** @type {{x:number,y:number,w:number,h:number} | null} */ r, /** @type {string} */ arrLabel) => {
  if (!r) { if (arrPrev) { arrPrev.remove(); arrPrev = null; } return; }
  if (!arrPrev) { arrPrev = document.createElement("div"); arrPrev.setAttribute("aria-hidden", "true"); Q("#dpanels").appendChild(arrPrev); }
  arrPrev.className = "arrange-prev" + (arrLabel === ARR_T("arrange.zone.nospace") ? " no" : "");
  Object.assign(arrPrev.style, { left: r.x + "px", top: r.y + "px", width: r.w + "px", height: r.h + "px" }); arrPrev.textContent = arrLabel;
};
const arrLabel = (/** @type {any} */ d) => ARR_T(d.k === "sx" ? "arrange.zone.left" : d.k === "dx" ? "arrange.zone.right" : d.k === "su" ? "arrange.zone.above" : "arrange.zone.below");
function arrDragGrip(/** @type {any} */ p, /** @type {PointerEvent} */ e0, /** @type {any} */ g) {
  const coarse = e0.pointerType !== "mouse", sx = e0.clientX, sy = e0.clientY;
  let on = false, key = "", d = /** @type {any} */ (null);
  const grpOf = (/** @type {any} */ d0) => (d0 && (d0.k === "sx" || d0.k === "dx" || d0.solo) ? [p.id] : blockOf(p).grp.map((/** @type {any} */ q) => q.id));
  const upd = (/** @type {PointerEvent} */ ev) => {
    const h = Q("#dpanels").getBoundingClientRect(), px = ev.clientX - h.left, py = ev.clientY - h.top, order = arrStack();
    let z = arrZona(order, [p.id], px, py);
    // dropped on its own spectrum: the panel moves alone, past it
    if (z.k !== "sx" && z.k !== "dx") { if (z.t != null && grpOf(null).includes(z.t)) z.solo = true; else z = arrZona(order, grpOf(z), px, py); }
    const kk = z.k + ":" + z.t + (z.solo ? "s" : ""); if (kk === key) return; key = kk; d = z;
    const cp = arrKeys(order), grp = grpOf(z), res = arrRedo(cp, p.id, grp, z, hostWidth());
    if (!res) { const t = order.find(q => q.id === z.t); arrShowPrev(t ? { x: t.x, y: t.y, w: t.w, h: t.h } : null, ARR_T("arrange.zone.nospace")); d = null; return; }
    const gm = LAYOUT.colloca(res, hostWidth(), pnlGap(), minW), r = gm.pos.get(res.find(q => q.id === p.id));
    arrShowPrev(r || null, arrLabel(z));
  };
  const stop = (/** @type {boolean} */ commit) => {
    g.removeEventListener("pointermove", mv); g.removeEventListener("pointerup", up); g.removeEventListener("pointercancel", cancel); document.removeEventListener("keydown", esc, true);
    try { g.releasePointerCapture(e0.pointerId); } catch (_) { /* already released */ }
    p.el.classList.remove("arrange-drag"); arrShowPrev(null, "");
    if (on) { g._drag = true; setTimeout(() => { g._drag = false; }, 0); }
    if (commit && on && d) arrRun(p, d, d.k !== "sx" && d.k !== "dx");
  };
  const mv = (/** @type {PointerEvent} */ ev) => {
    if (!on && (coarse || Math.hypot(ev.clientX - sx, ev.clientY - sy) >= 6)) { on = true; p.el.classList.add("arrange-drag"); }
    if (on) upd(ev);
  };
  const up = () => stop(true), cancel = () => stop(false);
  const esc = (/** @type {KeyboardEvent} */ ev) => { if (ev.key === "Escape") { ev.stopPropagation(); stop(false); } };
  g.setPointerCapture(e0.pointerId);
  g.addEventListener("pointermove", mv); g.addEventListener("pointerup", up); g.addEventListener("pointercancel", cancel); document.addEventListener("keydown", esc, true);
}

// ---- arrMenu of the arrGrip (the alternative to dragging) and the keyboard
  function arrVertical(/** @type {any} */ p, /** @type {number} */ dir) {         // one row up / down
  const grp = blockOf(p).grp, order = arrStack().filter(q => !grp.includes(q)), rows = LAYOUT.righeDi(order), all = arrStack();
  const near = all[all.indexOf(p) + dir];
  // its own spectrum is next to it: swap with it
  if (near && grp.includes(near) && near.riga !== p.riga) return { k: dir < 0 ? "su" : "giu", t: near.id, solo: true };
  const slot = rows.filter(r => all.indexOf(r[0]) < all.indexOf(p)).length, mates = all.filter(q => q !== p && q.riga === p.riga);
  // leaving its row: right above / below it
  if (mates.length) return { k: dir < 0 ? "su" : "giu", t: mates[0].id };
  const r = rows[dir < 0 ? slot - 1 : slot]; return r ? { k: dir < 0 ? "su" : "giu", t: r[0].id } : null;
}
function arrNeighbour(/** @type {any} */ p, /** @type {number} */ dir) {
  const o = arrStack(), i = o.indexOf(p) + dir, t = o[i];
  return t && !blockOf(p).grp.includes(t) ? { k: dir < 0 ? "sx" : "dx", t: t.id } : null;
}
/** @type {any} */ let arrMenu = null;
function arrCloseMenu(/** @type {boolean} */ back) { if (!arrMenu) return; const g = arrMenu._grip; arrMenu.remove(); arrMenu = null; g.setAttribute("aria-expanded", "false"); if (back) g.focus(); }
function arrOpenMenu(/** @type {any} */ p, /** @type {any} */ g) {
  if (arrMenu) { arrCloseMenu(false); return; }
  const W = hostWidth(), order = arrStack(), item = (/** @type {string} */ txt, /** @type {any} */ d, /** @type {boolean} */ wide) => {
    const b = document.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitem"); b.textContent = txt;
    const ok = !!d && !!arrRedo(arrKeys(order), p.id, wide ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id], d, W);
    if (!ok) b.setAttribute("aria-disabled", "true"); else b.onclick = () => { arrCloseMenu(false); arrRun(p, d, wide); g.focus(); };
    return b;
  };
  arrMenu = document.createElement("div"); arrMenu.className = "arrange-menu"; arrMenu.setAttribute("role", "menu"); arrMenu.setAttribute("aria-label", ARR_T("arrange.grip")); arrMenu._grip = g;
  const others = order.filter(q => q !== p && !blockOf(p).grp.includes(q) && arrEligible(q));
  if (others.length) {
    const l = document.createElement("span"); l.className = "aff-lbl"; l.textContent = ARR_T("arrange.menu.beside"); arrMenu.appendChild(l);
    others.forEach(q => arrMenu.appendChild(item(`${q.num ?? ""} · ${q.el.querySelector(".ttl").textContent}`, { k: p.y < q.y || p.y === q.y && p.x < q.x ? "sx" : "dx", t: q.id }, false)));
  }
  const mate = order.find(q => q !== p && q.riga === p.riga);
  arrMenu.appendChild(item(ARR_T("arrange.menu.up"), arrVertical(p, -1), true)); arrMenu.appendChild(item(ARR_T("arrange.menu.down"), arrVertical(p, 1), true));
  arrMenu.appendChild(item(ARR_T("arrange.menu.full"), mate ? { k: "giu", t: mate.id } : null, true));
  const eq = document.createElement("button"); eq.type = "button"; eq.setAttribute("role", "menuitem"); eq.textContent = ARR_T("arrange.menu.equal");
  if (!mate) eq.setAttribute("aria-disabled", "true"); else eq.onclick = () => { arrCloseMenu(false); arrRun(p, { k: "uguali" }, false); g.focus(); };
  arrMenu.appendChild(eq);
  const rs = document.createElement("button"); rs.type = "button"; rs.setAttribute("role", "menuitem"); rs.textContent = ARR_T("arrange.menu.reset");
  if (!arrSplit()) rs.setAttribute("aria-disabled", "true"); else rs.onclick = () => { arrCloseMenu(false); arrRestore(); g.focus(); };
  arrMenu.appendChild(rs); document.body.appendChild(arrMenu); g.setAttribute("aria-expanded", "true");
  const r = g.getBoundingClientRect(); arrMenu.style.left = Math.max(4, Math.min(r.left, innerWidth - arrMenu.offsetWidth - 4)) + "px"; arrMenu.style.top = Math.min(r.bottom + 2, innerHeight - arrMenu.offsetHeight - 4) + "px";
  const btns = () => /** @type {HTMLElement[]} */ ([...arrMenu.querySelectorAll("button")]);
  arrMenu.addEventListener("keydown", e => {
    const b = btns(), i = b.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    if (e.key === "Escape") { e.preventDefault(); arrCloseMenu(true); }
    else if (e.key === "ArrowDown") { e.preventDefault(); b[(i + 1) % b.length].focus(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); b[(i - 1 + b.length) % b.length].focus(); }
    else if (e.key === "Tab") arrCloseMenu(false);
  });
  closeOutside(arrMenu, e => g.contains(e.target), () => arrCloseMenu(false), true);
  (btns().find(b => b.getAttribute("aria-disabled") !== "true") || btns()[0]).focus();
}
function arrGrip(/** @type {any} */ p) {
  /** @type {any} */ const g = document.createElement("button"); g.type = "button"; g.className = "arrange-grip"; g.textContent = "⠿";
  g.title = ARR_T("arrange.grip"); g.setAttribute("aria-label", ARR_T("arrange.grip")); g.setAttribute("aria-haspopup", "menu"); g.setAttribute("aria-expanded", "false");
  g.addEventListener("pointerdown", e => { if (e.button === 0 && arrEligible(p)) { setActive(p); front(p.el); arrDragGrip(p, e, g); } });
  g.addEventListener("click", () => { if (!g._drag) arrOpenMenu(p, g); });
  g.addEventListener("keydown", e => {
    if (!e.altKey || !/^Arrow/.test(e.key)) return;
    e.preventDefault();
    const d = e.key === "ArrowUp" ? arrVertical(p, -1) : e.key === "ArrowDown" ? arrVertical(p, 1) : arrNeighbour(p, e.key === "ArrowLeft" ? -1 : 1);
    if (d) arrRun(p, d, d.k !== "sx" && d.k !== "dx"); else arrSay(ARR_T("arrange.zone.nospace"));
    g.focus();
  });
  p.el.querySelector(".hd").prepend(g); p._grip = g;
}

// ---- dividers between side-by-side neighbours
let arrBusy = false;
const AFF_MIN_KEY = 16, AFF_BIG_KEY = 48;
function arrResize(/** @type {any[]} */ row, /** @type {number} */ i, /** @type {number} */ dx) {
  const a = row[i], b = row[i + 1], tot = a.w + b.w, w0 = Math.max(minW(a), Math.min(tot - minW(b), a.w + dx));
  a.w = w0; b.x = a.x + a.w + pnlGap(); b.w = tot - w0; apply(a); apply(b);
}
const arrCommitRow = (/** @type {any[]} */ row) => { const s = row.reduce((m, q) => m + q.w, 0); row.forEach(q => { q.quota = q.w / s; }); arrRelay(null); };
function arrSep(/** @type {any[]} */ row, /** @type {number} */ i) {
  const a = row[i], b = row[i + 1], /** @type {any} */ el = document.createElement("button");
  el.type = "button"; el.className = "arrange-sep"; el.setAttribute("role", "separator"); el.setAttribute("aria-orientation", "vertical"); el.setAttribute("aria-label", ARR_T("arrange.sep"));
  const pos = () => { const h = Math.min(a.h, 64); el.style.left = a.x + a.w + pnlGap() / 2 - el.offsetWidth / 2 + "px"; el.style.top = a.y + (a.h - h) / 2 + "px"; el.style.height = h + "px"; el.setAttribute("aria-valuenow", String(Math.round(100 * a.w / (a.w + b.w)))); };
  Q("#dpanels").appendChild(el); pos();
  el.addEventListener("pointerdown", e => {
    if (e.button !== 0) return;
    e.preventDefault(); el.setPointerCapture(e.pointerId); el.classList.add("on"); arrBusy = true; arrSnap();
    const x0 = e.clientX, w0 = a.w;
    const mv = (/** @type {PointerEvent} */ ev) => { arrResize(row, i, w0 + ev.clientX - x0 - a.w); pos(); };
    const up = () => { el.removeEventListener("pointermove", mv); el.removeEventListener("pointerup", up); el.removeEventListener("pointercancel", up); el.classList.remove("on"); arrBusy = false; arrCommitRow(row); };
    el.addEventListener("pointermove", mv); el.addEventListener("pointerup", up); el.addEventListener("pointercancel", up);
  });
  el.addEventListener("keydown", e => {
    const step = e.shiftKey ? AFF_BIG_KEY : AFF_MIN_KEY, tot = a.w + b.w;
    let dx = null;
    if (e.key === "ArrowLeft") dx = -step; else if (e.key === "ArrowRight") dx = step;
    else if (e.key === "Home") dx = minW(a) - a.w; else if (e.key === "End") dx = tot - minW(b) - a.w;
    else if (e.key === "Enter") dx = (tot - pnlGap()) / 2 - a.w;
    if (dx == null) return;
    e.preventDefault(); arrSnap(); arrResize(row, i, dx); arrCommitRow(row);
  });
  el.addEventListener("dblclick", () => { arrSnap(); row.forEach(q => { q.quota = null; }); arrRelay(null); });
  el._key = a.id + ":" + b.id; return el;
}
/** After every layout: the grips of the panels that can move, the dividers of the rows. */
function arrSync() {
  if (arrBusy) return;
  const host = document.getElementById("dpanels"); if (!host) return;
  // a rebuilt divider keeps the focus
  const was = document.activeElement && document.activeElement.classList.contains("arrange-sep") ? /** @type {any} */ (document.activeElement)._key : null;
  host.querySelectorAll(".arrange-sep").forEach(e => e.remove());
  tabPanels().forEach(q => { if (q._grip) q._grip.hidden = !arrEligible(q); });
  if (fsPanel()) return;
  LAYOUT.righeDi(arrStack()).forEach(r => { if (r.length > 1 && r.every((/** @type {any} */ q) => q.y === r[0].y)) for (let i = 0; i < r.length - 1; i++) { const e = arrSep(r, i); if (was === e._key) e.focus(); } });
}
// ---- registry actions/1: the arrangement of the panel the menu was opened on (group disp, the views of the graphs), the same entries as the menu of the grip; also in the palette
const arrWide = (/** @type {any} */ d) => !!d && d.k !== "sx" && d.k !== "dx" && d.k !== "uguali";
const arrOkFor = (/** @type {any} */ p, /** @type {any} */ d) => !!d && !!arrRedo(arrKeys(arrStack()), p.id, arrWide(d) && !d.solo ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id], d, hostWidth());
const arrMate = (/** @type {any} */ p) => arrStack().find(q => q !== p && q.riga === p.riga);
const arrOthers = (/** @type {any} */ p) => arrStack().filter(q => q !== p && !blockOf(p).grp.includes(q) && arrEligible(q));
const arrRowOf = (/** @type {any} */ q) => `${q.num ?? ""} \u00b7 ${q.el.querySelector(".ttl").textContent}`;
function arrRestore() {
  const l = tabPanels().filter(q => q.full); arrSnap();
  l.forEach(q => { q.riga = null; q.quota = null; q.x = 0; q.w = hostWidth(); });
  arrRelay(null); numberPanels(); arrSay(ARR_T("arrange.reset.done"));
}
const arrSplit = () => tabPanels().filter(q => q.full).some(q => q.riga != null && tabPanels().some(o => o !== q && o.full && o.riga === q.riga) || (q.quota != null && q.quota !== 1));
if (typeof AZIONI !== "undefined") {
  const reg = (/** @type {any} */ o) => AZIONI.register({ group: "disp", views: ["*"], visible: (/** @type {any} */ c) => c.view !== "map" && !!c.p && !!c.p.el && !!c.p.full && arrEligible(c.p), ...o });
  reg({ id: "disp.arrange", order: 1, menu: false, name: () => ARR_T("arrange.menu.beside"), available: (/** @type {any} */ c) => arrOthers(c.p).length > 0,
    run: (/** @type {any} */ c) => { if (c.p._grip) arrOpenMenu(c.p, c.p._grip); },
    rows: (/** @type {any} */ c) => {
      const p = c.p, o = arrOthers(p);
      if (!o.length) return [{ name: ARR_T("arrange.menu.beside"), dim: true, info: true, run: () => 0 }];
      return o.map(q => { const d = { k: p.y < q.y || p.y === q.y && p.x < q.x ? "sx" : "dx", t: q.id }; return { name: ARR_T("arrange.menu.beside") + " " + arrRowOf(q), dim: !arrOkFor(p, d), run: () => { arrRun(p, d, false); } }; });
    } });
  // only Move up / Move down are in the context menu (one row); the rest is the grip menu, the palette and dragging
  const via = (/** @type {string} */ id, /** @type {number} */ order, /** @type {string} */ key, /** @type {(p: any) => any} */ move, /** @type {any} */ extra = { menu: false }) =>
    reg({ id, order, ...extra, name: () => ARR_T(key), available: (/** @type {any} */ c) => arrOkFor(c.p, move(c.p)), run: (/** @type {any} */ c) => { const d = move(c.p); if (d) arrRun(c.p, d, arrWide(d)); } });
  via("disp.su", 2, "arrange.menu.up", p => arrVertical(p, -1), { fila: "mv" });
  via("disp.giu", 3, "arrange.menu.down", p => arrVertical(p, 1), { fila: "mv" });
  via("disp.intera", 4, "arrange.menu.full", p => { const m = arrMate(p); return m ? { k: "giu", t: m.id } : null; });
  via("disp.uguali", 5, "arrange.menu.equal", p => arrMate(p) ? { k: "uguali" } : null);
  reg({ id: "disp.ripristina", order: 6, menu: false, name: () => ARR_T("arrange.menu.reset"), available: () => arrSplit(), run: () => arrRestore() });
}

const ARRANGE = { grip: arrGrip, sync: arrSync, undo: arrUndo, last: arrLast, run: arrRun, redo: arrRedo, restore: arrRestore, eligible: arrEligible };
window.ARRANGE = ARRANGE;
if (window.BARRA && BARRA.late) BARRA.late();
