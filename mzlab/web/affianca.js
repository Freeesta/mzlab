// @ts-check
// Side-by-side panels of the Dati view: the affGrip in the header (drag with zones, affMenu, Alt+arrows), the dividers between neighbours, undo (Ctrl/Cmd+Z).
// Classic script, loaded after pannelli.js (shared globals). The rows themselves (riga, quota, geometry) are in layout.js; here only the gestures.
const AFF_T = (/** @type {string} */ k, /** @type {any} */ v) => I18N.t(k, v);
const affCss = document.createElement("style");
affCss.textContent = ".aff-grip{flex:none;min-width:24px;min-height:24px;padding:0 2px;margin-right:2px;border:0;background:transparent;color:var(--muted);cursor:grab;touch-action:none;opacity:0;line-height:1;font-size:14px}" +
  ".pnl:hover .aff-grip,.aff-grip:focus-visible,.aff-grip[aria-expanded=true]{opacity:1}.aff-grip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.aff-grip{opacity:1;min-width:40px;min-height:40px}}" +
  ".pnl.aff-drag{opacity:.45}.aff-prev{position:absolute;z-index:30;box-sizing:border-box;pointer-events:none;border:2px dashed var(--accent);background:var(--sel);opacity:.8;display:flex;align-items:center;justify-content:center;color:var(--ink);font-size:13px;font-weight:600}" +
  ".aff-prev.no{border-color:var(--muted);background:var(--soft);color:var(--muted)}" +
  ".aff-menu{position:fixed;z-index:10005;min-width:200px;max-width:min(320px,94vw);max-height:70vh;overflow:auto;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);padding:4px}" +
  ".aff-menu button{display:block;width:100%;min-height:28px;text-align:left;border:0;background:transparent;color:var(--ink);padding:3px 8px;border-radius:4px}.aff-menu button:hover,.aff-menu button:focus-visible{background:var(--sel);outline:2px solid var(--accent);outline-offset:-2px}.aff-menu button[aria-disabled=true]{color:var(--muted);cursor:default}" +
  ".aff-menu .aff-lbl{display:block;padding:4px 8px 2px;font-size:11px;color:var(--muted)}" +
  ".aff-sep{position:absolute;z-index:6;width:24px;cursor:col-resize;touch-action:none;display:flex;justify-content:center;align-items:stretch;border:0;padding:0;background:transparent}.aff-sep::after{content:'';width:4px;border-radius:2px;background:var(--line)}" +
  ".aff-sep:hover::after,.aff-sep:focus-visible::after,.aff-sep.on::after{background:var(--accent)}.aff-sep:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.aff-sep{width:40px}}" +
  "#aff-bar{position:fixed;z-index:100001;bottom:20px;left:50%;transform:translateX(-50%);display:flex;gap:10px;align-items:center;background:var(--ink);color:var(--panel);padding:6px 12px;border-radius:6px;box-shadow:0 3px 12px rgba(0,0,0,.25)}#aff-bar button{min-height:24px;padding:0 8px}" +
  ".aff-grip.aff-hl{opacity:1;outline:2px solid var(--accent);outline-offset:-2px;border-radius:4px}" +
  "#aff-hint{position:absolute;z-index:32;display:flex;gap:8px;align-items:flex-start;max-width:min(320px,calc(100% - 16px));box-sizing:border-box;padding:8px 8px 8px 12px;background:var(--panel);color:var(--ink);border:1px solid var(--accent);border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,.2);font-size:13px;line-height:1.4}" +
  "#aff-hint::before{content:'';position:absolute;top:-6px;left:var(--ha,14px);width:10px;height:10px;background:var(--panel);border-left:1px solid var(--accent);border-top:1px solid var(--accent);transform:rotate(45deg)}" +
  "#aff-hint button{flex:none;min-width:24px;min-height:24px;border:0;background:none;color:var(--muted);font-size:16px;line-height:1;padding:0 4px;cursor:pointer}@media (pointer:coarse){#aff-hint button{min-width:40px;min-height:40px}}" +
  "#aff-live{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}";
