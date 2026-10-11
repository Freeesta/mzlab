/* mzLab - Teoria: reading aids for narrow screens and touch (loaded by teoria.js after the layout).
   1. chapter menu as a bottom sheet with one fixed button (icon + current chapter) and a thin progress bar;
   2. last reading position per chapter (localStorage, optional);
   3. figures open in a full-screen viewer (pinch, double tap, drag, wheel, keys).
   Classic script, no network, no library: it works from file:// too. Only presentation: it never changes the text of a chapter. */
"use strict";
(function () {
  const $ = (q, el = document) => el.querySelector(q);
  const side = $("#side"), menu = $("#menu");
  if (!side || !menu) return;
  const here = location.pathname.split("/").pop() || "index.html";
  const cur = (typeof CHAPTERS !== "undefined" ? CHAPTERS : []).find(c => c[0] === here);
  const KEY = "qqq.teoria.pos"; // {last: file, pos: {file: ratio}}; reading position only, never sent anywhere
  const store = {
    get() { try { return JSON.parse(localStorage.getItem(KEY)) || { pos: {} }; } catch (_) { return { pos: {} }; } },
    set(v) { try { localStorage.setItem(KEY, JSON.stringify(v)); } catch (_) { /* storage blocked: no memory of the position */ } },
  };
  const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.hasAttribute("data-a11y-reduce");
  const narrow = matchMedia("(max-width:980px)");

  // ------------------------------------------------------------------ chapter menu as a sheet
  const bar = document.createElement("div"); bar.id = "cbar";
  const label = cur ? (/^[0-9]+$/.test(cur[1]) && cur[1] !== "0" ? cur[1] + " · " : "") + cur[2] : "Indice";
  menu.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/></svg><span class="mt"></span>`;
  $(".mt", menu).textContent = label;
  menu.setAttribute("aria-label", "Indice dei capitoli. Capitolo corrente: " + label);
  menu.setAttribute("aria-haspopup", "dialog");
  const prog = document.createElement("i"); prog.id = "prog"; prog.setAttribute("aria-hidden", "true");
  bar.append(menu, prog);
  document.body.prepend(bar);

  const bg = document.createElement("div"); bg.id = "sheetbg"; bg.hidden = true; document.body.appendChild(bg);
  const head = document.createElement("div"); head.className = "sheethd";
  head.innerHTML = `<span class="grab" aria-hidden="true"></span><button type="button" class="sheetx" aria-label="Chiudi l'indice">Chiudi</button>`;
  side.prepend(head);
  // "resume" entry: the chapter read last (if it is another one)
  const st0 = store.get(), lastC = (typeof CHAPTERS !== "undefined" ? CHAPTERS : []).find(c => c[0] === st0.last);
  if (lastC && lastC[0] !== here && st0.pos[lastC[0]] > 0.03) {
    const r = document.createElement("a"); r.className = "resume"; r.href = lastC[0] + "#riprendi";
    r.textContent = "Riprendi: " + (/^[0-9]+$/.test(lastC[1]) && lastC[1] !== "0" ? lastC[1] + " · " : "") + lastC[2];
    head.after(r);
  }

  if (window.self === window.top && location.protocol !== "file:") {   // standalone page: the way back to the program is in the sheet too
    const g = document.createElement("a"); g.className = "resume"; g.href = "../../#/dati"; g.textContent = "Apri il programma"; g.style.marginTop = "14px"; side.appendChild(g);
  }

  let opener = null;
  const isOpen = () => side.classList.contains("open");
  function sync() {
    const o = isOpen() && narrow.matches;
    bg.hidden = !o;
    if (o) { side.setAttribute("role", "dialog"); side.setAttribute("aria-modal", "true"); } else { side.removeAttribute("role"); side.removeAttribute("aria-modal"); }
    menu.setAttribute("aria-expanded", String(isOpen()));
  }
  function open() {
    opener = document.activeElement; side.classList.add("open"); sync();
    const on = $("#side li a.on", side); if (on) on.scrollIntoView({ block: "center" });
    (on || $(".sheetx", side)).focus({ preventScroll: true });
  }
  function close(back = true) {
    side.classList.remove("open"); sync(); side.style.transform = "";
    if (back && opener && opener.focus) opener.focus({ preventScroll: true });
  }
  // teoria.js already toggles on click of #menu; this takes over with focus handling
  menu.onclick = () => (isOpen() ? close() : open());
  bg.onclick = () => close();
  $(".sheetx", side).onclick = () => close();
  side.addEventListener("click", e => { if (e.target.closest("a")) { side.style.transform = ""; setTimeout(sync); } });   // teoria.js closes the sheet on a link: the backdrop follows
  document.addEventListener("keydown", e => {
    if (!isOpen() || !narrow.matches) return;
    if (e.key === "Escape") { e.preventDefault(); close(); return; }
    if (e.key === "Tab") {   // focus trap inside the sheet
      const f = [...side.querySelectorAll("a[href],button")].filter(x => x.offsetParent !== null);
      if (!f.length) return;
      const a = f[0], z = f[f.length - 1];
      if (e.shiftKey && document.activeElement === a) { e.preventDefault(); z.focus(); } else if (!e.shiftKey && document.activeElement === z) { e.preventDefault(); a.focus(); }
    }
  });
  narrow.addEventListener("change", () => { if (!narrow.matches) close(false); sync(); });
  // swipe down on the header of the sheet (or on the list scrolled to the top) closes it
  let sy = null, dy = 0;
  side.addEventListener("touchstart", e => { sy = ((side.scrollTop <= 0 || e.target.closest(".sheethd")) && e.touches.length === 1) ? e.touches[0].clientY : null; dy = 0; }, { passive: true });
  side.addEventListener("touchmove", e => {
    if (sy === null) return; dy = e.touches[0].clientY - sy;
    if (dy > 0 && narrow.matches) side.style.transform = `translateY(${dy}px)`;
  }, { passive: true });
  side.addEventListener("touchend", () => { if (sy !== null && dy > 90) close(); else side.style.transform = ""; sy = null; });

  // ------------------------------------------------------------------ progress and reading position
  let tick = 0, saved = 0;
  function onScroll() {
    if (tick) return; tick = requestAnimationFrame(() => {
      tick = 0;
      const h = document.documentElement.scrollHeight - innerHeight, r = h > 0 ? Math.min(1, Math.max(0, scrollY / h)) : 0;
      prog.style.transform = `scaleX(${r})`;
      const now = Date.now(); if (now - saved > 800) { saved = now; remember(r); }
    });
  }
  function remember(r) {
    if (!cur || /^pratica/.test(here)) return;
    const s = store.get(); s.last = here; s.pos = s.pos || {}; s.pos[here] = Math.round(r * 1000) / 1000; store.set(s);
  }
  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("pagehide", () => { const h = document.documentElement.scrollHeight - innerHeight; remember(h > 0 ? scrollY / h : 0); });
  onScroll();
  // back to the last position: only when the chapter is opened from outside the Teoria (the program, a bookmark) or from the "resume" entry, never by prev/next
  const fromInside = /\/teoria\//.test(document.referrer) && !/#riprendi/.test(location.hash);
  if ((!location.hash || location.hash === "#riprendi") && !fromInside && cur) {
    const r = store.get().pos[here];
    if (r > 0.03 && r < 0.97) addEventListener("load", () => {
      const h = document.documentElement.scrollHeight - innerHeight; if (h > 0) scrollTo({ top: r * h, behavior: "instant" });
    }, { once: true });
  }

  // ------------------------------------------------------------------ figure viewer
  const figs = [...document.querySelectorAll("main figure")].filter(f => {
    const m = $("svg,img", f);
    return m && !f.closest(".sim") && !$("canvas,input,select,textarea,button,a svg", f) && !m.closest("a");
  });
  let box = null, stage, node, baseH = 0, capEl, scale = 1, tx = 0, ty = 0, lastFocus = null, savedY = 0;
  const MAX = 8;
  function build() {
    box = document.createElement("div"); box.id = "lb"; box.hidden = true;
    box.setAttribute("role", "dialog"); box.setAttribute("aria-modal", "true"); box.setAttribute("aria-label", "Figura ingrandita");
    box.innerHTML = `<div class="lbstage"></div><p class="lbcap" id="lbcap"></p>
      <div class="lbtools" role="group" aria-label="Zoom della figura">
        <button type="button" data-a="out" aria-label="Riduci">&minus;</button><button type="button" data-a="fit" aria-label="Adatta alla schermata">1&times;</button><button type="button" data-a="in" aria-label="Ingrandisci">+</button>
        <button type="button" data-a="x" class="lbx" aria-label="Chiudi la figura">&times;</button></div>`;
    box.setAttribute("aria-describedby", "lbcap");
    document.body.appendChild(box);
    stage = $(".lbstage", box); capEl = $("#lbcap", box);
    box.addEventListener("click", e => {
      const b = e.target.closest("button[data-a]");
      if (b) { const a = b.dataset.a; if (a === "x") closeFig(); else if (a === "fit") reset(); else zoomAt(a === "in" ? 1.5 : 1 / 1.5, innerWidth / 2, innerHeight / 2); return; }
      if (!moved && (e.target === stage || e.target === box)) closeFig();
    });
    document.addEventListener("keydown", e => {   // on the document: a tap on the picture moves the focus away from the buttons
      if (box.hidden) return;
      if (e.key === "Escape") { e.preventDefault(); closeFig(); }
      else if (e.key === "+" || e.key === "=") { e.preventDefault(); zoomAt(1.25, innerWidth / 2, innerHeight / 2); }
      else if (e.key === "-" || e.key === "_") { e.preventDefault(); zoomAt(0.8, innerWidth / 2, innerHeight / 2); }
      else if (e.key === "0") { e.preventDefault(); reset(); }
      else if (e.key.startsWith("Arrow")) { e.preventDefault(); const d = 60; if (e.key === "ArrowLeft") tx += d; else if (e.key === "ArrowRight") tx -= d; else if (e.key === "ArrowUp") ty += d; else ty -= d; apply(); }
      else if (e.key === "Tab") {
        const f = [...box.querySelectorAll("button")]; const a = f[0], z = f[f.length - 1];
        if (e.shiftKey && document.activeElement === a) { e.preventDefault(); z.focus(); } else if (!e.shiftKey && document.activeElement === z) { e.preventDefault(); a.focus(); }
      }
    });
    box.addEventListener("wheel", e => { e.preventDefault(); zoomAt(Math.exp(-e.deltaY * 0.0015), e.clientX, e.clientY); }, { passive: false });
    pointers();
  }
  function apply() {
    const w = stage.clientWidth, h = stage.clientHeight, cw = w * scale, ch = baseH * scale;
    // keep the picture at least partly in view: centred when smaller than the screen
    tx = cw <= w ? (w - cw) / 2 : Math.min(0, Math.max(w - cw, tx));
    ty = ch <= h ? (h - ch) / 2 : Math.min(0, Math.max(h - ch, ty));
    node.style.transform = `translate(${tx}px,${ty}px) scale(${scale})`;
  }
  function reset() { scale = 1; tx = ty = 0; apply(); }
  addEventListener("resize", () => { if (box && !box.hidden) { node.style.transform = ""; baseH = node.getBoundingClientRect().height; apply(); } });
  function zoomAt(k, cx, cy) {
    const r = stage.getBoundingClientRect(), ns = Math.min(MAX, Math.max(1, scale * k)), f = ns / scale;
    const px = cx - r.left, py = cy - r.top;
    tx = px - (px - tx) * f; ty = py - (py - ty) * f; scale = ns; apply();
  }
  let moved = false;
  function pointers() {
    const P = new Map(); let d0 = 0, s0 = 1, lastTap = 0, tapX = 0, tapY = 0;
    stage.addEventListener("pointerdown", e => {
      stage.setPointerCapture(e.pointerId); P.set(e.pointerId, [e.clientX, e.clientY]); moved = false;
      if (P.size === 2) { const [a, b] = [...P.values()]; d0 = Math.hypot(a[0] - b[0], a[1] - b[1]); s0 = scale; }
    });
    stage.addEventListener("pointermove", e => {
      if (!P.has(e.pointerId)) return;
      const o = P.get(e.pointerId), x = e.clientX, y = e.clientY;
      if (P.size === 2) {
        P.set(e.pointerId, [x, y]); const [a, b] = [...P.values()], d = Math.hypot(a[0] - b[0], a[1] - b[1]);
        if (d0 > 0) zoomAt(Math.min(MAX, Math.max(1, s0 * d / d0)) / scale, (a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
        moved = true; return;
      }
      if (Math.abs(x - o[0]) + Math.abs(y - o[1]) > 3) moved = true;
      tx += x - o[0]; ty += y - o[1]; P.set(e.pointerId, [x, y]); apply();
    });
    const up = e => {
      if (!P.has(e.pointerId)) return; P.delete(e.pointerId);
      if (e.type === "pointerup" && !moved && P.size === 0) {   // double tap / double click: zoom in at the point, or back to fit
        const t = Date.now();
        if (t - lastTap < 320 && Math.hypot(e.clientX - tapX, e.clientY - tapY) < 30) { scale > 1.05 ? reset() : zoomAt(2.5, e.clientX, e.clientY); lastTap = 0; }
        else { lastTap = t; tapX = e.clientX; tapY = e.clientY; }
      }
    };
    stage.addEventListener("pointerup", up); stage.addEventListener("pointercancel", up);
  }
  function openFig(fig) {
    if (!box) build();
    const src = $("svg,img", fig); lastFocus = document.activeElement; savedY = scrollY;
    node = src.cloneNode(true); node.removeAttribute("style"); node.removeAttribute("width"); node.removeAttribute("height");
    node.classList.add("lbnode");
    if (node.tagName === "IMG") {   // the large picture is fetched only now
      node.removeAttribute("srcset"); node.removeAttribute("sizes"); node.loading = "eager"; node.src = src.dataset.full || src.currentSrc || src.src; node.decoding = "async";
    }
    stage.replaceChildren(node);
    const cap = $("figcaption", fig);
    capEl.textContent = cap ? cap.textContent : "";
    const nm = src.getAttribute("aria-label") || src.getAttribute("alt") || (cap && cap.textContent) || "Figura";
    box.setAttribute("aria-label", "Figura ingrandita: " + nm);
    box.hidden = false; document.documentElement.classList.add("lbon");
    scale = 1; tx = ty = 0; node.style.transformOrigin = "0 0";
    const go = () => { node.style.transform = ""; baseH = node.getBoundingClientRect().height; apply(); }; node.tagName === "IMG" && !node.complete ? node.addEventListener("load", go, { once: true }) : go();
    $(".lbx", box).focus({ preventScroll: true });
  }
  function closeFig() {
    box.hidden = true; document.documentElement.classList.remove("lbon"); stage.replaceChildren();
    scrollTo({ top: savedY, behavior: "instant" });
    if (lastFocus && lastFocus.focus) lastFocus.focus({ preventScroll: true });
  }
  figs.forEach(f => {
    const m = $("svg,img", f), name = m.getAttribute("aria-label") || m.getAttribute("alt") || "figura";
    const b = document.createElement("button"); b.type = "button"; b.className = "fzoom";
    b.setAttribute("aria-label", "Apri a schermo intero: " + name);
    b.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><path d="M12 3h5v5M8 17H3v-5M17 3l-5.5 5.5M3 17l5.5-5.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/></svg>`;
    f.classList.add("zoomable"); f.prepend(b);
    b.onclick = () => openFig(f);
    m.addEventListener("click", () => openFig(f));
  });
  // the same tools are reachable from the program: Esc in the iframe parent is not needed, the viewer lives in this document
})();
