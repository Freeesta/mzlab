// @ts-check
// Mouse and keyboard on the charts, acquisition method, toolbar, formula calculator, tooltips. Split from explore.js: classic script, loaded after picchi.js.
// ------------------------------------------------------------------ mouse
function attach(p) {
  const cv = p.cv;
  let drag = null;
  // pixel -> axis value, always inside the real range of the data (no negative RT, nothing beyond the last scan): selection, zoom, integration, cursor
  const xd = px => { const a = p._a; if (!a) return null; const x = a.x0 + (px - M.l) / (a.W - M.l - M.r) * (a.x1 - a.x0); return a.full ? Math.max(a.full[0], Math.min(a.full[1], x)) : x; };
  const rect = e => { const r = cv.getBoundingClientRect(); return e.clientX - r.left; };
  const recty = e => e.clientY - cv.getBoundingClientRect().top;
  const canYZ = a => a && !a.map && !a.stk && !a.logy && !a.cas && a.yinv;                      // y zoom exists for chromatograms, XIC, MRM and spectra (not stacked / log views)
  const inPlotY = py => py >= M.t && py <= p._a.H - M.b;
  const yAxisZone = (px, py) => canYZ(p._a) && px < M.l && inPlotY(py);               // the numbers at the left of the y axis
  const xAxisZone = (px, py) => p._a && !p._a.map && px >= M.l && px <= p._a.W - M.r && py > p._a.H - M.b;   // the numbers under the x axis
  const onCur = px => p.type !== "spec" && p.type !== "map" && p.cur != null && p._a && Math.abs(p._a.X(p.cur) - px) < 6;
  const edgeAt = px => {
    if (!p._a || p.type === "spec") return null;
    for (const it of p.ints) for (const e of ["a", "b"]) if (Math.abs(p._a.X(it[e]) - px) < 6) {
      let mate = null; for (const o of p.ints) if (o !== it && o.key === it.key) for (const f of ["a", "b"]) if (Math.abs(o[f] - it[e]) < INT_EPS) mate = { it: o, e: f };      // a border shared with the next peak moves both
      return { it, e, mate, snap: intSnap(p) };
    }
    return null;
  };
  const edgeMove = (ed, x) => {                                              // the border stops at the neighbouring peak (touching is fine, overlapping is not)
    const { it, e, mate } = ed, o = e === "a" ? it.b : it.a, v = it[e], oth = p.ints.filter(i => i !== it && (!mate || i !== mate.it) && i.key === it.key);
    const mo = mate ? (mate.e === "a" ? mate.it.b : mate.it.a) : null;     // far border of the peak that shares this one
    let nx;
    if (v >= o) { let u = mo != null ? Math.max(mo, v) : Infinity; oth.forEach(i => { if (iLo(i) >= v - INT_EPS) u = Math.min(u, iLo(i)); }); nx = Math.max(o, Math.min(u, x)); }
    else { let l = mo != null ? Math.min(mo, v) : -Infinity; oth.forEach(i => { if (iHi(i) <= v + INT_EPS) l = Math.max(l, iHi(i)); }); nx = Math.min(o, Math.max(l, x)); }
    it[e] = nx; if (mate) mate.it[mate.e] = nx;
  };
  // the vertical line (or its dot) goes to x: the linked spectrum follows once per frame
  const moveCur = x => {
    const a = p._a; if (!a || x == null) return;
    p.cur = Math.max(a.full[0], Math.min(a.full[1], x)); p.sel = null; draw(p);
    if (!p._lm) { p._lm = true; requestAnimationFrame(() => { p._lm = false; const sn = p.tab === "ms2" && p.type === "chrom" ? ms2Near(p, p.cur, 0) : null, dt = scanStep(); if (sn) pushLinked(p, sn.rt - sn.g / 2, sn.rt + sn.g / 2, sn.k, true); else pushLinked(p, p.cur - dt / 2, p.cur + dt / 2, nearestFile(p, p.cur), true); }); }
  };
  // the dot on top of the line: dragged with mouse, finger or pen; the arrows move it one scan (Shift: five)
  if (p.ch) {
    const h = p.ch, at = e => e.clientX - cv.getBoundingClientRect().left;
    h.addEventListener("pointerdown", e => { e.preventDefault(); e.stopPropagation(); try { h.setPointerCapture(e.pointerId); } catch (_) { /* the events still arrive over the dot */ } setActive(p); drag = { cursor: true }; h.classList.add("on"); });
    h.addEventListener("pointermove", e => { if (drag && drag.cursor && h.classList.contains("on")) moveCur(xd(at(e))); });
    const end = e => { if (!h.classList.contains("on")) return; h.classList.remove("on"); if (e.type === "pointerup") moveCur(xd(at(e))); p._up(e); };
    h.addEventListener("pointerup", end); h.addEventListener("pointercancel", end);
    h.addEventListener("keydown", e => {
      const d = e.key === "ArrowLeft" || e.key === "ArrowDown" ? -1 : e.key === "ArrowRight" || e.key === "ArrowUp" ? 1 : 0;
      if (!d || e.ctrlKey || e.metaKey || e.altKey) return;
      e.preventDefault(); e.stopPropagation(); setActive(p); if (p._a && p._a.sr) stepScan(p, d * (e.shiftKey ? 5 : 1));
    });
  }
  cv.onmousemove = e => {
    const a = p._a; if (!a) return; const px = rect(e), py = recty(e), x = xd(px);
    if (p.type === "spec" && a.markN != null) hoverMark(p, px);
    if (drag && drag.pan) {                                   // Maiusc + trascina: sposta la vista
      const pw = a.W - M.l - M.r, span = drag.z[1] - drag.z[0], dx = (px - drag.x0) / pw * span;
      p.zoom = clampView(drag.z[0] - dx, drag.z[1] - dx, a.full[0], a.full[1]);
      if (a.map && drag.zy) { const sy = drag.zy[1] - drag.zy[0], dy = (py - drag.y0) / (a.H - M.t - M.b) * sy; p.zoomY = clampView(drag.zy[0] + dy, drag.zy[1] + dy, a.fullY[0], a.fullY[1]); }
      draw(p);
    } else if (drag && drag.cursor) moveCur(x);                // dragging the vertical line: the linked spectrum follows
    else if (drag && drag.edge) { edgeMove(drag.edge, x); draw(p); }
    else if (drag && drag.ya) {                               // line along the y axis: only the intensity axis is zoomed
      drag.y = Math.max(M.t, Math.min(a.H - M.b, py)); zl.hidden = false; zl.className = "zl v";
      zl.style.left = cv.offsetLeft + M.l - 20 + "px"; zl.style.width = ""; zl.style.top = cv.offsetTop + Math.min(drag.y, drag.y0) + "px"; zl.style.height = Math.abs(drag.y - drag.y0) + "px";
      zl.dataset.v = `${fmt(a.yinv(Math.max(drag.y, drag.y0)))} – ${fmt(a.yinv(Math.min(drag.y, drag.y0)))}`;
    } else if (drag && drag.xa) {                             // line along the x axis: only the x axis is zoomed
      drag.x = Math.max(M.l, Math.min(a.W - M.r, px)); zl.hidden = false; zl.className = "zl h";
      zl.style.top = cv.offsetTop + a.H - M.b + 6 + "px"; zl.style.height = ""; zl.style.left = cv.offsetLeft + Math.min(drag.x, drag.x0) + "px"; zl.style.width = Math.abs(drag.x - drag.x0) + "px";
      const u = xd(drag.x0), v = xd(drag.x), lo = Math.min(u, v), hi = Math.max(u, v);
      zl.dataset.v = p.type === "spec" ? I18N.t("zoom.mz", { lo: lo.toFixed(mzd(p, p.level)), hi: hi.toFixed(mzd(p, p.level)) }) : I18N.t("zoom.rt", { lo: lo.toFixed(2), hi: hi.toFixed(2) });
    }
    else if (drag) { drag.x = px; drag.y = py; const mapBox = p.type === "map" && !drag.alt; if (p.type === "spec" || mapBox || (p.imode === "zoom" && p.type !== "map")) {      // zoom box (x and y together; on the map Alt + drag is the spectrum of the interval)
      if (Math.abs(px - drag.x0) > 4 || (mapBox && Math.abs(py - drag.y0) > 4)) { zr.hidden = false; const yy = canYZ(a) || mapBox ? [Math.min(py, drag.y0), Math.abs(py - drag.y0)] : [M.t, a.H - M.t - M.b]; zr.style.left = cv.offsetLeft + Math.min(px, drag.x0) + "px"; zr.style.width = Math.abs(px - drag.x0) + "px"; zr.style.top = cv.offsetTop + yy[0] + "px"; zr.style.height = yy[1] + "px"; }
    } else { p.sel = [Math.min(xd(drag.x0), x), Math.max(xd(drag.x0), x)]; draw(p); } }
    else axisHint(yAxisZone(px, py) ? "v" : xAxisZone(px, py) ? "h" : null, e);
    if (!drag) cv.style.cursor = e.shiftKey ? "grab" : yAxisZone(px, py) ? "ns-resize" : xAxisZone(px, py) ? "ew-resize" : edgeAt(px) ? "col-resize" : onCur(px) ? "ew-resize" : p.imode === "zoom" ? "zoom-in" : p.imode === "idel" ? "pointer" : p.imode ? "cell" : "crosshair";
    const lh = !drag && (a.lbls || []).find(b => px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h);
    if (lh) { cv.style.cursor = "pointer"; lb.hidden = false; lb.style.left = cv.offsetLeft + lh.x + "px"; lb.style.top = cv.offsetTop + lh.y + "px"; lb.style.width = lh.w + "px"; lb.style.height = lh.h + "px"; } else lb.hidden = true;
    p.rd.textContent = p.type === "spec" ? "m/z " + x.toFixed(a.hrp ? a.dec : 1) : "RT " + x.toFixed(2) + " min" + (a.map ? " · m/z " + a.mzAt(py).toFixed(mzd(a, a.level)) : "");   // pass-over text: it goes away when the mouse leaves (see onmouseleave)
    showHover(p, px, py); if (drag) p.tip.hidden = true;
    if (p.type === "map" && window.MAPPA) MAPPA.move(p, px, py);
    if (lh) { p.tip.hidden = false; p.tip.innerHTML = lh.tip; p.vl.hidden = true; const tw = p.tip.offsetWidth; let l = cv.offsetLeft + px + 14; if (l + tw > p.el.clientWidth - 4) l = cv.offsetLeft + px - tw - 14; p.tip.style.left = Math.max(2, l) + "px"; p.tip.style.top = cv.offsetTop + py + 14 + "px"; }
  };
  cv.addEventListener("click", e => {                                     // click on the diamond of a contaminant sign: exclude / include
    const mk = (p._a && p._a.cmarks || []).find(c => Math.abs(rect(e) - c.x) <= 9 && Math.abs(recty(e) - c.y) <= 9);
    if (mk) { e.stopPropagation(); contMenu(p, e, mk); }
  });
  const lb = document.createElement("div"); lb.className = "lbbox"; lb.hidden = true; p.el.appendChild(lb);   // box around the label under the mouse
  const zr = document.createElement("div"); zr.className = "zr"; zr.hidden = true; p.el.appendChild(zr);   // area being zoomed (box)
  const zl = document.createElement("div"); zl.className = "zl"; zl.hidden = true; p.el.appendChild(zl);
  // over the numbers of an axis: a faint strip and the cursor say that a drag there zooms only that axis (the program's own label appears after TIP_DELAY)
  let hintT = null;
  const axisHint = (z, e) => {
    if (z === p._hz) return; p._hz = z; clearTimeout(hintT); const q = Q("#qtip"); if (q && q._ax) { q.hidden = true; q._ax = false; }
    const a = p._a; if (!z || !a) { zl.hidden = true; return; }
    zl.hidden = false; zl.className = "zl hint " + z; zl.dataset.v = "";
    if (z === "v") { zl.style.left = cv.offsetLeft + "px"; zl.style.width = M.l - 2 + "px"; zl.style.top = cv.offsetTop + M.t + "px"; zl.style.height = a.H - M.t - M.b + "px"; }
    else { zl.style.top = cv.offsetTop + a.H - M.b + 1 + "px"; zl.style.height = M.b - 2 + "px"; zl.style.left = cv.offsetLeft + M.l + "px"; zl.style.width = a.W - M.l - M.r + "px"; }
    const cx = e.clientX, cy = e.clientY;
    hintT = setTimeout(() => { if (!q) return; q.textContent = I18N.t("axis.zoomHint"); q.hidden = false; q._ax = true; q.style.left = Math.max(6, Math.min(innerWidth - q.offsetWidth - 6, cx + 12)) + "px"; q.style.top = Math.min(innerHeight - q.offsetHeight - 6, cy + 16) + "px"; }, TIP_DELAY);
  };   // line being dragged on an axis (zooms that axis only)
  cv.onmouseleave = () => { if (p._mk != null) { p._mk = null; drawSoon(p); } axisHint(null, {}); hideHover(p); lb.hidden = true; p.rd.textContent = p._msg && p.cur != null ? p._msg : ""; if (p.type === "map" && window.MAPPA) MAPPA.leave(p); };   // no leftover "m/z 289.2": only the useful scan message stays while the cursor is there
  cv.onmousedown = e => {
    if (e.button !== 0 || !p._a) return; const px = rect(e), py = recty(e), a = p._a;
    if (e.shiftKey) { e.preventDefault(); drag = { pan: true, x0: px, y0: py, z: [a.x0, a.x1], zy: a.map ? [a.y0, a.y1] : null }; return; }
    if (p.type === "map" && window.MAPPA && MAPPA.down(p, e, px, py)) return;          // mappa.js: dragging a marked point
    if (yAxisZone(px, py)) { drag = { ya: true, y0: Math.max(M.t, Math.min(a.H - M.b, py)), y: py }; e.preventDefault(); return; }
    if (xAxisZone(px, py)) { drag = { xa: true, x0: px, x: px }; e.preventDefault(); return; }
    const ed = edgeAt(px); drag = ed ? { edge: ed } : onCur(px) && !p.imode ? { cursor: true } : { x0: px, x: px, y0: py, y: py, alt: e.altKey };
  };
  p._up = e => {
    if (!drag) return; const d = drag; drag = null; zr.hidden = true;
    if (p.type === "map" && window.MAPPA && MAPPA.touched(p)) { zl.hidden = true; return; }       // mappa.js: the mouse events after a finger gesture
    zl.hidden = true; p._hz = null;
    if (d.pan) { uiSave(); return; }
    if (d.edge) { if (intSnap(p) !== d.edge.snap) pushIh(p, d.edge.snap); uiSave(); return; }
    if (d.cursor) { uiSave(); return; }
    const a = p._a;
    if (a && (d.ya || d.xa) && Math.abs((d.ya ? d.y - d.y0 : d.x - d.x0)) > 4) pushZh(p);
    if (d.ya) { if (a && Math.abs(d.y - d.y0) > 4) zoomYTo(p, a.yinv(Math.max(d.y, d.y0)), a.yinv(Math.min(d.y, d.y0))); return; }
    if (d.xa) { if (a && Math.abs(d.x - d.x0) > 4) { const u = xd(d.x0), v = xd(d.x); zoomTo(p, Math.min(u, v), Math.max(u, v)); } return; }
    const x0 = xd(d.x0), x1 = xd(d.x);
    if (!a || x0 == null) return;
    if (p.type === "map" && !d.alt && window.MAPPA && (Math.abs(d.x - d.x0) > 4 || Math.abs(d.y - d.y0) > 4)) { MAPPA.boxZoom(p, d.x0, d.y0, d.x, d.y); return; }   // mappa.js: box = zoom
    if (p.type === "map" && window.MAPPA && MAPPA.click(p, d.x0, d.y0)) return;       // mappa.js: «Segna» adds a point; a click on a point chooses it
    if (p._pickNoise && p.type !== "spec" && p.type !== "map" && Math.abs(d.x - d.x0) > 4) {          // the noise stretch for S/N of an integration
      const it = p._pickNoise; p._pickNoise = null; p.sel = null; it.noise = [Math.min(x0, x1), Math.max(x0, x1)]; draw(p); uiSave(); showInts(); return;
    }
    if (p._pickIbk && p.type !== "spec" && p.type !== "map" && Math.abs(d.x - d.x0) > 4) {            // "Sottrai il fondo di un tratto": the stretch the student just dragged
      p._pickIbk = false; p.sel = null; p.bk = ""; p.snip = false; ibkSet(p, "mean", [Math.min(x0, x1), Math.max(x0, x1)]); ctl(p); return;
    }
    if (p.type !== "spec" && p.type !== "map" && Math.abs(d.x - d.x0) <= 4 && p.imode !== "auto") {      // a click on a filled peak selects it; elsewhere it deselects
      if (p.imode === "idel") {
        const hit = intAt(p, d.x0, d.y0) || p.ints.find(it => x0 >= iLo(it) && x0 <= iHi(it));
        if (hit) delInts(p, [hit]);
        return;
      }
      const hit = intAt(p, d.x0, d.y0);
      if (hit) { p.isel = hit.id; draw(p); return; }
      if (p.isel != null) { p.isel = null; draw(p); }
    }
    if (p.imode && p.type !== "spec" && p.type !== "map") {          // zoom and integration tools
      if (p.imode === "zoom") {                                        // box: x and y together
        p.sel = null; if (Math.abs(d.x - d.x0) > 4) pushZh(p);
        if (Math.abs(d.x - d.x0) > 4) { if (canYZ(a) && Math.abs(d.y - d.y0) > 4) zoomYTo(p, a.yinv(Math.max(d.y, d.y0)), a.yinv(Math.min(d.y, d.y0)), true); zoomTo(p, Math.min(x0, x1), Math.max(x0, x1)); } else draw(p);
        return;
      }
      if (p.imode === "auto" && Math.abs(d.x - d.x0) <= 4) { if (guardInt(p)) intTargets(p, x0, d.y0).forEach(s => autoInt(p, s, x0)); return; }
      if (p.imode === "man" && Math.abs(d.x - d.x0) > 4) { p.sel = null; if (!guardInt(p)) { draw(p); return; } const t = intTargets(p, (x0 + x1) / 2, d.y0); if (t.length) t.forEach(s => addInt(p, s, x0, x1)); else draw(p); return; }
    }
    if (Math.abs(d.x - d.x0) > 4) {
      if (p.type === "spec") { pushZh(p); if (Math.abs(d.y - d.y0) > 4) zoomYTo(p, a.yinv(Math.max(d.y, d.y0)), a.yinv(Math.min(d.y, d.y0)), true); p.zoom = [Math.min(x0, x1), Math.max(x0, x1)]; draw(p); }
      else { p.sel = [Math.min(x0, x1), Math.max(x0, x1)]; draw(p); pushLinked(p, p.sel[0], p.sel[1], nearestFile(p, (x0 + x1) / 2)); }
    } else if (p.type === "spec") { if (window.DDA && DDA.click(p, d.x0, d.y0)) return; specClick(p, d.x0, d.y0); }
    else if (p.type !== "spec") {
      const tt = p.ms2tri && p._a.tri && d.y0 < M.t + 14 ? p._a.tri.reduce((b, q) => (Math.abs(q.px - d.x0) < 6 && (!b || Math.abs(q.px - d.x0) < Math.abs(b.px - d.x0)) ? q : b), null) : null;
      if (tt) { if (window.DDA && DDA.onTri(p, tt)) return; goMs2(tt); return; }          // a triangle: that MS2 scan (in the DDA trio, or in the MS2 tab)
      p.sel = null; p.cur = x0; const sn = p.tab === "ms2" && p.type === "chrom" && p._a.sr ? ms2Near(p, x0, 0) : null; if (sn) { p.cur = sn.rt; stepMsg(p, I18N.t("ms2.nearestScan", { rt: sn.rt.toFixed(2) }), true, true); } draw(p); const dt = scanStep(); if (sn) pushLinked(p, sn.rt - sn.g / 2, sn.rt + sn.g / 2, sn.k); else { const k0 = nearestFile(p, x0); pushLinked(p, x0 - dt / 2, x0 + dt / 2, k0); nearScan(k0, x0).then(n => { if (n && p.cur === x0) { p.cur = n.rt; draw(p); pushLinked(p, n.rt - NEAR, n.rt + NEAR, k0); } }); }
    }
  };
  addEventListener("mouseup", p._up, { signal: p._ac.signal });
  // Ctrl/Cmd + rotella (o pizzico sul trackpad): zoom attorno al cursore. La rotella da sola continua a far scorrere la pagina.
  cv.addEventListener("wheel", e => {
    if (p.type === "map" && window.MAPPA && p._a && MAPPA.wheel(p, e, rect(e), recty(e))) return;     // mappa.js: the wheel alone zooms the map
    if (!(e.ctrlKey || e.metaKey) && p._a && p.zoom && p.type !== "map" && Math.abs(e.deltaX) > Math.abs(e.deltaY)) {      // trackpad: two fingers sideways pan the zoomed x axis (vertical scroll still moves the page)
      e.preventDefault(); const a = p._a, span = a.x1 - a.x0, dx = e.deltaX / (a.W - M.l - M.r) * span;
      p.zoom = clampView(a.x0 + dx, a.x1 + dx, a.full[0], a.full[1]); drawSoon(p); return;
    }
    if (!(e.ctrlKey || e.metaKey) || !p._a) return;
    e.preventDefault();
    const a = p._a, f = Math.exp(Math.max(-60, Math.min(60, e.deltaY)) * 0.004), x = xd(rect(e));
    if (!p._wz || Date.now() - p._wz > 600) pushZh(p); p._wz = Date.now();                // a burst of wheel steps is one zoom for the history
    if (rect(e) < M.l && canYZ(a)) { const v = a.yinv(recty(e)); zoomYTo(p, v - (v - a.ymin) * f, v + (a.ymax - v) * f); uiSave(); return; }      // over the y axis: only the vertical scale changes
    if (a.map) { const y = a.mzAt(recty(e)); p.zoomY = clampView(y - (y - a.y0) * f, y + (a.y1 - y) * f, a.fullY[0], a.fullY[1]); }
    zoomTo(p, x - (x - a.x0) * f, x + (a.x1 - x) * f);
  }, { passive: false });
  cv.ondblclick = async e => {
    const hit = (p._a?.lbls || []).find(b => b.ann && rect(e) >= b.x && rect(e) <= b.x + b.w && recty(e) >= b.y && recty(e) <= b.y + b.h);
    if (hit) {
      const v = await ask("Annotazione", hit.ann.text);
      if (v != null) {
        if (v === "") p.anns = p.anns.filter(a => a !== hit.ann);
        else hit.ann.text = v;
        draw(p);
      }
      return;
    }
    if (p.type === "map") {                                              // double click = whole view (the spectrum at that RT is in the right-click menu)
      if (window.MAPPA && p._a && p._a.map && Date.now() - (MAPPA.st(p).dtap || 0) < 600) return;      // the double tap has already done it
      if (window.MAPPA) MAPPA.whole(p); else { p.zoom = null; p.zoomY = null; p.sel = null; draw(p); uiSave(); }
      return;
    }
    if (p.type === "mrm") { p.zoom = null; p.zoomY = null; p.sel = null; draw(p); uiSave(); return; }      // MRM has no spectra (a new one would jump to the Full Scan tab): double click = whole view
    if (p.type === "chrom" || p.type === "xic") {      // double click = mass spectrum at that retention time (also on the PDA trace)
      if (p.el.classList.contains("max")) { p.zoom = null; p.zoomY = null; p.sel = null; draw(p); return; }      // full screen: double click = whole view (a new spectrum would appear in a panel nobody sees)
      const x = xd(rect(e)), k = nearestFile(p, x), f = E.files[k];
      if (!p._a || !f || f.kind === "mrm") return;
      let dt = scanStep(), r0 = x - dt / 2, r1 = x + dt / 2, kk = k;
      const sn = p.tab === "ms2" && p.type === "chrom" ? ms2Near(p, x, 0) : null; if (sn) { r0 = sn.rt - sn.g / 2; r1 = sn.rt + sn.g / 2; kk = sn.k; }
      let at = sn ? sn.rt : x;
      if (!sn) { const n = await nearScan(kk, x); if (n) { r0 = n.rt - NEAR; r1 = n.rt + NEAR; at = n.rt; } }
      p.sel = null; p.cur = at;
      newSpec(p, r0, r1, kk);          // every double click makes a new LIVE spectrum right under the chromatogram; the previous ones freeze
      draw(p); return;
    }
    if (p.type === "spec") { p.lock = null; if (p.meas || p.rul) { p.meas = null; p.rul = false; const rb = p.el.querySelector('[data-a="rul"]'); if (rb) rb.classList.remove("on"); } }                     // double click on a spectrum: whole spectrum, free axes, no measures
    p.zoom = null; p.zoomY = null; p.sel = null; draw(p);
  };
  // no dragging needed (WCAG 2.5.7): on the focused chart + and - zoom, [ and ] move the visible range, 0 shows everything
  cv.addEventListener("keydown", e => {
    const a = p._a; if (e.ctrlKey || e.metaKey || e.altKey || !a || !a.full || p.type === "map" || e.target !== cv) return;
    const span = a.x1 - a.x0, c = p.cur != null && p.cur >= a.x0 && p.cur <= a.x1 ? p.cur : (a.x0 + a.x1) / 2;
    const z = e.key === "+" || e.key === "=" ? [c - (c - a.x0) / 2, c + (a.x1 - c) / 2] : e.key === "-" || e.key === "_" ? [a.x0 - span / 2, a.x1 + span / 2]
      : e.key === "[" ? [a.x0 - span / 4, a.x1 - span / 4] : e.key === "]" ? [a.x0 + span / 4, a.x1 + span / 4] : e.key === "0" ? [a.full[0], a.full[1]] : null;
    if (!z) return;
    e.preventDefault(); pushZh(p); p.zoom = clampView(z[0], z[1], a.full[0], a.full[1]); draw(p); uiSave();
  });
  cv.oncontextmenu = e => { if (p.type === "map" && window.MAPPA && MAPPA.ctx(p, e, rect(e), recty(e))) return; hideHover(p); ctxFor(p, e, xd(rect(e)), rect(e), recty(e)); };   // map: right click locks (mappa.js)
  if (p.type === "map" && window.MAPPA) MAPPA.init(p);
}
// high resolution only: the scan of the file (survey or product ions) closest in time to x, found by the server; null for any other file (the window around x is used)
async function nearScan(k, x) {
  const f = E.files[k]; if (!f || !window.HR || !HR.isHr(f, f.lv)) return null;
  try { const j = await J(`api/nearest_scan?k=${k}&rt=${x}&level=${f.lv || 1}`); return j && j.rt != null ? j : null; } catch (e) { return null; }
}
const NEAR = 1e-4;                                      // half width (min) of the window around the RT of one exact scan
const scanStep = () => { const f = scanFiles(tabFiles())[0]; return f ? Math.max((f.rt_max - f.rt_min) / Math.max(f.ms1 + f.ms2 - 1, 1), 0.005) : 0.02; };
/** @param {Pannello} p @param {number} x */
function nearestFile(p, x) {
  const a = p._a; if (a && a.map) return a.f.k; if (!a || !a.sr) return scanFiles(shownFor(p))[0]?.k ?? 0;
  /** @type {{ v: number, k: number } | null} */
  let best = null;
  a.sr.forEach(s => { if (E.files[s.k]?.kind === "mrm") return; const v = s.ys[nearIdx(s.x, x)]; if (!best || v > best.v) best = { v, k: s.k }; });
  return best ? best.k : (scanFiles(tabFiles())[0]?.k ?? 0);
}
function pushLinked(p, r0, r1, k, keepZoom, o = {}) {
  if (fsPanel()) return;                                      // full screen: the panels underneath stay exactly as they were
  if (!o.step) { p._msg = null; p._sk = null; }              // a click or a drag starts a new walk: the axes of the spectrum are free again
  E.panels.filter(s => s.type === "spec" && s.link === p.id).forEach(s => {
    s.r0 = r0; s.r1 = r1; s.k = k; s.si = o.i ?? null; if (o.dir) s._dir = o.dir;
    if (!keepZoom) s.zoom = null;
    if (!o.step) { s.lock = null; ctl(s); }                  // no rebuilding of the controls while scrolling
    if (o.step) scAhead(s);
    drawSoon(s);
  });
}
// A new spectrum from a chromatogram (double click, right click) is the "live" one: it sits right under the chromatogram (the older ones slide down),
// and only it follows clicks, cursor, arrows and selections. The older ones are frozen: they keep their time in the title and stay children of the
// chromatogram (src) so they travel with it. Closing the live one promotes nobody; "Ricollega al cromatogramma" in a spectrum's menu makes it live again.
function stackAfter(p, anchor) {
  if (!p.full || !anchor.full || !p.el || !anchor.el) return;
  const st = stackOrder().filter(q => q !== p); st.splice(st.indexOf(anchor) + 1, 0, p);
  let y = 0; st.forEach((q, i) => { q.y = y; apply(q); y += q.h + gapAfter(q, st[i + 1]); });
  E.panels.sort((a, b) => a.y - b.y || a.id - b.id);
}
function freezeSpec(s) {
  if (!s.link) return;
  s.src = s.src || s.link; s.link = null;
  if (/^@(spec|product)/.test(s.title) && s.r0 != null) s.title = s.level === 2 ? ptProduct(s.prec) : PT.spec;   // the time is written once, next to the title (.rtl)
  ctl(s);
}
function liveSpec(s, from) {
  E.panels.filter(q => q !== s && q.type === "spec" && q.link === from.id).forEach(freezeSpec);
  s.link = from.id; s.src = from.id; ctl(s);
  if (from.cur != null && s.r0 == null) { const dt = scanStep(); s.r0 = from.cur - dt / 2; s.r1 = from.cur + dt / 2; }
  draw(s); uiSave();
}
function newSpec(from, r0, r1, k, link = from.id) {
  if (link) E.panels.filter(q => q.type === "spec" && q.link === from.id).forEach(freezeSpec);
  const s = addPanel("spec", { link, src: from.id, k, r0, r1, level: E.files[k]?.lv || 1, prec: from.tab === "ms2" ? from.prec : null, title: from.tab === "ms2" ? ptProduct(from.prec) : PT.spec });
  
  // Disable transition temporarily so it doesn't fly from the bottom
  s.el.classList.add("drag");
  s.y = from.y + from.h;
  s.el.style.top = s.y + "px";
  void s.el.offsetHeight; // force reflow
  s.el.classList.remove("drag");
  
  stackAfter(s, from); relayout(); fitHost();
  // Do NOT reveal(s) so the viewport does not shift randomly
  return s;
}