document.head.appendChild(affCss);

const affEligible = (/** @type {any} */ p) => !!(p.full && p.el && !dockedP(p) && !hasDocked(p) && !p.el.classList.contains("max"));
const affKeys = (/** @type {any[]} */ l) => l.map(q => ({ id: q.id, x: q.x, y: q.y, w: q.w, h: q.h, riga: q.riga, quota: q.quota, tab: q.tab, full: q.full }));
const affEntra = (/** @type {any[]} */ row, /** @type {number} */ W) => row.length <= LAYOUT.MAX_ROW && row.every(q => minW(q) * row.length + pnlGap() * (row.length - 1) <= W);

/**
 * The move `d` applied to an ordered affStack (real panels or copies of them). `grp` = ids that travel together (affVertical moves).
 * d.k: sx / dx (beside d.t), su / giu (own row before / after the row of d.t), fine (own row at the end), uguali (equal widths of the row of p).
 * @returns {any[] | null} the new order, or null when it does not fit (nothing is touched then)
 */
function affRifai(/** @type {any[]} */ order, /** @type {number} */ pid, /** @type {number[]} */ grp, /** @type {any} */ d, /** @type {number} */ W) {
  const p = order.find(q => q.id === pid), rest = order.filter(q => !grp.includes(q.id)), mates = (/** @type {any} */ q) => rest.filter(x => x.riga === q.riga);
  if (!p) return null;
  if (d.k === "uguali") { order.filter(q => q.riga === p.riga).forEach(q => { q.quota = null; }); return order; }
  const t = d.t != null ? rest.find(q => q.id === d.t) : null;
  if (d.k !== "fine" && (!t || t === p)) return null;
  const side = d.k === "sx" || d.k === "dx";
  if (side && !affEntra([...mates(t), p], W)) return null;
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

/** The affStack in reading order; a panel that has no row yet (just opened) gets its own. */
const affStack = () => { const o = stackOrder(); let n = o.reduce((m, q) => Math.max(m, q.riga ?? -1), -1) + 1; o.forEach(q => { if (q.riga == null) q.riga = n++; }); return o; };

/** @type {{ n: number, s: any[] }[]} */ const affHist = [];
const affSnap = () => { affHist.push({ n: ++HSEQ, s: affKeys(tabPanels().filter(q => q.full)) }); if (affHist.length > 20) affHist.shift(); };
const affRelay = (/** @type {any} */ keep) => { placeRows(affStack()); E.panels.sort(LAYOUT.ordine); if (window.DDA) DDA.sync(); fitHost(); if (keep && keep.el) front(keep.el); uiSave(); };

const affLive = document.createElement("div"); affLive.id = "aff-live"; affLive.setAttribute("role", "status"); affLive.setAttribute("aria-live", "polite"); document.body.appendChild(affLive);
const affSay = (/** @type {string} */ m) => { affLive.textContent = ""; setTimeout(() => { affLive.textContent = m; }, 30); };
function affWhere(/** @type {any} */ p) {
  const rows = LAYOUT.righeDi(affStack()), r = rows.find(x => x.includes(p));
  return r ? AFF_T("aff.where", { n: p.num ?? "", r: rows.indexOf(r) + 1, t: rows.length, i: r.indexOf(p) + 1, m: r.length }) : "";
}

/** @type {any} */ let affBarTimer = null;
function affBar(/** @type {string} */ msg) {
  const old = document.getElementById("aff-bar"); if (old) old.remove();
  clearTimeout(affBarTimer);
  const b = document.createElement("div"); b.id = "aff-bar"; b.setAttribute("role", "status");
  b.innerHTML = `<span>${EH(msg)}</span><button type="button">${EH(AFF_T("aff.undo"))}</button>`;
  b.querySelector("button").onclick = () => { b.remove(); affAnnulla(); };
  document.body.appendChild(b); affBarTimer = setTimeout(() => b.remove(), 6000);
}
/** Do a move on the real panels; returns false (and says so) when the move does not fit. */
function affFai(/** @type {any} */ p, /** @type {any} */ d, /** @type {boolean} */ wide) {
  const before = affKeys(tabPanels().filter(q => q.full)), grp = wide && !d.solo ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id];
  const order = affStack(), res = affRifai(order, p.id, grp, d, hostWidth());
  if (!res) { affSay(AFF_T("aff.zone.nospace")); return false; }
  affHist.push({ n: ++HSEQ, s: before }); if (affHist.length > 20) affHist.shift();
  placeRows(res); affRelay(p); numberPanels(); affSay(affWhere(p)); affBar(AFF_T("aff.done")); if (d.k === "sx" || d.k === "dx") affHintClose();
  return true;
}
function affAnnulla() {
  const h = affHist.pop(); if (!h) return;
  h.s.forEach(s => { const q = E.panels.find(x => x.id === s.id); if (q) { q.x = s.x; q.y = s.y; q.w = s.w; q.h = s.h; q.riga = s.riga; q.quota = s.quota; } });
  affRelay(null); numberPanels(); affSay(AFF_T("aff.undone"));
}
const affUltimo = () => affHist.length ? affHist[affHist.length - 1].n : 0;

