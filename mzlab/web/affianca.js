// @ts-check
// Side-by-side panels of the Dati view: the grip in the header (drag with zones, menu, Alt+arrows), the dividers between neighbours, undo (Ctrl/Cmd+Z).
// Classic script, loaded after pannelli.js (shared globals). The rows themselves (riga, quota, geometry) are in layout.js; here only the gestures.
const AFFIANCA = (() => {
  const T = (/** @type {string} */ k, /** @type {any} */ v) => I18N.t(k, v);
  const css = document.createElement("style");
  css.textContent = ".aff-grip{flex:none;min-width:24px;min-height:24px;padding:0 2px;margin-right:2px;border:0;background:transparent;color:var(--muted);cursor:grab;touch-action:none;opacity:0;line-height:1;font-size:14px}" +
    ".pnl:hover .aff-grip,.aff-grip:focus-visible,.aff-grip[aria-expanded=true]{opacity:1}.aff-grip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.aff-grip{opacity:1;min-width:40px;min-height:40px}}" +
    ".pnl.aff-drag{opacity:.45}.aff-prev{position:absolute;z-index:30;box-sizing:border-box;pointer-events:none;border:2px dashed var(--accent);background:var(--sel);opacity:.8;display:flex;align-items:center;justify-content:center;color:var(--ink);font-size:13px;font-weight:600}" +
    ".aff-prev.no{border-color:var(--muted);background:var(--soft);color:var(--muted)}" +
    ".aff-menu{position:fixed;z-index:10005;min-width:200px;max-width:min(320px,94vw);max-height:70vh;overflow:auto;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.2);padding:4px}" +
    ".aff-menu button{display:block;width:100%;min-height:28px;text-align:left;border:0;background:transparent;color:var(--ink);padding:3px 8px;border-radius:4px}.aff-menu button:hover,.aff-menu button:focus-visible{background:var(--sel);outline:2px solid var(--accent);outline-offset:-2px}.aff-menu button[aria-disabled=true]{color:var(--muted);cursor:default}" +
    ".aff-menu .aff-lbl{display:block;padding:4px 8px 2px;font-size:11px;color:var(--muted)}" +
    ".aff-sep{position:absolute;z-index:6;width:24px;cursor:col-resize;touch-action:none;display:flex;justify-content:center;align-items:stretch;border:0;padding:0;background:transparent}.aff-sep::after{content:'';width:4px;border-radius:2px;background:var(--line)}" +
    ".aff-sep:hover::after,.aff-sep:focus-visible::after,.aff-sep.on::after{background:var(--accent)}.aff-sep:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.aff-sep{width:40px}}" +
    "#aff-bar{position:fixed;z-index:100001;bottom:20px;left:50%;transform:translateX(-50%);display:flex;gap:10px;align-items:center;background:var(--ink);color:var(--panel);padding:6px 12px;border-radius:6px;box-shadow:0 3px 12px rgba(0,0,0,.25)}#aff-bar button{min-height:24px;padding:0 8px}" +
    "#aff-live{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}";
  document.head.appendChild(css);

  const eligible = (/** @type {any} */ p) => !!(p.full && p.el && !dockedP(p) && !hasDocked(p) && !p.el.classList.contains("max"));
  const keys = (/** @type {any[]} */ l) => l.map(q => ({ id: q.id, x: q.x, y: q.y, w: q.w, h: q.h, riga: q.riga, quota: q.quota, tab: q.tab, full: q.full }));
  const entra = (/** @type {any[]} */ row, /** @type {number} */ W) => row.length <= LAYOUT.MAX_ROW && row.every(q => minW(q) * row.length + pnlGap() * (row.length - 1) <= W);

  /**
   * The move `d` applied to an ordered stack (real panels or copies of them). `grp` = ids that travel together (vertical moves).
   * d.k: sx / dx (beside d.t), su / giu (own row before / after the row of d.t), fine (own row at the end), uguali (equal widths of the row of p).
   * @returns {any[] | null} the new order, or null when it does not fit (nothing is touched then)
   */
  function rifai(/** @type {any[]} */ order, /** @type {number} */ pid, /** @type {number[]} */ grp, /** @type {any} */ d, /** @type {number} */ W) {
    const p = order.find(q => q.id === pid), rest = order.filter(q => !grp.includes(q.id)), mates = (/** @type {any} */ q) => rest.filter(x => x.riga === q.riga);
    if (!p) return null;
    if (d.k === "uguali") { order.filter(q => q.riga === p.riga).forEach(q => { q.quota = null; }); return order; }
    const t = d.t != null ? rest.find(q => q.id === d.t) : null;
    if (d.k !== "fine" && (!t || t === p)) return null;
    const side = d.k === "sx" || d.k === "dx";
    if (side && !entra([...mates(t), p], W)) return null;
    const old = order.filter(q => q !== p && q.riga === p.riga && !grp.includes(q.id));
    if (old.length === 1) old[0].quota = 1;                                               // left alone: back to the full width
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

  /** The stack in reading order; a panel that has no row yet (just opened) gets its own. */
  const stack = () => { const o = stackOrder(); let n = o.reduce((m, q) => Math.max(m, q.riga ?? -1), -1) + 1; o.forEach(q => { if (q.riga == null) q.riga = n++; }); return o; };

  /** @type {{ n: number, s: any[] }[]} */ const hist = [];
  const snap = () => { hist.push({ n: ++HSEQ, s: keys(tabPanels().filter(q => q.full)) }); if (hist.length > 20) hist.shift(); };
  const relay = (/** @type {any} */ keep) => { placeRows(stack()); E.panels.sort(LAYOUT.ordine); if (window.DDA) DDA.sync(); fitHost(); if (keep && keep.el) front(keep.el); uiSave(); };

  const live = document.createElement("div"); live.id = "aff-live"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); document.body.appendChild(live);
  const say = (/** @type {string} */ m) => { live.textContent = ""; setTimeout(() => { live.textContent = m; }, 30); };
  const where = (/** @type {any} */ p) => { const rows = LAYOUT.righeDi(stack()), r = rows.find(x => x.includes(p)); return r ? T("aff.where", { n: p.num ?? "", r: rows.indexOf(r) + 1, t: rows.length, i: r.indexOf(p) + 1, m: r.length }) : ""; };

  /** @type {any} */ let barTimer = null;
  function bar(/** @type {string} */ msg) {
    const old = document.getElementById("aff-bar"); if (old) old.remove();
    clearTimeout(barTimer);
    const b = document.createElement("div"); b.id = "aff-bar"; b.setAttribute("role", "status");
    b.innerHTML = `<span>${EH(msg)}</span><button type="button">${EH(T("aff.undo"))}</button>`;
    b.querySelector("button").onclick = () => { b.remove(); annulla(); };
    document.body.appendChild(b); barTimer = setTimeout(() => b.remove(), 6000);
  }
  /** Do a move on the real panels; returns false (and says so) when the move does not fit. */
  function fai(/** @type {any} */ p, /** @type {any} */ d, /** @type {boolean} */ wide) {
    const before = keys(tabPanels().filter(q => q.full)), grp = wide ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id];
    const order = stack(), res = rifai(order, p.id, grp, d, hostWidth());
    if (!res) { say(T("aff.zone.nospace")); return false; }
    hist.push({ n: ++HSEQ, s: before }); if (hist.length > 20) hist.shift();
    placeRows(res); relay(p); numberPanels(); say(where(p)); bar(T("aff.done"));
    return true;
  }
  function annulla() {
    const h = hist.pop(); if (!h) return;
    h.s.forEach(s => { const q = E.panels.find(x => x.id === s.id); if (q) { q.x = s.x; q.y = s.y; q.w = s.w; q.h = s.h; q.riga = s.riga; q.quota = s.quota; } });
    relay(null); numberPanels(); say(T("aff.undone"));
  }
  const ultimo = () => hist.length ? hist[hist.length - 1].n : 0;

  // ---- destination under the pointer (host coordinates) -> move
  function zona(/** @type {any[]} */ order, /** @type {number[]} */ grp, /** @type {number} */ px, /** @type {number} */ py) {
    const rest = order.filter(q => !grp.includes(q.id)), t = rest.find(q => px >= q.x && px < q.x + q.w && py >= q.y && py < q.y + q.h);
    if (t) { const fx = (px - t.x) / t.w; return fx < 0.25 ? { k: "sx", t: t.id } : fx > 0.75 ? { k: "dx", t: t.id } : { k: (py - t.y) / t.h < 0.5 ? "su" : "giu", t: t.id }; }
    const next = rest.find(q => q.y > py);
    return next ? { k: "su", t: next.id } : { k: "fine" };
  }
  /** @type {HTMLElement | null} */ let prev = null;
  const showPrev = (/** @type {{x:number,y:number,w:number,h:number} | null} */ r, /** @type {string} */ label) => {
    if (!r) { if (prev) { prev.remove(); prev = null; } return; }
    if (!prev) { prev = document.createElement("div"); prev.setAttribute("aria-hidden", "true"); Q("#dpanels").appendChild(prev); }
    prev.className = "aff-prev" + (label === T("aff.zone.nospace") ? " no" : "");
    Object.assign(prev.style, { left: r.x + "px", top: r.y + "px", width: r.w + "px", height: r.h + "px" }); prev.textContent = label;
  };
  const label = (/** @type {any} */ d) => T(d.k === "sx" ? "aff.zone.left" : d.k === "dx" ? "aff.zone.right" : "aff.zone.here");
  function dragGrip(/** @type {any} */ p, /** @type {PointerEvent} */ e0, /** @type {any} */ g) {
    const coarse = e0.pointerType !== "mouse", sx = e0.clientX, sy = e0.clientY;
    let on = false, key = "", d = /** @type {any} */ (null);
    const grpOf = (/** @type {any} */ d0) => (d0 && (d0.k === "sx" || d0.k === "dx") ? [p.id] : blockOf(p).grp.map((/** @type {any} */ q) => q.id));
    const upd = (/** @type {PointerEvent} */ ev) => {
      const h = Q("#dpanels").getBoundingClientRect(), px = ev.clientX - h.left, py = ev.clientY - h.top, order = stack();
      let z = zona(order, [p.id], px, py); if (z.k !== "sx" && z.k !== "dx") z = zona(order, grpOf(z), px, py);
      const kk = z.k + ":" + z.t; if (kk === key) return; key = kk; d = z;
      const cp = keys(order), grp = grpOf(z), res = rifai(cp, p.id, grp, z, hostWidth());
      if (!res) { const t = order.find(q => q.id === z.t); showPrev(t ? { x: t.x, y: t.y, w: t.w, h: t.h } : null, T("aff.zone.nospace")); d = null; return; }
      const gm = LAYOUT.colloca(res, hostWidth(), pnlGap(), minW), r = gm.pos.get(res.find(q => q.id === p.id));
      showPrev(r || null, label(z));
    };
    const stop = (/** @type {boolean} */ commit) => {
      g.removeEventListener("pointermove", mv); g.removeEventListener("pointerup", up); g.removeEventListener("pointercancel", cancel); document.removeEventListener("keydown", esc, true);
      try { g.releasePointerCapture(e0.pointerId); } catch (_) { /* already released */ }
      p.el.classList.remove("aff-drag"); showPrev(null, "");
      if (on) { g._drag = true; setTimeout(() => { g._drag = false; }, 0); }
      if (commit && on && d) fai(p, d, d.k !== "sx" && d.k !== "dx");
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

  // ---- menu of the grip (the alternative to dragging) and the keyboard
    function vertical(/** @type {any} */ p, /** @type {number} */ dir) {         // one row up / down
    const grp = blockOf(p).grp, order = stack().filter(q => !grp.includes(q)), rows = LAYOUT.righeDi(order), all = stack();
    const slot = rows.filter(r => all.indexOf(r[0]) < all.indexOf(p)).length, mates = all.filter(q => q !== p && q.riga === p.riga);
    if (mates.length) return { k: dir < 0 ? "su" : "giu", t: mates[0].id };            // leaving its row: right above / below it
    const r = rows[dir < 0 ? slot - 1 : slot]; return r ? { k: dir < 0 ? "su" : "giu", t: r[0].id } : null;
  }
  const neighbour = (/** @type {any} */ p, /** @type {number} */ dir) => { const o = stack(), i = o.indexOf(p) + dir, t = o[i]; return t && !blockOf(p).grp.includes(t) ? { k: dir < 0 ? "sx" : "dx", t: t.id } : null; };
  /** @type {any} */ let menu = null;
  function closeMenu(/** @type {boolean} */ back) { if (!menu) return; const g = menu._grip; menu.remove(); menu = null; g.setAttribute("aria-expanded", "false"); if (back) g.focus(); }
  function openMenu(/** @type {any} */ p, /** @type {any} */ g) {
    if (menu) { closeMenu(false); return; }
    const W = hostWidth(), order = stack(), item = (/** @type {string} */ txt, /** @type {any} */ d, /** @type {boolean} */ wide) => {
      const b = document.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitem"); b.textContent = txt;
      const ok = !!d && !!rifai(keys(order), p.id, wide ? blockOf(p).grp.map((/** @type {any} */ q) => q.id) : [p.id], d, W);
      if (!ok) b.setAttribute("aria-disabled", "true"); else b.onclick = () => { closeMenu(false); fai(p, d, wide); g.focus(); };
      return b;
    };
    menu = document.createElement("div"); menu.className = "aff-menu"; menu.setAttribute("role", "menu"); menu.setAttribute("aria-label", T("aff.grip")); menu._grip = g;
    const others = order.filter(q => q !== p && !blockOf(p).grp.includes(q) && eligible(q));
    if (others.length) {
      const l = document.createElement("span"); l.className = "aff-lbl"; l.textContent = T("aff.menu.beside"); menu.appendChild(l);
      others.forEach(q => menu.appendChild(item(`${q.num ?? ""} · ${q.el.querySelector(".ttl").textContent}`, { k: p.y < q.y || p.y === q.y && p.x < q.x ? "sx" : "dx", t: q.id }, false)));
    }
    const mate = order.find(q => q !== p && q.riga === p.riga);
    menu.appendChild(item(T("aff.menu.up"), vertical(p, -1), true)); menu.appendChild(item(T("aff.menu.down"), vertical(p, 1), true));
    menu.appendChild(item(T("aff.menu.full"), mate ? { k: "giu", t: mate.id } : null, true));
    const eq = document.createElement("button"); eq.type = "button"; eq.setAttribute("role", "menuitem"); eq.textContent = T("aff.menu.equal");
    if (!mate) eq.setAttribute("aria-disabled", "true"); else eq.onclick = () => { closeMenu(false); fai(p, { k: "uguali" }, false); g.focus(); };
    menu.appendChild(eq); document.body.appendChild(menu); g.setAttribute("aria-expanded", "true");
    const r = g.getBoundingClientRect(); menu.style.left = Math.max(4, Math.min(r.left, innerWidth - menu.offsetWidth - 4)) + "px"; menu.style.top = Math.min(r.bottom + 2, innerHeight - menu.offsetHeight - 4) + "px";
    const btns = () => /** @type {HTMLElement[]} */ ([...menu.querySelectorAll("button")]);
    menu.addEventListener("keydown", e => {
      const b = btns(), i = b.indexOf(/** @type {HTMLElement} */ (document.activeElement));
      if (e.key === "Escape") { e.preventDefault(); closeMenu(true); }
      else if (e.key === "ArrowDown") { e.preventDefault(); b[(i + 1) % b.length].focus(); }
      else if (e.key === "ArrowUp") { e.preventDefault(); b[(i - 1 + b.length) % b.length].focus(); }
      else if (e.key === "Tab") closeMenu(false);
    });
    const away = (/** @type {Event} */ e) => { if (menu && !menu.contains(/** @type {Node} */ (e.target)) && !g.contains(/** @type {Node} */ (e.target))) { closeMenu(false); document.removeEventListener("pointerdown", away, true); } };
    setTimeout(() => document.addEventListener("pointerdown", away, true), 0);
    (btns().find(b => b.getAttribute("aria-disabled") !== "true") || btns()[0]).focus();
  }
  function grip(/** @type {any} */ p) {
    /** @type {any} */ const g = document.createElement("button"); g.type = "button"; g.className = "aff-grip"; g.textContent = "⠿";
    g.title = T("aff.grip"); g.setAttribute("aria-label", T("aff.grip")); g.setAttribute("aria-haspopup", "menu"); g.setAttribute("aria-expanded", "false");
    g.addEventListener("pointerdown", e => { if (e.button === 0 && eligible(p)) { setActive(p); front(p.el); dragGrip(p, e, g); } });
    g.addEventListener("click", () => { if (!g._drag) openMenu(p, g); });
    g.addEventListener("keydown", e => {
      if (!e.altKey || !/^Arrow/.test(e.key)) return;
      e.preventDefault();
      const d = e.key === "ArrowUp" ? vertical(p, -1) : e.key === "ArrowDown" ? vertical(p, 1) : neighbour(p, e.key === "ArrowLeft" ? -1 : 1);
      if (d) fai(p, d, d.k !== "sx" && d.k !== "dx"); else say(T("aff.zone.nospace"));
      g.focus();
    });
    p.el.querySelector(".hd").prepend(g); p._grip = g;
  }

  // ---- dividers between side-by-side neighbours
  let busy = false;
  const MIN_KEY = 16, BIG_KEY = 48;
  function resize(/** @type {any[]} */ row, /** @type {number} */ i, /** @type {number} */ dx) {
    const a = row[i], b = row[i + 1], tot = a.w + b.w, w0 = Math.max(minW(a), Math.min(tot - minW(b), a.w + dx));
    a.w = w0; b.x = a.x + a.w + pnlGap(); b.w = tot - w0; apply(a); apply(b);
  }
  const commitRow = (/** @type {any[]} */ row) => { const s = row.reduce((m, q) => m + q.w, 0); row.forEach(q => { q.quota = q.w / s; }); relay(null); };
  function sep(/** @type {any[]} */ row, /** @type {number} */ i) {
    const a = row[i], b = row[i + 1], /** @type {any} */ el = document.createElement("button");
    el.type = "button"; el.className = "aff-sep"; el.setAttribute("role", "separator"); el.setAttribute("aria-orientation", "vertical"); el.setAttribute("aria-label", T("aff.sep"));
    const pos = () => { const h = Math.min(a.h, 64); el.style.left = a.x + a.w + pnlGap() / 2 - el.offsetWidth / 2 + "px"; el.style.top = a.y + (a.h - h) / 2 + "px"; el.style.height = h + "px"; el.setAttribute("aria-valuenow", String(Math.round(100 * a.w / (a.w + b.w)))); };
    Q("#dpanels").appendChild(el); pos();
    el.addEventListener("pointerdown", e => {
      if (e.button !== 0) return;
      e.preventDefault(); el.setPointerCapture(e.pointerId); el.classList.add("on"); busy = true; snap();
      const x0 = e.clientX, w0 = a.w;
      const mv = (/** @type {PointerEvent} */ ev) => { resize(row, i, w0 + ev.clientX - x0 - a.w); pos(); };
      const up = () => { el.removeEventListener("pointermove", mv); el.removeEventListener("pointerup", up); el.removeEventListener("pointercancel", up); el.classList.remove("on"); busy = false; commitRow(row); };
      el.addEventListener("pointermove", mv); el.addEventListener("pointerup", up); el.addEventListener("pointercancel", up);
    });
    el.addEventListener("keydown", e => {
      const step = e.shiftKey ? BIG_KEY : MIN_KEY, tot = a.w + b.w;
      let dx = null;
      if (e.key === "ArrowLeft") dx = -step; else if (e.key === "ArrowRight") dx = step;
      else if (e.key === "Home") dx = minW(a) - a.w; else if (e.key === "End") dx = tot - minW(b) - a.w;
      else if (e.key === "Enter") dx = (tot - pnlGap()) / 2 - a.w;
      if (dx == null) return;
      e.preventDefault(); snap(); resize(row, i, dx); commitRow(row);
    });
    el.addEventListener("dblclick", () => { snap(); row.forEach(q => { q.quota = null; }); relay(null); });
    el._key = a.id + ":" + b.id; return el;
  }
  /** After every layout: the grips of the panels that can move, the dividers of the rows. */
  function sync() {
    if (busy) return;
    const host = document.getElementById("dpanels"); if (!host) return;
    const was = document.activeElement && document.activeElement.classList.contains("aff-sep") ? /** @type {any} */ (document.activeElement)._key : null;       // a rebuilt divider keeps the focus
    host.querySelectorAll(".aff-sep").forEach(e => e.remove());
    tabPanels().forEach(q => { if (q._grip) q._grip.hidden = !eligible(q); });
    if (fsPanel()) return;
    LAYOUT.righeDi(stack()).forEach(r => { if (r.length > 1 && r.every((/** @type {any} */ q) => q.y === r[0].y)) for (let i = 0; i < r.length - 1; i++) { const e = sep(r, i); if (was === e._key) e.focus(); } });
  }
  return { grip, sync, annulla, ultimo, fai, rifai };
})();
window.AFFIANCA = AFFIANCA;