// The peak a click on the spectrum means: a click inside the box of a peak label gives exactly that peak; otherwise the peak the mouse box highlights (p._a.snap).
// Hover, right-click menu, ruler and XIC all use it, so the m/z they show is always the same.
function specPick(p, px, py) {
  const a = p._a; if (!a) return null;
  const lb = (a.lbls || []).find(l => l.m != null && px >= l.x && px <= l.x + l.w && py >= l.y && py <= l.y + l.h);
  if (lb) return lb.m;
  const s = a.snap && a.snap(px); return s ? s.m : null;
}
function ctxFor(p, e, x, px, py) {
  if (!p._a) return e.preventDefault();
  const items = [];
  const inLbl = a => (p._a.lbls || []).some(l => l.ann === a && px >= l.x && px <= l.x + l.w && py >= l.y && py <= l.y + l.h);
  const near = p.anns.find(a => a.x >= p._a.x0 && a.x <= p._a.x1 && (inLbl(a) || Math.abs(p._a.X(a.x) - px) < 14));
  if (p.type === "map") {
    const a = p._a, mz = a.mzAt(py), lab = mz.toFixed(mzd(p)), f = a.f, dt = scanStep();
    items.push({ label: I18N.t("ctx.map.where", { rt: x.toFixed(2), mz: lab, file: f.label }), dim: true }, "-");
    items.push({ label: I18N.t("ctx.map.xic", { mz: lab }), fn: () => openXic(null, { mz, obs: true }) });
    tabPanels().filter(q => q.type === "xic").forEach(q => items.push({ label: I18N.t("ctx.map.addTo", { mz: lab, panel: ptitle(q.title) }), fn: () => openXic(q, { mz, obs: true }) }));
    items.push("-");
    if (p.sel) items.push({ label: I18N.t("ctx.spec.avgSel"), fn: () => newSpec(p, p.sel[0], p.sel[1], f.k) });
    items.push({ label: I18N.t("ctx.spec.atRt"), fn: () => newSpec(p, x - dt / 2, x + dt / 2, f.k) });
  } else if (p.type === "spec") {
    const a = p._a, d0 = a.data[0].d;
    const m = specPick(p, px, py);                          // the SAME peak that the label and the mouse box show; on empty space the m/z entries are grey
    const has = m != null, lab = has ? m.toFixed(mzd(p, p.level)) : "…", mi = o => has ? o : { label: o.label, dim: true };
    items.push({ label: has ? `m/z ${lab}` : I18N.t("ctx.spec.noPeak"), dim: true }, "-");
    p._pm = has ? m : null;
    items.push(mi({ label: I18N.t("ctx.spec.copy"), aria: I18N.t("ctx.spec.copy.aria", { mz: lab }), fn: () => { try { navigator.clipboard.writeText(lab).catch(() => {}); } catch (_) { /* clipboard unavailable */ } } }));
    let mark = items.length;                                  // HR: the entries are grouped at the end (tag), the others keep their order
    const tag = g => { for (let i = mark; i < items.length; i++) if (items[i] !== "-" && !items[i].g) items[i].g = g; mark = items.length; };
    if (a.hrp && window.DDA && has) { const q = DDA.ms2For(p, m)[0]; if (q) { items.push({ label: I18N.t("ctx.spec.openMs2", { mz: q.prec.toFixed(mzd(p, 2)) }), aria: I18N.t("ctx.spec.openMs2.aria", { mz: lab }), fn: () => DDA.openMs2(p, q.j) }); tag("ms2"); } }
    items.push(mi({ label: I18N.t("ctx.spec.xic", { mz: lab }), fn: () => xicDirect(m, p) })); tag("xic");
    if (window.LISTE) {
      const pol = a.data[0].f && a.data[0].f.polarity;
      items.push(mi({ label: I18N.t("ctx.spec.contLists"), fn: () => LISTE.open({ mz: m, polarity: pol, hr: !!a.hrp }) }));
      if (a.hrp) {                                              // known contaminants (high resolution): information, and the contaminants of the laboratory
        const kn = has && LISTE.isOn() && a.cont ? a.cont.match(m) : [];
        if (kn.length) items.push({ label: I18N.t("ctx.spec.known", { name: kn[0].name, adduct: kn[0].adduct || I18N.t("ctx.spec.knownOnlyMz"), err: (kn[0].err >= 0 ? "+" : "") + kn[0].err.toFixed(1), more: kn.length > 1 ? I18N.t("ctx.spec.knownMore", { n: kn.length - 1 }) : "" }), dim: true });
        items.push(mi({ label: I18N.t("ctx.spec.addLab"), fn: () => LISTE.askAdd(m, pol) }));
      }
      tag("cont");
    }
    if (a.hrp && window.DISP && window.BANCO && BANCO.on()) { items.push({ label: I18N.t("ctx.spec.marker"), check: markOn(p), aria: I18N.t("ctx.spec.marker.aria"), fn: () => { p.mk = !markOn(p); draw(p); uiSave(); } }, { label: I18N.t("ctx.spec.noCont"), check: !!p.noCont, aria: I18N.t("ctx.spec.noCont.aria"), fn: () => { p.noCont = !p.noCont; draw(p); uiSave(); } }); tag("view"); }
    if (window.DISP && window.BANCO && BANCO.on()) { items.push({ label: I18N.t("disp.menu"), aria: I18N.t("disp.menu.aria"), fn: () => DISP.open(p) }); tag("disp"); }
    if (a.hrp && window.ELEM) {
      items.push(mi({ label: I18N.t("ctx.spec.generate", { mz: lab }), fn: () => ELEM.generate({ mz: m, sp: p, spec: a.data[0].d, polarity: a.data[0].f && a.data[0].f.polarity }) }));
      if (p.fm && p.fm.length) items.push({ label: I18N.t("cx.clear"), fn: () => { p.fm = []; draw(p); } });
      tag("gen");
    }
    const xs = tabPanels().filter(q => q.type === "xic"), TIPX = I18N.t("ctx.spec.overlay.tip");
    const ov = q => ({ label: I18N.t("ctx.spec.overlay", { name: xicName(q), n: q.num || "?" }), tip: TIPX, fn: () => xicDirect(m, p, q) });
    if (xs.length > 3) items.push(mi({ label: I18N.t("ctx.spec.overlayMany"), tip: TIPX, fn: () => menu({ preventDefault() {}, clientX: e.clientX, clientY: e.clientY }, xs.map(ov)) }));
    else xs.forEach(q => items.push(mi(ov(q))));
    tag("xic");
    if (window.DDA) { const dm = DDA.menu(p, px, py, m); if (dm.length) items.push("-", ...dm); }       // DDA trio: average of the MS2 of an ion, follow an ion
    tag("dda");
    items.push("-", ...measMenu(p, m).map((o, i) => i === 0 ? mi(o) : o));
    items.push(mi({ label: I18N.t("ctx.spec.annotate"), fn: async () => { const v = await ask(I18N.t("ctx.spec.annotate.prompt", { mz: lab }), ""); if (v) { p.anns.push({ x: m, text: v }); draw(p); } } }));
    items.push({ label: I18N.t("ctx.spec.iso"), fn: async () => {
      const v = await ask(I18N.t("ctx.spec.iso.prompt", { adduct: EH(defAdduct(a.data.map(x => x.f))) }), p.iso ? p.iso.formula + " " + p.iso.ad : "");
      if (!v) return;
      const mm = v.match(/^\s*(\S+)\s*(\[.*\].*)?$/); if (!mm) return;
      let ftxt = mm[1], adt = mm[2];
      const chg = ftxt.match(/^(.+?)([+\-\u2212])$/);               // a formula written with its charge (C10H14N+, as copied from the drawing): the ion as it is, no proton added
      if (chg) { ftxt = chg[1]; if (!adt) adt = chg[2] === "+" ? "[M]+" : "[M]-"; }
      try { const r = await getFormula(ftxt, ""); p.iso = { formula: r.formula, ad: (adt || defAdduct(a.data.map(x => x.f))).trim() };
        const ion = QQQRef.ionCounts(p.iso.formula, p.iso.ad), pat = QQQRef.isoPattern(ion.n, ion.z);
        p.zoom = [pat[0].mz - 4, pat[pat.length - 1].mz + 4]; p._isoKey = JSON.stringify(p.zoom); draw(p); }   // zoom on the pattern (double click: whole spectrum); a later change of zoom switches the simulation off
      catch (e) { info(I18N.t("ctx.spec.iso.fail", { message: EH(e.message) })); }
    } });
    if (p.iso && window.HR && a.hrp) items.push({ label: I18N.t("ctx.spec.isoRes"), fn: async () => {
      const v = await ask(I18N.t("ctx.spec.isoRes.prompt"), p.iso.R != null ? String(p.iso.R) : "auto");
      if (v == null) return; const t = String(v).trim().replace(",", ".");
      if (t === "auto") delete p.iso.R; else { const r = parseFloat(t); if (!(r >= 0)) { info(I18N.t("ctx.spec.isoRes.number")); return; } p.iso.R = r; }
      draw(p);
    } });
    if (p.iso) items.push({ label: I18N.t("ctx.spec.isoOff"), fn: () => { p.iso = null; draw(p); } });
    tag("tools");
    if (window.LIB && p.level === 2) items.push("-", LIB.isHr(p) ? { label: I18N.t("ctx.libWhat"), fn: () => LIB.whatIs(p) } : { label: I18N.t("ctx.libSearch"), fn: () => LIB.searchFrom(p) });       // «What is it?» only for high resolution
    if (p.level === 2 && window.WORKFLOW) items.push("-", { label: I18N.t("ctx.spec.exportMgf"), fn: () => WORKFLOW.exportSpectrumMgf(p) }, { label: I18N.t("ctx.spec.exportMsp"), fn: () => WORKFLOW.exportSpectrumMsp(p) });
    if (p.level === 2 && window.FAIR && FAIR.available(p)) items.push({ label: I18N.t("ctx.spec.exportMassbank"), fn: () => FAIR.massbank(p) }, { label: I18N.t("ctx.spec.exportSirius"), fn: () => FAIR.sirius(p) });
    tag("lib");
    const srcP = p.src && E.panels.find(q => q.id === p.src && q.el);
    if (window.DDA) { const q = DDA.menuShow(p); if (q) items.push("-", q); }
    if (p.link) items.push("-", { label: I18N.t("ctx.spec.freeze"), fn: () => { freezeSpec(p); uiSave(); } });
    else if (srcP) items.push("-", { label: I18N.t("ctx.spec.relink"), fn: () => liveSpec(p, srcP) });
    tag("link");
    if (a.hrp) {                                              // HR: open the MS2, then formula and display, then the rest in groups with separators
      const ORDER = [["ms2"], ["gen", "view", "disp"], ["xic"], ["cont"], ["tools"], ["dda"], ["lib"], ["link"]], head = items.splice(0, 2), body = items.splice(0).filter(i => i !== "-");
      items.push(...head);
      ORDER.forEach(g => { const l = body.filter(i => g.includes(i.g)).sort((a, b) => g.indexOf(a.g) - g.indexOf(b.g)); if (l.length) { if (items.length > head.length) items.push("-"); items.push(...l); } });
    }
  } else {
    const k = nearestFile(p, x);
    items.push({ label: I18N.t("ctx.chrom.where", { rt: x.toFixed(2), file: E.files[k]?.label || "" }), dim: true }, "-");
    const dt = scanStep();
    if (p.type !== "mrm") {
      if (p.sel) items.push({ label: I18N.t("ctx.spec.avgSel"), fn: () => newSpec(p, p.sel[0], p.sel[1], k) });
      items.push({ label: I18N.t("ctx.spec.atRt"), fn: () => { const sn = p.tab === "ms2" && p.type === "chrom" ? ms2Near(p, x, 0) : null; if (sn) { p.cur = sn.rt; newSpec(p, sn.rt - sn.g / 2, sn.rt + sn.g / 2, sn.k); } else nearScan(k, x).then(n => { if (n) { p.cur = n.rt; newSpec(p, n.rt - NEAR, n.rt + NEAR, k); } else newSpec(p, x - dt / 2, x + dt / 2, k); }); } });
    }
    if (p.type === "chrom") items.push({ label: I18N.t("ctx.chrom.xic"), fn: () => openXic(null, { after: p }) });
    if (window.DISP && window.BANCO && BANCO.on() && (p.type === "chrom" || p.type === "xic")) items.push({ label: I18N.t("disp.menu"), fn: () => DISP.open(p) });
    if (window.GRADIENTE && GRADIENTE.pressHave(p) && !isAnalog(p.kind)) items.push({ label: I18N.t("ctx.chrom.press"), check: p.overlay === "press", aria: I18N.t("ctx.chrom.press.aria"), fn: () => { p.overlay = p.overlay === "press" ? null : "press"; draw(p); uiSave(); } });
    if (window.GRADIENTE && GRADIENTE.have(p)) items.push({ label: I18N.t("ctx.chrom.grad"), check: p.overlay === "grad", aria: I18N.t("ctx.chrom.grad.aria"), fn: () => { p.overlay = p.overlay === "grad" ? null : "grad"; draw(p); uiSave(); } });
    if (p.type === "chrom" && window.DDA) { const q = DDA.menuShow(p); if (q) items.push(q); }
    if (p.type === "chrom" && window.LIB && window.HR && HR.isHr(E.files[k], 2) && E.files.some(x => x.file.split("#")[0] === E.files[k].file.split("#")[0] && x.kind === "ms2")) items.push({ label: I18N.t("ctx.chrom.identAll"), fn: () => LIB.identifyAll(k) });
    if (p.type === "chrom" && window.COMP && window.HR && HR.isHr(E.files[k], 2) && E.files[k].kind === "ms2") items.push({ label: I18N.t("ctx.chrom.tree"), fn: () => COMP.tree(k) });
    if (p.type === "xic") {
      items.push({ label: I18N.t("ctx.chrom.addIon"), fn: () => openXic(p) });
    }
    items.push("-", ...intMenuItems(p, x, near, py));
    { const ib = ibkMenuItems(p); if (ib.length) items.push("-", ...ib); }
    items.push("-", { label: I18N.t("ctx.annotate.point"), fn: async () => { const v = await ask(I18N.t("ctx.annotate.prompt", { rt: x.toFixed(2) }), ""); if (v) { p.anns.push({ x, text: v }); draw(p); } } });
  }
  if (near) items.push("-", { label: I18N.t("ctx.ann.edit", { text: near.text }), fn: async () => { const v = await ask(I18N.t("ctx.ann.prompt"), near.text); if (v != null) { if (v === "") p.anns = p.anns.filter(a => a !== near); else near.text = v; draw(p); } } },
    { label: I18N.t("ctx.ann.delete", { text: near.text }), fn: () => { p.anns = p.anns.filter(a => a !== near); draw(p); } });
  if (p.zoom || p.zoomY) items.unshift({ label: I18N.t("ctx.resetZoom"), fn: () => { p.zoom = null; p.zoomY = null; draw(p); } }, "-");     // with a zoom active it is always the first entry
  menu(e, items);
}