// ---- destination under the pointer (host coordinates) -> move: a cross on the target panel, above / below in its top and bottom 30 %, left / right in the middle band
function affZona(/** @type {any[]} */ order, /** @type {number[]} */ grp, /** @type {number} */ px, /** @type {number} */ py) {
  const rest = order.filter(q => !grp.includes(q.id)), t = rest.find(q => px >= q.x && px < q.x + q.w && py >= q.y && py < q.y + q.h);
  if (t) { const fx = (px - t.x) / t.w, fy = (py - t.y) / t.h; return fy < 0.3 ? { k: "su", t: t.id } : fy > 0.7 ? { k: "giu", t: t.id } : { k: fx < 0.5 ? "sx" : "dx", t: t.id }; }
  const next = rest.find(q => q.y > py);
  return next ? { k: "su", t: next.id } : { k: "fine" };
}
/** @type {HTMLElement | null} */ let affPrev = null;
const affShowPrev = (/** @type {{x:number,y:number,w:number,h:number} | null} */ r, /** @type {string} */ affLabel) => {
  if (!r) { if (affPrev) { affPrev.remove(); affPrev = null; } return; }
  if (!affPrev) { affPrev = document.createElement("div"); affPrev.setAttribute("aria-hidden", "true"); Q("#dpanels").appendChild(affPrev); }
  affPrev.className = "aff-prev" + (affLabel === AFF_T("aff.zone.nospace") ? " no" : "");
  Object.assign(affPrev.style, { left: r.x + "px", top: r.y + "px", width: r.w + "px", height: r.h + "px" }); affPrev.textContent = affLabel;
};
const affLabel = (/** @type {any} */ d) => AFF_T(d.k === "sx" ? "aff.zone.left" : d.k === "dx" ? "aff.zone.right" : d.k === "su" ? "aff.zone.above" : "aff.zone.below");
function affDragGrip(/** @type {any} */ p, /** @type {PointerEvent} */ e0, /** @type {any} */ g) {
  const coarse = e0.pointerType !== "mouse", sx = e0.clientX, sy = e0.clientY;
  let on = false, key = "", d = /** @type {any} */ (null);
  const grpOf = (/** @type {any} */ d0) => (d0 && (d0.k === "sx" || d0.k === "dx" || d0.solo) ? [p.id] : blockOf(p).grp.map((/** @type {any} */ q) => q.id));
  const upd = (/** @type {PointerEvent} */ ev) => {
    const h = Q("#dpanels").getBoundingClientRect(), px = ev.clientX - h.left, py = ev.clientY - h.top, order = affStack();
    let z = affZona(order, [p.id], px, py);
    // dropped on its own spectrum: the panel moves alone, past it
    if (z.k !== "sx" && z.k !== "dx") { if (z.t != null && grpOf(null).includes(z.t)) z.solo = true; else z = affZona(order, grpOf(z), px, py); }
    const kk = z.k + ":" + z.t + (z.solo ? "s" : ""); if (kk === key) return; key = kk; d = z;
    const cp = affKeys(order), grp = grpOf(z), res = affRifai(cp, p.id, grp, z, hostWidth());
    if (!res) { const t = order.find(q => q.id === z.t); affShowPrev(t ? { x: t.x, y: t.y, w: t.w, h: t.h } : null, AFF_T("aff.zone.nospace")); d = null; return; }
    const gm = LAYOUT.colloca(res, hostWidth(), pnlGap(), minW), r = gm.pos.get(res.find(q => q.id === p.id));
    affShowPrev(r || null, affLabel(z));
  };
  const stop = (/** @type {boolean} */ commit) => {
    g.removeEventListener("pointermove", mv); g.removeEventListener("pointerup", up); g.removeEventListener("pointercancel", cancel); document.removeEventListener("keydown", esc, true);
    try { g.releasePointerCapture(e0.pointerId); } catch (_) { /* already released */ }
    p.el.classList.remove("aff-drag"); affShowPrev(null, "");
    if (on) { g._drag = true; setTimeout(() => { g._drag = false; }, 0); }
    if (commit && on && d) affFai(p, d, d.k !== "sx" && d.k !== "dx");
  };
  const mv = (/** @type {PointerEvent} */ ev) => {
    if (!on && (coarse || Math.hypot(ev.clientX - sx, ev.clientY - sy) >= 6)) { on = true; p.el.classList.add("aff-drag"); }
    if (on) upd(ev);
  };
  const up = () => stop(true), cancel = () => stop(false);
  const esc = (/** @type {KeyboardEvent} */ ev) => { if (ev.key === "Escape") { ev.stopPropagation(); stop(false); } };
  g.setPointerCapture(e0.pointerId);
  g.addEventListener("pointermove", mv); g.addEventListener("pointerup", up); g.addEventListener("pointercancel", cancel); document.addEventListener("keydown", esc, true);
}

