"use strict";
// Touch and pen (iPad + Apple Pencil, Android tablets). Classic script, loaded after explore.js, scroll.js, spettro.js and draw.js.
//
// The graphs (and the editor of the molecules, Ketcher) were written for the mouse: mousedown / mousemove / mouseup / wheel / dblclick / contextmenu / keys.
// A finger or a pen produce Pointer Events instead, and a drag on a canvas would only scroll the page. This layer turns a finger or a pen into the SAME
// events the mouse produces, so every tool keeps working (zoom, integrations, ruler, cursor, 3D rotation, drawing bonds), and adds the gestures of a touch screen:
//   one finger / pen  = the mouse: a drag acts as the chosen tool; a tap is a click; a double tap = double click
//   long press        = right click (the menu); the barrel button of a pen too
//   two fingers       = pinch zooms (as Ctrl + wheel). On a graph: sideways pans it (as Shift + drag), up / down scrolls the page. In the editor: both pan the drawing
//   pen hover         = the hover information (Apple Pencil on the newest iPads)
//   a pen in range    = palm rejection: touches are ignored for a moment
// The keys a touch screen lacks are not needed: the dot on the cursor line of a chromatogram moves it. Everything switches on only when a touch or a pen is really
// used, so a computer with a mouse does not change at all.
const TOUCH = (() => {
  const LONG = 550, MOVE = 8, DTAP = 350, PALM = 900;
  let penAt = 0, seen = false, lastTouch = 0, lastUp = 0, lpFired = false;
  const dist = (a, b) => Math.hypot(a.x - b.x, a.y - b.y), mid = (a, b) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
  // events are built in the realm of the element that receives them (the editor lives in an iframe)
  const win = el => (el.ownerDocument && el.ownerDocument.defaultView) || window;
  const mouse = (type, el, x, y, o = {}) => el.dispatchEvent(new (win(el).MouseEvent)(type, { bubbles: true, cancelable: true, view: win(el), clientX: x, clientY: y, button: o.button || 0, buttons: type === "mouseup" ? 0 : o.buttons ?? 1, shiftKey: !!o.shift, ctrlKey: !!o.ctrl, detail: o.detail || 1 }));
  const wheel = (el, x, y, o) => el.dispatchEvent(new (win(el).WheelEvent)("wheel", { bubbles: true, cancelable: true, view: win(el), clientX: x, clientY: y, deltaX: o.dx || 0, deltaY: o.dy || 0, ctrlKey: !!o.ctrl, deltaMode: 0 }));

  // the dot of the cursor line is for fingers and pens only: the last pointer used decides (a laptop with a touch screen shows it after a touch, hides it at the first mouse move)
  const kind = e => { const t = e.pointerType === "mouse" ? "ptr-mouse" : e.pointerType === "touch" || e.pointerType === "pen" ? "ptr-touch" : null; if (!t) return; const c = document.documentElement.classList; if (!c.contains(t)) { c.add(t); c.remove(t === "ptr-mouse" ? "ptr-touch" : "ptr-mouse"); } };
  addEventListener("pointerdown", kind, true); addEventListener("pointermove", kind, true);

  function touchMode() { if (seen) return; seen = true; document.documentElement.classList.add("touch"); }

  // ============================================================ the engine: one per document (the page, the editor)
  // hooks.surface(target) -> { el, ctx } | null   the element that takes the mouse events, and what the page knows about it
  // hooks.down(ctx)                               a finger / pen went down on it (bring the panel to the front, ...)
  // hooks.hit                                     true: the events go to the element under the finger, not to the surface itself
  // hooks.panDrag                                 true: a sideways pan is a Shift + drag and up / down scrolls the page (the graphs); false: both are wheel events (the editor)
  function engine(doc, hooks) {
    const pts = new Map();                                       // active pointers on a surface: id -> {x, y}
    let G = null, lastTap = { t: 0, x: 0, y: 0, el: null };
    const surfaceOf = e => (e.target && e.target.closest ? hooks.surface(e.target) : null);
    // the element that gets the mouse event: the graphs listen on the canvas itself; the editor listens on elements INSIDE its drawing area, so the event goes to
    // whatever is under the finger (an event only bubbles up, never down)
    const tgt = (S, x, y) => (hooks.hit ? doc.elementFromPoint(x, y) || S.el : S.el);

    function down(e, S) {
      if (e.pointerType === "pen") penAt = Date.now();
      else if (Date.now() - penAt < PALM) { e.preventDefault(); return; }               // palm rejection: the pen is near
      touchMode(); e.preventDefault(); lastTouch = Date.now();
      try { S.el.setPointerCapture(e.pointerId); } catch (_) { /* not capturable: the events still arrive while the pointer is over it */ }
      pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (pts.size === 2) return startTwo(S);
      if (pts.size > 2) return;
      hooks.down && hooks.down(S.ctx);
      mouse("mousemove", tgt(S, e.clientX, e.clientY), e.clientX, e.clientY, { buttons: 0 });                    // the hover information under the finger
      G = { S, id: e.pointerId, mode: "wait", x0: e.clientX, y0: e.clientY };
      if ((e.buttons & 2) === 2) { G.mode = "done"; mouse("contextmenu", tgt(S, e.clientX, e.clientY), e.clientX, e.clientY, { button: 2, buttons: 2 }); return; }      // the barrel button of a pen
      G.timer = setTimeout(() => { if (G && G.mode === "wait") { G.mode = "long"; navigator.vibrate && navigator.vibrate(15); mouse("contextmenu", tgt(S, G.x0, G.y0), G.x0, G.y0, { button: 2, buttons: 2 }); } }, LONG);   // long press = right click
    }
    function move(e) {
      const q = pts.get(e.pointerId); if (!q) return;
      q.x = e.clientX; q.y = e.clientY;
      if (G && G.two) { if (!G.raf) { const g = G; g.raf = requestAnimationFrame(() => { g.raf = 0; if (G === g) moveTwo(); }); } return; }      // once per frame: both fingers have reported
      if (!G || e.pointerId !== G.id) return;
      if (G.mode === "wait" && Math.hypot(e.clientX - G.x0, e.clientY - G.y0) > MOVE) {
        clearTimeout(G.timer); G.mode = "drag";                                          // the drag starts where the finger first touched
        G.t = tgt(G.S, G.x0, G.y0); mouse("mousedown", G.t, G.x0, G.y0); mouse("mousemove", G.t, G.x0, G.y0);
      }
      if (G.mode === "drag") mouse("mousemove", tgt(G.S, e.clientX, e.clientY), e.clientX, e.clientY);
    }
    function up(e) {
      if (!pts.has(e.pointerId)) return;
      lastUp = Date.now(); pts.delete(e.pointerId);
      if (G && G.two) { if (pts.size < 2) endTwo(); return; }
      if (!G || e.pointerId !== G.id) return;
      const g = G; G = null; clearTimeout(g.timer); const el = g.S.el;
      if (g.mode === "drag") { mouse("mouseup", tgt(g.S, e.clientX, e.clientY), e.clientX, e.clientY); return; }
      if (g.mode !== "wait") return;                                                     // a long press already opened the menu
      const dbl = Date.now() - lastTap.t < DTAP && lastTap.el === el && Math.hypot(e.clientX - lastTap.x, e.clientY - lastTap.y) < 28;
      const t = tgt(g.S, g.x0, g.y0);
      mouse("mousedown", t, g.x0, g.y0); mouse("mouseup", t, g.x0, g.y0);                // a tap = a click
      if (dbl) { mouse("dblclick", t, g.x0, g.y0, { detail: 2 }); lastTap = { t: 0, x: 0, y: 0, el: null }; }
      else lastTap = { t: Date.now(), x: g.x0, y: g.y0, el };
    }
    function cancel(e) {
      if (!pts.has(e.pointerId)) return;
      pts.delete(e.pointerId);
      if (G) { clearTimeout(G.timer); if (G.mode === "drag" && !G.two) mouse("mouseup", tgt(G.S, e.clientX, e.clientY), e.clientX, e.clientY); if (G.two && G.panning) endTwo(); if (!pts.size) G = null; }
    }
    // two fingers: pinch = zoom, sideways = pan, up / down = scroll (graphs) or pan (editor)
    function startTwo(S) {
      if (G) { clearTimeout(G.timer); if (G.mode === "drag") mouse("mouseup", tgt(S, G.x0, G.y0), G.x0, G.y0); }
      const [a, b] = [...pts.values()];
      G = { S, two: true, kind: null, d: dist(a, b), m: mid(a, b), d0: dist(a, b), m0: mid(a, b), panning: false };
    }
    function moveTwo() {
      if (pts.size < 2) return;
      const [a, b] = [...pts.values()], d = dist(a, b), m = mid(a, b), g = G, el = tgt(g.S, m.x, m.y);
      if (!g.kind) {                                                                     // decide once the fingers have moved enough
        const dd = Math.abs(d - g.d0), dx = m.x - g.m0.x, dy = m.y - g.m0.y;
        if (dd < 10 && Math.hypot(dx, dy) < 10) return;
        g.kind = dd >= 10 && dd > Math.hypot(dx, dy) * 0.8 ? "pinch" : hooks.panDrag ? (Math.abs(dx) > Math.abs(dy) ? "pan" : "scroll") : "pan";
        if (g.kind === "pan" && hooks.panDrag) { g.panning = true; mouse("mousedown", el, g.m.x, g.m.y, { shift: true }); }
      }
      if (g.kind === "pinch") {                                                          // the same wheel event as Ctrl + wheel (or the trackpad pinch)
        if (d > 0 && g.d > 0 && Math.abs(d - g.d) > 0.5) wheel(el, m.x, m.y, { ctrl: true, dy: Math.log(g.d / d) / 0.004 });
      } else if (g.kind === "pan") { if (hooks.panDrag) mouse("mousemove", el, m.x, m.y, { shift: true }); else wheel(el, m.x, m.y, { dx: g.m.x - m.x, dy: g.m.y - m.y }); }
      else window.scrollBy(0, g.m.y - m.y);
      g.d = d; g.m = m;
    }
    function endTwo() {
      const g = G; G = null;
      if (g && g.panning) mouse("mouseup", tgt(g.S, g.m.x, g.m.y), g.m.x, g.m.y, { shift: true });
      pts.clear();                                                                       // the remaining finger does nothing until it is lifted
    }
    // listeners on the document (capture): they see the events of the surfaces that will exist later too
    doc.addEventListener("pointerdown", e => { if (e.pointerType === "mouse") return; const S = surfaceOf(e); if (S) down(e, S); }, true);
    doc.addEventListener("pointermove", e => {
      if (e.pointerType === "mouse") return;
      if (pts.has(e.pointerId)) { move(e); return; }
      if (e.pointerType === "pen" && !pts.size && !e.buttons) { penAt = Date.now(); const S = surfaceOf(e); if (S) mouse("mousemove", tgt(S, e.clientX, e.clientY), e.clientX, e.clientY, { buttons: 0 }); }      // the pen hovers
    }, true);
    doc.addEventListener("pointerup", e => { if (e.pointerType !== "mouse") up(e); }, true);
    doc.addEventListener("pointercancel", e => { if (e.pointerType !== "mouse") cancel(e); }, true);
    // the browser also sends its own click / double click / menu after a touch: the program already got the mouse events of this layer, so those are not wanted
    // (a click would close the menu a long press has just opened, a double click would run twice, Android would show its own menu)
    ["click", "dblclick"].forEach(t => doc.addEventListener(t, e => { if (e.isTrusted && Date.now() - lastUp < 700 && surfaceOf(e)) { e.stopPropagation(); e.preventDefault(); } }, true));
    // ... and its compatibility mouse events (mousedown / mouseup / mousemove after a tap) too: the editor would count them as a second click (two bonds from one tap)
    ["mousedown", "mouseup", "mousemove"].forEach(t => doc.addEventListener(t, e => { if (e.isTrusted && (pts.size || Date.now() - lastUp < 700) && surfaceOf(e)) { e.stopPropagation(); e.preventDefault(); } }, true));
    doc.addEventListener("contextmenu", e => { if (e.isTrusted && Date.now() - lastTouch < 1500 && surfaceOf(e)) { e.stopPropagation(); e.preventDefault(); } }, true);
    return { pts: () => pts.size, state: () => G && (G.kind || G.mode) };
  }

  // ============================================================ the graphs of the page
  const panelOf = cv => E.panels.find(q => q.cv === cv);
  const page = engine(document, {
    panDrag: true,
    surface: t => { const cv = t.closest(".pnl canvas"); if (!cv) return null; const p = panelOf(cv); return p && (p._a || p.type === "spec") ? { el: cv, ctx: p } : null; },
    down: p => { front(p.el); setActive(p); },
  });
  // a long press on a file of the list = right click (rename, colour, remove)
  let lp = null;
  document.addEventListener("pointerdown", e => {
    if (e.pointerType === "mouse") return; const row = e.target.closest && e.target.closest("#flst .fl"); if (!row) return;
    const x0 = e.clientX, y0 = e.clientY; clearTimeout(lp);
    lp = setTimeout(() => { lp = null; lpFired = true; setTimeout(() => { lpFired = false; }, 8000); row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, view: window, clientX: x0, clientY: y0, button: 2 })); }, LONG);
    const stop = ev => { if (ev.type === "pointermove" && Math.hypot(ev.clientX - x0, ev.clientY - y0) < MOVE) return; clearTimeout(lp); lp = null; ["pointermove", "pointerup", "pointercancel"].forEach(t => removeEventListener(t, stop, true)); };
    ["pointermove", "pointerup", "pointercancel"].forEach(t => addEventListener(t, stop, true));
  }, true);
  addEventListener("pointerup", e => { if (e.pointerType !== "mouse") lastUp = Date.now(); }, true);
  document.addEventListener("click", e => { if (e.isTrusted && lpFired && Date.now() - lastUp < 700 && e.target.closest && e.target.closest("#flst .fl")) { lpFired = false; e.stopPropagation(); e.preventDefault(); } }, true);
  // the first touch anywhere (a button, the list of files) also switches the touch mode on
  addEventListener("pointerdown", e => { if (e.pointerType !== "mouse") { touchMode(); lastTouch = Date.now(); } }, true);

  // ============================================================ the editor of the molecules (Ketcher, in an iframe of the same origin)
  const CANVAS = '[class*="StructEditor-module_canvas"]';
  let ketcherDoc = null;
  function bridgeKetcher() {
    const fr = document.getElementById("kframe"); let d = null;
    try { d = fr && fr.contentDocument; } catch (_) { return; }
    if (!d || d === ketcherDoc || !d.querySelector(CANVAS)) return;
    ketcherDoc = d;
    const st = d.createElement("style");                                                  // the drawing area takes the finger; no callout / selection on a long press
    st.textContent = `${CANVAS}{touch-action:none;-webkit-touch-callout:none;-webkit-user-select:none;user-select:none}`; d.head.appendChild(st);
    engine(d, { panDrag: false, hit: true, surface: t => { const c = t.closest(CANVAS); return c ? { el: c, ctx: null } : null; } });
  }
  setInterval(bridgeKetcher, 1000);

  // ============================================================ a grip to change the height of a panel (the CSS handle does not work with a finger)
  function grip(p) {
    if (p.el.querySelector(".rzg")) return;
    const g = document.createElement("div"); g.className = "rzg"; g.title = I18N.t("touch.grip.title"); g.innerHTML = "<i></i>"; p.el.appendChild(g);
    let st = null;
    g.addEventListener("pointerdown", e => { e.preventDefault(); e.stopPropagation(); g.setPointerCapture(e.pointerId); st = { y: e.clientY, h: p.h }; touchMode(); });
    g.addEventListener("pointermove", e => { if (!st) return; p.h = Math.max(190, Math.round(st.h + e.clientY - st.y)); apply(p); relayout(); fitHost(); });
    const end = () => { if (st) { st = null; uiSave(); } };
    g.addEventListener("pointerup", end); g.addEventListener("pointercancel", end);
  }
  const scan = () => E.panels.forEach(p => { if (p.el) grip(p); });
  new MutationObserver(scan).observe(Q("#dpanels"), { childList: true }); scan();
  return { scan, get seen() { return seen; }, _state: () => ({ seen, pts: page.pts(), gesture: page.state(), ketcher: !!ketcherDoc }) };
})();
window.TOUCH = TOUCH;