// LC method of the laboratory (gradient, flow, PDA, oven): it is stored in the .wiff/.dam files, not in the mzML
function lcHtml(lc, row) {
  const g = lc.gradient || [], T = lc.run_time || (g.length ? g[g.length - 1].t : 22), RH = 27;       // the figure is as tall as the table (rows of 27 px)
  const w = 440, hh = Math.max(160, (g.length + 1) * RH), l = 40, r = 10, t = 10, b = 34, X = v => l + v / T * (w - l - r), Y = v => t + (100 - v) / 100 * (hh - t - b);
  const pts = g.map(q => [q.t, q.b]), ink = css("--muted"), acc = css("--accent");
  let svg = `<svg viewBox="0 0 ${w} ${hh}" width="${w}" height="${hh}" style="background:#fff;border:1px solid ${css("--line")};border-radius:6px;flex:none;max-width:100%" font-family="system-ui" font-size="11"><path d="M${l} ${t}V${hh - b}H${w - r}" fill="none" stroke="${ink}"/>`;
  for (const v of [0, 25, 50, 75, 100]) svg += `<path d="M${l - 3} ${Y(v)}H${l}" stroke="${ink}"/><text x="${l - 6}" y="${Y(v) + 4}" text-anchor="end" fill="${ink}">${v}</text>`;
  for (let v = 0; v <= T; v += 5) svg += `<path d="M${X(v)} ${hh - b}v3" stroke="${ink}"/><text x="${X(v)}" y="${hh - b + 15}" text-anchor="middle" fill="${ink}">${v}</text>`;
  svg += `<polyline fill="none" stroke="${acc}" stroke-width="2" points="${pts.map(q => X(q[0]) + "," + Y(q[1])).join(" ")}"/>${pts.map(q => `<circle cx="${X(q[0])}" cy="${Y(q[1])}" r="2.6" fill="${acc}"/>`).join("")}`;
  svg += `<text x="${(l + w - r) / 2}" y="${hh - 4}" text-anchor="middle" fill="${css("--ink")}">${I18N.t("method.time")}</text><text transform="translate(11 ${(t + hh - b) / 2}) rotate(-90)" text-anchor="middle" fill="${css("--ink")}">% B</text></svg>`;
  const pda = lc.pda || {};
  const tbl = `<table class="lct"><tr><th class="num">${I18N.t("method.time")}</th><th class="num">% A</th><th class="num">% B</th><th class="num">${I18N.t("method.flow")}</th></tr>${g.map(q => `<tr><td class="num">${q.t}</td><td class="num">${+(100 - q.b).toFixed(1)}</td><td class="num">${q.b}</td><td class="num">${q.flow}</td></tr>`).join("")}</table>`;
  const chips = [lc.run_time != null ? [I18N.t("method.runTime"), lc.run_time + " min"] : null, lc.oven != null ? [I18N.t("method.oven"), lc.oven + " °C"] : null].filter(Boolean);
  return `<section class="msec"><h4>${I18N.t("method.lc.title")}</h4>
   <div class="sm" style="margin:0 0 8px">A = H<sub>2</sub>O + 0.1% FA &nbsp;&middot;&nbsp; B = ACN + 0.1% FA</div>
   <div style="display:flex;gap:22px;align-items:flex-start;flex-wrap:wrap">${svg}<div style="flex:1 1 300px;max-width:520px">${tbl}</div></div>
   ${chips.length ? `<div class="mchips">${chips.map(c => `<span><span class="muted">${c[0]}</span> <b>${c[1]}</b></span>`).join("")}</div>` : ""}</section>
   ${pda.start != null ? `<section class="msec"><h4>${I18N.t("method.pda.title")}</h4><table>${row(I18N.t("method.pda.range"), I18N.t("method.pda.value", { start: pda.start, stop: pda.stop, step: pda.step }))}</table></section>` : ""}`;
}