// ---- affMenu of the affGrip (the alternative to dragging) and the keyboard
  function affVertical(/** @type {any} */ p, /** @type {number} */ dir) {         // one row up / down
  const grp = blockOf(p).grp, order = affStack().filter(q => !grp.includes(q)), rows = LAYOUT.righeDi(order), all = affStack();
  const near = all[all.indexOf(p) + dir];
  // its own spectrum is next to it: swap with it
  if (near && grp.includes(near) && near.riga !== p.riga) return { k: dir < 0 ? "su" : "giu", t: near.id, solo: true };
  const slot = rows.filter(r => all.indexOf(r[0]) < all.indexOf(p)).length, mates = all.filter(q => q !== p && q.riga === p.riga);
  // leaving its row: right above / below it
  if (mates.length) return { k: dir < 0 ? "su" : "giu", t: mates[0].id };
  const r = rows[dir < 0 ? slot - 1 : slot]; return r ? { k: dir < 0 ? "su" : "giu", t: r[0].id } : null;
}
function affNeighbour(/** @type {any} */ p, /** @type {number} */ dir) {
  const o = affStack(), i = o.indexOf(p) + dir, t = o[i];
  return t && !blockOf(p).grp.includes(t) ? { k: dir < 0 ? "sx" : "dx", t: t.id } : null;
}
/** @type {any} */ let affMenu = null;
function affCloseMenu(/** @type {boolean} */ back) { if (!affMenu) return; const g = affMenu._grip; affMenu.remove(); affMenu = null; g.setAttribute("aria-expanded", "false"); if (back) g.focus(); }
function affOpenMenu(/** @type {any} */ p, /** @type {any} */ g) {
  if (affMenu) { affCloseMenu(false); return; }
  const W = hostWidth(), order = affStack(), item = (/** @type {string} */ txt, /** @type {any} */ d, /** @type {boolean} */ wide) => {
    const b = document.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitem"); b.textContent = txt;
    const ok = !!d && !!affRifai(affKeys(order), p.id, wide ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id], d, W);
    if (!ok) b.setAttribute("aria-disabled", "true"); else b.onclick = () => { affCloseMenu(false); affFai(p, d, wide); g.focus(); };
    return b;
  };
  affMenu = document.createElement("div"); affMenu.className = "aff-menu"; affMenu.setAttribute("role", "menu"); affMenu.setAttribute("aria-label", AFF_T("aff.grip")); affMenu._grip = g;
  const others = order.filter(q => q !== p && !blockOf(p).grp.includes(q) && affEligible(q));
  if (others.length) {
    const l = document.createElement("span"); l.className = "aff-lbl"; l.textContent = AFF_T("aff.menu.beside"); affMenu.appendChild(l);
    others.forEach(q => affMenu.appendChild(item(`${q.num ?? ""} · ${q.el.querySelector(".ttl").textContent}`, { k: p.y < q.y || p.y === q.y && p.x < q.x ? "sx" : "dx", t: q.id }, false)));
  }
  const mate = order.find(q => q !== p && q.riga === p.riga);
  affMenu.appendChild(item(AFF_T("aff.menu.up"), affVertical(p, -1), true)); affMenu.appendChild(item(AFF_T("aff.menu.down"), affVertical(p, 1), true));
  affMenu.appendChild(item(AFF_T("aff.menu.full"), mate ? { k: "giu", t: mate.id } : null, true));
  const eq = document.createElement("button"); eq.type = "button"; eq.setAttribute("role", "menuitem"); eq.textContent = AFF_T("aff.menu.equal");
  if (!mate) eq.setAttribute("aria-disabled", "true"); else eq.onclick = () => { affCloseMenu(false); affFai(p, { k: "uguali" }, false); g.focus(); };
  affMenu.appendChild(eq); document.body.appendChild(affMenu); g.setAttribute("aria-expanded", "true");
  const r = g.getBoundingClientRect(); affMenu.style.left = Math.max(4, Math.min(r.left, innerWidth - affMenu.offsetWidth - 4)) + "px"; affMenu.style.top = Math.min(r.bottom + 2, innerHeight - affMenu.offsetHeight - 4) + "px";
  const btns = () => /** @type {HTMLElement[]} */ ([...affMenu.querySelectorAll("button")]);
  affMenu.addEventListener("keydown", e => {
    const b = btns(), i = b.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    if (e.key === "Escape") { e.preventDefault(); affCloseMenu(true); }
    else if (e.key === "ArrowDown") { e.preventDefault(); b[(i + 1) % b.length].focus(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); b[(i - 1 + b.length) % b.length].focus(); }
    else if (e.key === "Tab") affCloseMenu(false);
  });
  const away = (/** @type {Event} */ e) => { if (affMenu && !affMenu.contains(/** @type {Node} */ (e.target)) && !g.contains(/** @type {Node} */ (e.target))) { affCloseMenu(false); document.removeEventListener("pointerdown", away, true); } };
  setTimeout(() => document.addEventListener("pointerdown", away, true), 0);
  (btns().find(b => b.getAttribute("aria-disabled") !== "true") || btns()[0]).focus();
}
function affGrip(/** @type {any} */ p) {
  /** @type {any} */ const g = document.createElement("button"); g.type = "button"; g.className = "aff-grip"; g.textContent = "⠿";
  g.title = AFF_T("aff.grip"); g.setAttribute("aria-label", AFF_T("aff.grip")); g.setAttribute("aria-haspopup", "menu"); g.setAttribute("aria-expanded", "false");
  g.addEventListener("pointerdown", e => { if (e.button === 0 && affEligible(p)) { setActive(p); front(p.el); affDragGrip(p, e, g); } });
  g.addEventListener("click", () => { if (!g._drag) affOpenMenu(p, g); });
  g.addEventListener("keydown", e => {
    if (!e.altKey || !/^Arrow/.test(e.key)) return;
    e.preventDefault();
    const d = e.key === "ArrowUp" ? affVertical(p, -1) : e.key === "ArrowDown" ? affVertical(p, 1) : affNeighbour(p, e.key === "ArrowLeft" ? -1 : 1);
    if (d) affFai(p, d, d.k !== "sx" && d.k !== "dx"); else affSay(AFF_T("aff.zone.nospace"));
    g.focus();
  });
  p.el.querySelector(".hd").prepend(g); p._grip = g;
}

