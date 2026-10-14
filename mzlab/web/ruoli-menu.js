// @ts-check
"use strict";
// Lists opened by the boxes Blank · t0 · Infusion and the listeners of the strip.
Object.assign(RUOLI, (() => {
  const { t, eh, BOX_ROLE, val, roleOf, label, setRole, paint } = RUOLI;
  // ---------------------------------------------------------------- the boxes and their lists
  function closeMenu(/** @type {Element} */ el, /** @type {boolean} */ back) {
    const m = el.querySelector(".rl-menu");
    if (!m) return;
    const b = /** @type {HTMLElement|null} */ (el.querySelector(`[data-box="${m.getAttribute("data-for")}"]`));
    m.remove();
    if (b) { b.setAttribute("aria-expanded", "false"); if (back) b.focus(); }
  }
  function openMenu(/** @type {Element} */ el, /** @type {Voce[]} */ F, /** @type {HTMLElement} */ btn, /** @type {() => void} */ redraw) {
    closeMenu(el, false);
    const b = btn.dataset.box || "", r = BOX_ROLE[b], m = document.createElement("div");
    m.className = "rl-menu"; m.setAttribute("role", "menu"); m.setAttribute("data-for", b); m.setAttribute("aria-label", t("load.box.menu", { role: t(`load.box.${b}`) }));
    const items = F.map((f, i) => f.use ? `<button type="button" role="menuitemcheckbox" class="sec sm" data-pick="${i}" aria-checked="${roleOf(f) === r}">${eh(f.name)} <span class="muted">${eh(label(val(f)))}</span></button>` : "").join("")
      + `<button type="button" role="menuitem" class="sec sm" data-add="1">${eh(t("load.box.add"))}</button>`;
    m.innerHTML = items;
    btn.setAttribute("aria-expanded", "true");
    btn.after(m);
    const all = () => /** @type {HTMLElement[]} */ ([...m.querySelectorAll("button")]);
    m.addEventListener("click", e => {
      const x = /** @type {HTMLElement} */ (e.target).closest("button"); if (!x) return;
      if (x.dataset.add) { closeMenu(el, true); const p = /** @type {HTMLElement|null} */ (document.getElementById("pick")); if (p) p.click(); return; }
      const f = F[+(x.dataset.pick || 0)];
      f.ed = 1; setRole(f, roleOf(f) === r ? "sample" : b === "blank" ? "blank" : r);
      closeMenu(el, false); redraw(); refocusBox(el, b);
    });
    m.addEventListener("keydown", e => {
      const a = all(), k = a.indexOf(/** @type {HTMLElement} */ (document.activeElement));
      if (e.key === "Escape") { e.preventDefault(); closeMenu(el, true); }
      else if (e.key === "ArrowDown") { e.preventDefault(); a[(k + 1) % a.length].focus(); }
      else if (e.key === "ArrowUp") { e.preventDefault(); a[(k - 1 + a.length) % a.length].focus(); }
    });
    const first = all()[0]; if (first) first.focus();
  }
  function refocusBox(/** @type {Element} */ el, /** @type {string} */ b) { const x = /** @type {HTMLElement|null} */ (document.querySelector(`#flist [data-box="${b}"]`)); if (x) x.focus(); }

  /** listeners of the strip; `redraw` sorts and draws the table again */
  function bind(/** @type {Element} */ el, /** @type {Voce[]} */ F, /** @type {() => void} */ redraw) {
    el.querySelectorAll("[data-box]").forEach(x => {
      const b = /** @type {HTMLElement} */ (x);
      b.onclick = () => { if (b.getAttribute("aria-expanded") === "true") closeMenu(el, true); else openMenu(el, F, b, redraw); };
      b.onkeydown = e => {
        if (e.key === "Delete" || e.key === "Backspace") {
          e.preventDefault();
          F.filter(f => roleOf(f) === BOX_ROLE[b.dataset.box || ""]).forEach(f => { f.ed = 1; setRole(f, "sample"); });
          redraw(); refocusBox(el, b.dataset.box || "");
        } else if (e.key === "ArrowDown" && b.getAttribute("aria-expanded") !== "true") { e.preventDefault(); openMenu(el, F, b, redraw); }
      };
    });
    const c = /** @type {HTMLElement|null} */ (el.querySelector("#rl-conf"));
    if (c) c.onclick = () => { F.forEach(f => { if (f.use && f.gT && f.time != null) f.gT = false; }); redraw(); refocusBox(el, "blank"); };
    paint(el, F);
  }

  return { bind };
})());