// ------------------------------------------------------------------ metodo di acquisizione (letto dagli mzML)
async function showMethod(sel) {
  const again = () => showMethod(sel);
  const f = E.files[E.cur]; if (!f) return info(I18N.t("method.none"));
  let m; try { m = await J("api/method?k=" + f.k); } catch (e) { return info(I18N.t("method.error", { message: EH(e.message) })); }
  const row = (a, b) => b == null || b === "" ? "" : `<tr><td class="muted">${EH(a)}</td><td>${EH(String(b))}</td></tr>`;   // values come from the file: always escaped here
  const pol = ["positive", "negative", "mixed", "unknown"].includes(m.polarity) ? I18N.t(`method.pol.${m.polarity}`) : m.polarity;
  const labs = m.lab || [], li = labs.length ? Math.min(Math.max(sel ?? m.lab_pick ?? 0, 0), labs.length - 1) : -1, lab = labs[li];
  const sp = m.scan_params && !m.scan_params.error ? m.scan_params : null;       // high resolution / DDA file: the parameters stored in the scans themselves
  const spNote = `<div style="background:var(--soft,#eef2f7);border-left:4px solid var(--accent);border-radius:6px;padding:8px 12px;margin-bottom:8px">${I18N.t("method.scanParamsNote")}</div>`;
  let h = labs.length ? "" : sp ? spNote : `<div style="background:#fff4e0;border-left:4px solid #e08a00;border-radius:6px;padding:8px 12px;margin-bottom:8px">${I18N.t("method.missing")}<div style="margin-top:6px"><button class="go" id="m-load">${I18N.t("method.loadDam")}</button></div></div>`;
  const sec = (title, body, extra = "") => `<section class="msec"><h4>${title}${extra}</h4>${body}</section>`;
  const HOW = { q1: I18N.t("method.how.q1"), ems: I18N.t("method.how.ems"), ms2: I18N.t("method.how.ms2"), mrm: I18N.t("method.how.mrm") };

  const scanLike = m.kind === "full" || m.kind === "ms2";
  const expTbl = `<table>${row(I18N.t("method.row.polarity"), pol)}${scanLike ? row(I18N.t("method.row.massRange"), m.scan_window ? `m/z ${m.scan_window[0]}-${m.scan_window[1]}` : "") : ""}${row(I18N.t("method.row.duration"), m.rt_max > 0 ? `${m.rt_min.toFixed(2)}-${m.rt_max.toFixed(2)} min` : "")}${scanLike ? row(I18N.t("method.row.scans"), m.scans || "") + row(I18N.t("method.row.cycle"), m.cycle_s ? m.cycle_s.toFixed(2) + " s" : "") : ""}${m.kind === "ms2" ? row(I18N.t("method.row.precursors"), m.precursors.join(", ")) + row(I18N.t("method.row.ce"), m.ce.length ? m.ce.join(", ") + " eV" : "") : ""}</table>`;
  const labName = x => x.sample ? I18N.t("method.name", { n: x.sample }) : x.name;       // «Metodo 1»: the .dam holds several methods, one per sample
  const trTbl = m.transitions.length ? `<table><tr><th>${I18N.t("method.tr.name")}</th><th class="num">Q1 (m/z)</th><th class="num">Q3 (m/z)</th><th class="num">CE (eV)</th><th class="num">Dwell (ms)</th></tr>${m.transitions.map(t => `<tr><td>${EH(t.name || "")}</td><td class="num">${EH(String(t.q1))}</td><td class="num">${EH(String(t.q3))}</td><td class="num">${EH(String(t.ce ?? ""))}</td><td class="num">${t.dwell != null ? Math.round(t.dwell * 1000) : ""}</td></tr>`).join("")}</table>` : "";
  h += `<div class="msub"><span class="tag">${kindOf(f)}</span> <b>${EH(f.label)}</b> <span class="muted">· ${EH(m.instrument)} (${EH(m.serial)})</span></div><div class="mgrid">`;
  h += sec(I18N.t("method.sec.experiment"), `<div class="sm hw">${HOW[m.kind === "full" ? (f.mode === "ems" ? "ems" : "q1") : m.kind] || ""}</div>${expTbl}${trTbl}`);
  if (sp && !lab) {
    const n1 = sp.ms1, n2 = sp.ms2, nf = (v, d = 0) => v == null ? "" : (+v).toFixed(d), join = a => a.join(", ");
    const t1 = `<table>${row(I18N.t("method.row.analyzer"), n1.an)}${row(I18N.t("method.row.massRange"), n1.window ? `m/z ${n1.window[0]}-${n1.window[1]}` : "")}${row(I18N.t("method.row.resolution"), n1.res ? nf(n1.res) : "")}${row(I18N.t("method.row.inject"), n1.inject != null ? nf(n1.inject, 1) + " ms" : "")}${row(I18N.t("method.row.scansN"), nf(n1.n))}</table>`;
    const t2 = n2 ? `<table>${row(I18N.t("method.row.activation"), join(n2.act))}${row(I18N.t(n2.nce ? "method.row.ceNce" : "method.row.ce"), n2.ce.length ? join(n2.ce) + (n2.nce ? "" : " eV") : "")}${row(I18N.t("method.row.isolation"), n2.iso.length ? "±" + join(n2.iso) + " m/z" : "")}${row(I18N.t("method.row.analyzer"), n2.an)}${row(I18N.t("method.row.resolution"), n2.res ? nf(n2.res) : "")}${row(I18N.t("method.row.inject"), n2.inject != null ? nf(n2.inject, 1) + " ms" : "")}${row("MS2", I18N.t("method.row.ms2n", { n: nf(n2.n), per: n2.per_cycle }))}</table>` : "";
    h += sec(I18N.t("method.sec.scanParams"), `<div class="sm hw"><b>Full Scan (MS1)</b></div>${t1}${n2 ? `<div class="sm hw" style="margin-top:8px"><b>MS2</b></div>${t2}` : ""}`);
  }
  if (lab && lab.check && lab.check.length) {
    const ic = { ok: `<span style="color:var(--ok)">${I18N.t("method.check.ok")}</span>`, diff: `<span style="color:var(--bad)">${I18N.t("method.check.diff")}</span>`, na: `<span class="muted">${I18N.t("method.check.na")}</span>` }, mtx = x => x && x.key ? I18N.t(x.key, x.params) : String(x ?? "");
    const bad = lab.check.filter(r => r.status === "diff").length;
    h += sec(I18N.t("method.check.title"), `<table><tr><th>${I18N.t("method.check.what")}</th><th>${I18N.t("method.check.method")}</th><th>${I18N.t("method.check.file")}</th><th></th></tr>${lab.check.map(r => `<tr><td>${EH(mtx(r.what))}</td><td>${EH(mtx(r.method))}</td><td>${EH(mtx(r.data))}</td><td>${ic[r.status] || ""}</td></tr>`).join("")}</table>
      ${bad ? `<div class="sm" style="margin-top:4px;color:var(--bad)">${I18N.t("method.check.bad", { n: bad, another: labs.length > 1 ? I18N.t("method.check.another") : "" })}</div>` : ""}`);
  }
  if (lab) {
    const pr = a => a.map(s => `<tr><td class="muted">${EH(s.label)}</td><td>${s.id === "ihe" ? I18N.t(s.value ? "method.source.on" : "method.source.off") : s.value + " " + s.unit}</td></tr>`).join("");
    const pick = labs.length > 1 ? `<div class="sm" style="margin:2px 0 6px">${I18N.t("method.source.pick", { select: `<select id="m-sel">${labs.map((x, i) => `<option value="${i}" ${i === li ? "selected" : ""}>${EH(labName(x))}</option>`).join("")}</select>` })}</div>` : `<div class="muted sm" style="margin:2px 0 6px">${I18N.t("method.source.from", { name: EH(labName(lab)) })}</div>`;
    h += sec(I18N.t("method.source.title"), pick + (lab.error ? `<div class="fail">${EH(lab.error_key ? I18N.t(lab.error_key, lab.params) : lab.error)}</div>` : lab.source.length || lab.compound.length ? `<table>${pr(lab.source)}${pr(lab.compound)}</table>` : `<div class="muted sm">${I18N.t("method.source.none")}</div>`));
  }
  h += "</div>";
  if (lab) {
    if (lab.lc) h += lcHtml(lab.lc, row);
    else if (!lab.error) h += `<div class="muted sm" style="margin-top:6px">${I18N.t("method.noLc")}</div>`;
  }
  const gradHere = !!(lab && lab.lc && lab.lc.gradient && lab.lc.gradient.length > 1);
  if (!gradHere && window.GRADIENTE) h += await GRADIENTE.html(f);       // no gradient in the .dam: the student can enter one
  h = h.replace(/m\/z/g, "<i>m/z</i>");   // m/z is always in italics
  const d = Q("#bigdlg"); if (d.open) d.close();
  big(I18N.t("method.dialog.title"), h, () => {
    if (!gradHere && window.GRADIENTE) GRADIENTE.bind(f, again);
    const sl = Q("#m-sel"); if (sl) sl.onchange = () => showMethod(+sl.value);
    const ld = Q("#m-load"); if (ld) ld.onclick = loadDam;
  });
}
// the method (.dam) can also be loaded here, without going back to the start screen
function loadDam() {
  const inp = document.createElement("input"); inp.type = "file"; inp.accept = ".dam,.DAM";
  inp.onchange = async () => {
    const fl = inp.files[0]; if (!fl) return;
    try {
      const j = await (await fetch("api/upload?name=" + encodeURIComponent(fl.name), { method: "POST", body: fl })).json();
      if (j.error) throw I18N.err(j);
      if (typeof ST !== "undefined") { ST.methods = j.methods || ST.methods; renderMethods(); }
      methodWarn(); showMethod();
    } catch (e) { info(I18N.t("method.error", { message: EH(e.message) })); }
  };
  inp.click();
}
Q("#np-method").onclick = () => showMethod();