// ---- dividers between side-by-side neighbours
let affBusy = false;
const AFF_MIN_KEY = 16, AFF_BIG_KEY = 48;
function affResize(/** @type {any[]} */ row, /** @type {number} */ i, /** @type {number} */ dx) {
  const a = row[i], b = row[i + 1], tot = a.w + b.w, w0 = Math.max(minW(a), Math.min(tot - minW(b), a.w + dx));
  a.w = w0; b.x = a.x + a.w + pnlGap(); b.w = tot - w0; apply(a); apply(b);
}
const affCommitRow = (/** @type {any[]} */ row) => { const s = row.reduce((m, q) => m + q.w, 0); row.forEach(q => { q.quota = q.w / s; }); affRelay(null); };
function affSep(/** @type {any[]} */ row, /** @type {number} */ i) {
  const a = row[i], b = row[i + 1], /** @type {any} */ el = document.createElement("button");
  el.type = "button"; el.className = "aff-sep"; el.setAttribute("role", "separator"); el.setAttribute("aria-orientation", "vertical"); el.setAttribute("aria-label", AFF_T("aff.sep"));
  const pos = () => { const h = Math.min(a.h, 64); el.style.left = a.x + a.w + pnlGap() / 2 - el.offsetWidth / 2 + "px"; el.style.top = a.y + (a.h - h) / 2 + "px"; el.style.height = h + "px"; el.setAttribute("aria-valuenow", String(Math.round(100 * a.w / (a.w + b.w)))); };
  Q("#dpanels").appendChild(el); pos();
  el.addEventListener("pointerdown", e => {
    if (e.button !== 0) return;
    e.preventDefault(); el.setPointerCapture(e.pointerId); el.classList.add("on"); affBusy = true; affSnap();
    const x0 = e.clientX, w0 = a.w;
    const mv = (/** @type {PointerEvent} */ ev) => { affResize(row, i, w0 + ev.clientX - x0 - a.w); pos(); };
    const up = () => { el.removeEventListener("pointermove", mv); el.removeEventListener("pointerup", up); el.removeEventListener("pointercancel", up); el.classList.remove("on"); affBusy = false; affCommitRow(row); };
    el.addEventListener("pointermove", mv); el.addEventListener("pointerup", up); el.addEventListener("pointercancel", up);
  });
  el.addEventListener("keydown", e => {
    const step = e.shiftKey ? AFF_BIG_KEY : AFF_MIN_KEY, tot = a.w + b.w;
    let dx = null;
    if (e.key === "ArrowLeft") dx = -step; else if (e.key === "ArrowRight") dx = step;
    else if (e.key === "Home") dx = minW(a) - a.w; else if (e.key === "End") dx = tot - minW(b) - a.w;
    else if (e.key === "Enter") dx = (tot - pnlGap()) / 2 - a.w;
    if (dx == null) return;
    e.preventDefault(); affSnap(); affResize(row, i, dx); affCommitRow(row);
  });
  el.addEventListener("dblclick", () => { affSnap(); row.forEach(q => { q.quota = null; }); affRelay(null); });
  el._key = a.id + ":" + b.id; return el;
}
/** After every layout: the grips of the panels that can move, the dividers of the rows. */
function affSync() {
  if (affBusy) return;
  const host = document.getElementById("dpanels"); if (!host) return;
  // a rebuilt divider keeps the focus
  const was = document.activeElement && document.activeElement.classList.contains("aff-sep") ? /** @type {any} */ (document.activeElement)._key : null;
  host.querySelectorAll(".aff-sep").forEach(e => e.remove());
  tabPanels().forEach(q => { if (q._grip) q._grip.hidden = !affEligible(q); });
  if (fsPanel()) return;
  if (document.getElementById("aff-hint")) affHintPlace(); else if (!affHintSeen() && window.SUGG && affHintTarget()) SUGG.chiedi("affianca", affHintShow, 1500);
  LAYOUT.righeDi(affStack()).forEach(r => { if (r.length > 1 && r.every((/** @type {any} */ q) => q.y === r[0].y)) for (let i = 0; i < r.length - 1; i++) { const e = affSep(r, i); if (was === e._key) e.focus(); } });
}
// ---- hint «drag here to place side by side»: once per browser, when two panels could share a row and no other hint is on screen (suggerimenti.js); × and Esc close it, so does the first side by side
const AFF_K_HINT = NOMI.KEY("pannelli.affianca.hint");
let affHintMem = false;
const affHintSeen = () => { try { return !!localStorage.getItem(AFF_K_HINT); } catch (_) { return affHintMem; } };
const affHintMark = () => { affHintMem = true; try { localStorage.setItem(AFF_K_HINT, "1"); } catch (_) { /* no storage: once per page */ } };
/** The panel to point at: the second of the stack that can move and does not share a row yet (a spectrum under its chromatogram), when the two would fit side by side. */
function affHintTarget() {
  if (window.QQQ_PHONE || typeof S === "undefined" || S.view !== "data" || fsPanel() || affBusy) return null;
  const l = affStack().filter(affEligible);
  if (l.length < 2 || l.some(q => l.some(o => o !== q && o.riga === q.riga)) || !affEntra([l[0], l[1]], hostWidth())) return null;
  return l[1]._grip && !l[1]._grip.hidden ? l[1] : null;
}
function affHintPlace() {
  const el = document.getElementById("aff-hint"), host = document.getElementById("dpanels"), t = affHintTarget();
  if (!el) return;
  if (!t || !host) { affHintClose(); return; }
  const g = t._grip, r = g.getBoundingClientRect(), h = host.getBoundingClientRect(), left = Math.max(4, Math.min(r.left - h.left, host.clientWidth - el.offsetWidth - 4));
  el.style.left = Math.round(left) + "px"; el.style.top = Math.round(r.bottom - h.top + 8) + "px";
  el.style.setProperty("--ha", Math.round(Math.max(10, Math.min(r.left - h.left + r.width / 2 - left, el.offsetWidth - 20)) - 5) + "px");
}
const affHintEsc = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Escape") affHintClose(); };
function affHintClose() {
  document.querySelectorAll(".aff-grip.aff-hl").forEach(g => g.classList.remove("aff-hl"));
  const el = document.getElementById("aff-hint"); if (!el) return;
  el.remove(); document.removeEventListener("keydown", affHintEsc);
}
function affHintShow() {
  const t = affHintTarget(), host = document.getElementById("dpanels");
  if (!t || !host || affHintSeen() || document.getElementById("aff-hint")) return false;
  const el = document.createElement("div"); el.id = "aff-hint"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite");
  const txt = document.createElement("span"); txt.textContent = AFF_T("aff.hint");
  const x = document.createElement("button"); x.type = "button"; x.setAttribute("aria-label", AFF_T("aff.hint.close")); x.textContent = "\u00d7"; x.onclick = affHintClose;
  el.append(txt, x); host.appendChild(el); t._grip.classList.add("aff-hl"); affHintPlace(); affHintMark();
  document.addEventListener("keydown", affHintEsc);
  return true;
}