// ------------------------------------------------------------------ BioTransformer
function openBT() { window.open("https://biotransformer.ca/new", "_blank", "noopener"); }
window.openBT = openBT;

// ------------------------------------------------------------------ barra strumenti
Q("#np-chrom").onclick = () => reveal(addPanel("chrom", {}));
Q("#np-spec").onclick = () => { const f = scanFiles(vis())[0] || scanFiles(tabFiles())[0]; reveal(addPanel("spec", { k: f?.k ?? 0, level: f?.lv || 1 })); };
Q("#np-xic").onclick = () => openXic(null);
Q("#np-map").onclick = () => { const f = scanFiles(tabFiles("full").filter(x => x.vis))[0] || scanFiles(tabFiles("full"))[0]; reveal(addPanel("map", { k: f?.k ?? 0, tab: "full" })); };
Q("#np-mrm").onclick = () => reveal(addPanel("mrm", { imode: "man", intf: "all" }));
const closeAllBtn = Q("#np-closeall");
if (closeAllBtn) closeAllBtn.onclick = closeAllPanels;
Q("#addf").onclick = async () => { STATO.aggiunta(true, "add-files"); try { const d = await J("api/state"); if (d.methods) { ST.methods = d.methods; renderMethods(); } } catch (_) { /* the list stays as it was */ } applyView(); };
addEventListener("resize", () => { fitWidth(); redrawSoon(); });
document.addEventListener("tpview", e => { if (/** @type {CustomEvent} */ (e).detail.view === "data") setTimeout(() => { fitWidth(); redrawAll(); }, 0); });

// ------------------------------------------------------------------ calcolatrice formula -> m/z (come «mass from formula» di Xcalibur)
// the calculator is a drop-down under its button (no dark backdrop): the graphs stay visible and usable; it closes with the button, Esc or the ×
const calcBox = Q("#calcdlg");
Object.defineProperty(calcBox, "open", {
  get: () => {
    if (window.BARRA && BARRA.isDataView()) return BARRA.isOpen("calc");
    return !calcBox.hidden;
  }
});
calcBox.close = () => {
  if (window.BARRA && BARRA.isDataView()) {
    BARRA.close(true);
  } else {
    calcBox.hidden = true;
  }
};
function calcPlace() {
  const r = Q("#np-calc2").getBoundingClientRect(), w = Math.min(560, innerWidth - 16);
  calcBox.style.top = Math.round(r.bottom + 6) + "px"; calcBox.style.left = Math.max(8, Math.min(innerWidth - w - 8, r.left)) + "px";
}
// outside the Dati view the calculator is a drop-down under its toolbar button (in the Dati view it is the "Calcolatrice" panel of the drawer)
window.calcToggleFloat = () => {
  if (!calcBox.hidden) return calcBox.close();
  calcBox.hidden = false;
  calcPlace();
  Q("#calcin").focus();
  calcRun();
};
Q("#calcx").onclick = () => calcBox.close();
addEventListener("resize", () => { if (!calcBox.hidden && !calcBox.classList.contains("sb-docked")) calcPlace(); });
document.addEventListener("keydown", e => {
  if (e.key !== "Escape") return;
  if (window.BARRA && BARRA.isDataView()) {
    const i = Q("#calcin");
    if (document.activeElement === i && i.value) {
      i.value = ""; CALC.fresh = false; calcRun();
    }
    return;                                                  // the drawer itself closes with Esc (barra.js)
  }
  if (calcBox.hidden) return;
  const i = Q("#calcin"); if (i.value) { i.value = ""; CALC.fresh = false; calcRun(); } else calcBox.close();      // Esc: first clear the field, then close
});
const CALC = { ans: 0, tape: [], fresh: false };       // fresh: the field holds a result; a digit typed now starts a new calculation
let CALC_MORE = false;       // the calculator shows the adducts of the other polarity too
// the field holds a CALCULATION (digits and + - x / ( )) -> big result, keypad and tape; or a chemical FORMULA -> table of the adducts
function calcTape() {
  const tp = Q("#calctape"); tp.hidden = !CALC.tape.length || !Q("#calcpad") || Q("#calcpad").hidden;
  tp.innerHTML = CALC.tape.map((r, i) => `<div data-i="${i}" title="${I18N.t("calc.tape.title")}">${EH(r.e)} = <b>${EH(r.r)}</b></div>`).join("");
  tp.querySelectorAll("div").forEach(d => d.onclick = () => { const r = CALC.tape[+d.dataset.i]; Q("#calcin").value = r.r; CALC.ans = parseFloat(r.r); CALC.fresh = true; calcRun(); Q("#calcin").focus(); });
}
function calcEquals() {
  const t = Q("#calcin").value.trim(), r = calcEval(t, CALC.ans); if (r.v == null) return;
  const f = calcFmt(r.v); CALC.tape.unshift({ e: calcPretty(t.replace(/ans/gi, "Ans")), r: f }); CALC.tape.length = Math.min(CALC.tape.length, 10);
  CALC.ans = r.v; Q("#calcin").value = f; CALC.fresh = true; calcRun();
}
function calcKey(k) {
  const i = Q("#calcin");
  if (k === "=") return calcEquals();
  if (CALC.fresh && /[0-9(,.]/.test(k)) i.value = "";
  CALC.fresh = false;
  i.value = k === "C" ? "" : k === "\u2190" ? i.value.slice(0, -1) : i.value + k;
  i.focus(); calcRun();
}
Q("#calcpad").querySelectorAll("button").forEach(b => { b.onmousedown = ev => ev.preventDefault(); b.onclick = () => calcKey(b.dataset.k); });      // the field keeps the focus
Q("#calcin").onkeydown = ev => {
  if (ev.key === "Enter" && calcIsExpr(ev.target.value)) { ev.preventDefault(); calcEquals(); return; }
  if (CALC.fresh && /^[0-9(,.]$/.test(ev.key)) ev.target.value = "";
  if (ev.key.length === 1 || ev.key === "Backspace") CALC.fresh = false;
};
// the optional field «m/z osservata» of the calculator: only with a high-resolution file loaded (the field stays outside #calcout, so what is typed survives the redraws)
function calcObs(on) {
  let w = Q("#calcobsw");
  if (!w && on) {
    Q("#calcsum").insertAdjacentHTML("afterend", '<label id="calcobsw" class="sm muted" style="display:block;margin:4px 0" title="' + I18N.t("calc.obs.title") + '">' + I18N.t("calc.obs") + ' <input id="calcobs" inputmode="decimal" autocomplete="off" style="width:130px;margin:0 0 0 6px"></label>');
    Q("#calcobs").oninput = calcRun; w = Q("#calcobsw");
  }
  if (w) w.hidden = !on;
}
async function calcRun() {
  const t = Q("#calcin").value.trim(), out = Q("#calcout"), sum = Q("#calcsum"), res = Q("#calcres"), pad = Q("#calcpad"), nlBtn = Q("#calc-nl");
  const ex = !t || calcIsExpr(t);
  pad.hidden = !ex; res.hidden = !ex || !t; if (nlBtn) nlBtn.hidden = true; calcTape();
  if (!t) { out.innerHTML = ""; sum.innerHTML = ""; res.textContent = ""; return; }
  if (ex) {
    out.innerHTML = ""; const r = calcEval(t, CALC.ans); res.textContent = r.v != null ? calcFmt(r.v) : "";
    if (nlBtn && r.v != null && Math.abs(r.v) > 0.0001) {
      nlBtn.hidden = false;
      const vabs = calcFmt(Math.abs(r.v));
      nlBtn.title = I18N.t("calc.toLosses", { v: vabs });
      nlBtn.onclick = () => {
        if (window.BARRA && BARRA.isDataView()) {
          BARRA.setTab("losses", { q: vabs });
        } else {
          calcBox.close();
          if (window.QQQRef) QQQRef.open("ls", { q: vabs });
        }
      };
    }
    sum.innerHTML = r.err ? EH(r.err) : ""; return;
  }
  try {
    const r = await getFormula(t, "");
    const want = defAdduct();
    sum.innerHTML = t.replace(/\s+/g, "") !== r.formula ? I18N.t("calc.interpreted", { formula: fmtFormula(r.formula) }) : "";      // what was calculated, when the capitals were fixed
    // only the adducts of the polarity of the files shown (positive files: [M+H]+, [M+Na]+...; negative: [M-H]-...); no files or mixed: both, under ESI+ / ESI-; a small link shows the other polarity
    const fl = shown().length ? shown() : E.files.filter(f => f.vis), pos = fl.some(f => f.polarity === "positive"), neg = fl.some(f => f.polarity === "negative");
    const mine = pos && !neg ? "+" : neg && !pos ? "-" : "", both = !mine || CALC_MORE;
    const ent = Object.entries(r.adducts), isPos = a => a.endsWith("+");
    // high resolution (an Orbitrap / Q-TOF file is loaded): m/z with the decimals of the profile and, if the student writes the m/z measured, the error in ppm of each adduct (the program does not guess which one it is)
    const hrf = window.HR ? E.files.filter(f => !f.gone && (HR.isHr(f, 1) || HR.isHr(f, 2))) : [], hr = hrf.length > 0, dd = hr ? Math.max(HR.dec(hrf, 1), HR.dec(hrf, 2)) : 4;
    calcObs(hr); const ob = hr ? numIt(Q("#calcobs").value) : NaN, hasObs = hr && Number.isFinite(ob) && ob > 0;
    const rowOf = ([a, v]) => `<tr${a === want ? ' style="font-weight:600"' : ""}><td>${fmtAdduct(a)}</td><td>${hr ? v.mz5.toFixed(dd) : v.mz.toFixed(4)}</td>${hasObs ? `<td title="${((ob - v.mz5) * 1000).toFixed(1)} mDa">${HR.ppm(ob, v.mz5).toFixed(1)}</td>` : ""}<td>${v.nominal}</td><td><button data-a="${EH(a)}" data-m="${v.mz1}" title="${I18N.t("calc.xic.title")}">XIC</button></td></tr>`;
    const grp = (lab, rows) => rows.length ? `<tr class="pg"><td colspan="${hasObs ? 5 : 4}"><b>${lab}</b></td></tr>` + rows.map(rowOf).join("") : "";
    const P = ent.filter(([a]) => isPos(a)), N = ent.filter(([a]) => !isPos(a));
    const body = !both ? (mine === "+" ? P : N).map(rowOf).join("") : !mine ? grp("ESI+", P) + grp("ESI\u2212", N) : mine === "+" ? grp("ESI+", P) + grp("ESI\u2212", N) : grp("ESI\u2212", N) + grp("ESI+", P);
    const more = mine ? `<p class="sm"><a href="#" id="calcmore">${CALC_MORE ? I18N.t("calc.moreOnly", { pol: mine === "+" ? "ESI+" : "ESI\u2212" }) : I18N.t("calc.moreAlso", { pol: mine === "+" ? "ESI\u2212" : "ESI+" })}</a></p>` : "";
    out.innerHTML = `<p>${I18N.t("calc.neutral", { formula: fmtFormula(r.formula), mass: r.neutral.toFixed(4), nominal: r.nominal_neutral })}</p>
      <table><tr><th>${I18N.t("calc.col.adduct")}</th><th>${I18N.t("calc.col.exact")}</th>${hasObs ? `<th>${I18N.t("calc.col.ppm")}</th>` : ""}<th>${I18N.t("calc.col.nominal")}</th><th></th></tr>${body}</table>${more}`;
    const mo = out.querySelector("#calcmore"); if (mo) mo.onclick = e => { e.preventDefault(); CALC_MORE = !CALC_MORE; calcRun(); };
    out.querySelectorAll("button[data-m]").forEach(b => b.onclick = () => {
      Q("#calcdlg").close();
      openXic([...tabPanels()].reverse().find(q => q.type === "xic") || null, { formula: r.formula, adduct: b.dataset.a });
    });
  } catch (e) { sum.innerHTML = ""; out.innerHTML = `<p class="muted">${EH(e.message)}</p>`; }
}
Q("#calcin").oninput = calcRun;

// ---- discreet tooltip on every button and control (the program's own, not the browser's): it appears when the pointer rests on one for TIP_DELAY (1.25 s; it was 1.5 s until 7 Oct 2026) and goes away when the pointer leaves.
// The text is the element's title; the title is moved to data-tip on the first hover so the browser's own (earlier, fixed-style) tooltip does not also show up.
(function () {
  const tip = document.createElement("div"); tip.id = "qtip"; tip.hidden = true; document.body.appendChild(tip);
  const SEL = "[title], [data-tip], [data-tiph]";
  let timer = null, cur = null;
  const hide = () => { clearTimeout(timer); timer = null; tip.hidden = true; cur = null; };
  document.addEventListener("mouseover", e => {
    const el = /** @type {HTMLElement} */ (e.target), t = /** @type {HTMLElement | null} */ (el.closest && el.closest(SEL));
    if (t === cur) return;
    hide();
    if (!t || t.closest("dialog[open]")) return;
    if (t.hasAttribute("title")) { t.dataset.tip = t.getAttribute("title"); t.removeAttribute("title"); }
    const txt = t.dataset.tip, html = t.dataset.tiph; if (!txt && !html) return;
    cur = t;
    const delay = t.closest(".fl") ? 1000 : TIP_DELAY;
    timer = setTimeout(() => {
      if (html) tip.innerHTML = html; else tip.textContent = txt;
      tip.hidden = false;
      const r = t.getBoundingClientRect(), w = tip.offsetWidth, h = tip.offsetHeight;
      tip.style.left = Math.max(6, Math.min(innerWidth - w - 6, r.left + r.width / 2 - w / 2)) + "px";
      tip.style.top = (r.bottom + h + 10 < innerHeight ? r.bottom + 6 : Math.max(6, r.top - h - 6)) + "px";
    }, delay);
  });
  document.addEventListener("mouseout", e => { if (cur && !(e.relatedTarget && cur.contains(e.relatedTarget))) hide(); });
  ["mousedown", "keydown", "wheel"].forEach(ev => document.addEventListener(ev, hide, true));
  addEventListener("scroll", hide, true);
})();