// ---- registry azioni/1: the arrangement of the panel the menu was opened on (group disp, the views of the graphs), the same entries as the menu of the grip; also in the palette
const affWide = (/** @type {any} */ d) => !!d && d.k !== "sx" && d.k !== "dx" && d.k !== "uguali";
const affOkFor = (/** @type {any} */ p, /** @type {any} */ d) => !!d && !!affRifai(affKeys(affStack()), p.id, affWide(d) && !d.solo ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id], d, hostWidth());
const affMate = (/** @type {any} */ p) => affStack().find(q => q !== p && q.riga === p.riga);
const affOthers = (/** @type {any} */ p) => affStack().filter(q => q !== p && !blockOf(p).grp.includes(q) && affEligible(q));
const affRowOf = (/** @type {any} */ q) => `${q.num ?? ""} \u00b7 ${q.el.querySelector(".ttl").textContent}`;
function affRipristina() {
  const l = tabPanels().filter(q => q.full); affSnap();
  l.forEach(q => { q.riga = null; q.quota = null; q.x = 0; q.w = hostWidth(); });
  affRelay(null); numberPanels(); affSay(AFF_T("aff.reset.done")); affBar(AFF_T("aff.reset.done"));
}
const affSplit = () => tabPanels().filter(q => q.full).some(q => q.riga != null && tabPanels().some(o => o !== q && o.full && o.riga === q.riga) || (q.quota != null && q.quota !== 1));
if (typeof AZIONI !== "undefined") {
  const reg = (/** @type {any} */ o) => AZIONI.registra({ gruppo: "disp", viste: ["*"], visibile: (/** @type {any} */ c) => c.vista !== "map" && !!c.p && !!c.p.el && !!c.p.full && affEligible(c.p), ...o });
  reg({ id: "disp.affianca", ordine: 1, nome: () => AFF_T("aff.menu.beside"), disponibile: (/** @type {any} */ c) => affOthers(c.p).length > 0,
    esegui: (/** @type {any} */ c) => { if (c.p._grip) affOpenMenu(c.p, c.p._grip); },
    voci: (/** @type {any} */ c) => {
      const p = c.p, o = affOthers(p);
      if (!o.length) return [{ nome: AFF_T("aff.menu.beside"), dim: true, esegui: () => 0 }];
      return o.map(q => { const d = { k: p.y < q.y || p.y === q.y && p.x < q.x ? "sx" : "dx", t: q.id }; return { nome: AFF_T("aff.menu.beside") + " " + affRowOf(q), dim: !affOkFor(p, d), esegui: () => { affFai(p, d, false); } }; });
    } });
  const via = (/** @type {string} */ id, /** @type {number} */ ordine, /** @type {string} */ key, /** @type {(p: any) => any} */ move) =>
    reg({ id, ordine, nome: () => AFF_T(key), disponibile: (/** @type {any} */ c) => affOkFor(c.p, move(c.p)), esegui: (/** @type {any} */ c) => { const d = move(c.p); if (d) affFai(c.p, d, affWide(d)); } });
  via("disp.su", 2, "aff.menu.up", p => affVertical(p, -1));
  via("disp.giu", 3, "aff.menu.down", p => affVertical(p, 1));
  via("disp.intera", 4, "aff.menu.full", p => { const m = affMate(p); return m ? { k: "giu", t: m.id } : null; });
  via("disp.uguali", 5, "aff.menu.equal", p => affMate(p) ? { k: "uguali" } : null);
  reg({ id: "disp.ripristina", ordine: 6, nome: () => AFF_T("aff.menu.reset"), disponibile: () => affSplit(), esegui: () => affRipristina() });
}

/** The spectrum opened from `from` goes to its right, in the same row, when `from` is alone in its row and the two fit; otherwise it stays below. */
function affAccanto(/** @type {any} */ s, /** @type {any} */ from) {
  if (!s.el || !from.el || !affEligible(s) || !affEligible(from)) return false;
  const row = LAYOUT.righeDi(affStack()).find(r => r.includes(from));
  if (!row || row.length !== 1) return false;
  const d = { k: "dx", t: from.id };
  return affOkFor(s, d) && affFai(s, d, false);
}

const AFFIANCA = { accanto: affAccanto, grip: affGrip, sync: affSync, annulla: affAnnulla, ultimo: affUltimo, fai: affFai, rifai: affRifai, ripristina: affRipristina, eligible: affEligible, hint: { target: affHintTarget, show: affHintShow, close: affHintClose, seen: affHintSeen } };
window.AFFIANCA = AFFIANCA;
