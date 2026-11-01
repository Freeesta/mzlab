// @ts-check
// «Group by ▾» of the side list header: the menu (None, the ranked variables of the names with a preview, Then by…, Token position…, Contains…, Custom) and the two
// dialogs of the manual rules. Lazy (carica("gruppi-per.js") from lista-file.js); the engine's answer and the writer are in gruppi.js, nothing here splits a name.
window.GRUPPIPER = (() => {
  const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  // the button the menu hangs from: the side list's, or another one given to open() (the table's «Group by»)
  let anchor = "sb-by";
  const btn = () => /** @type {HTMLButtonElement} */ (document.getElementById(anchor) || document.getElementById("sb-by"));
  /** @type {HTMLElement | null} */ let box = null;
  let view = "main";
  const close = (/** @type {boolean} */ focus = true) => {
    box?.remove(); box = null; view = "main"; btn().setAttribute("aria-expanded", "false");
    document.removeEventListener("pointerdown", away, true);
    if (focus) btn().focus();
  };
  const away = (/** @type {Event} */ e) => { if (box && !box.contains(/** @type {Node} */ (e.target)) && !btn().contains(/** @type {Node} */ (e.target))) close(false); };
  /** «MQ 20 · TW 9», cut at three values and «…». @param {any} v */
  const preview = v => v.valori.slice(0, 3).map((/** @type {any} */ x) => x.etichetta + " " + x.file.length).join(" · ") + (v.valori.length > 3 ? " …" : "");
  /** @typedef {{ label: string, sub?: string, checked?: boolean, disabled?: boolean, fn: () => void }} Item */
  function items(/** @type {any} */ ans) {
    const ub = E.ugBy, vs = ans ? ans.variabili : [], list = /** @type {Item[]} */ ([]);
    const name = (/** @type {any} */ v) => t(`groups.var.${v.id}`);
    if (view === "then") {
      list.push({ label: "‹ " + t("groups.back"), fn: () => draw("main") });
      vs.filter((/** @type {any} */ v) => v.id !== ub?.by).forEach((/** @type {any} */ v) => list.push({ label: name(v), sub: preview(v), checked: ub?.then === v.id, fn: () => pick({ by: ub.by, then: ub.then === v.id ? undefined : v.id, manuale: ub.manuale }) }));
      return list;
    }
    if (anchor === "sb-by") {
      list.push({ label: t("groups.type"), checked: !ub && !GRUPPI.has(), fn: () => pick(null) });
      list.push({ label: t("groups.none"), checked: ub?.by === "flat", fn: () => pick({ by: "flat" }) });
    } else list.push({ label: t("groups.none"), checked: !ub && !GRUPPI.has(), fn: () => pick(null) });
    vs.forEach((/** @type {any} */ v) => list.push({ label: name(v), sub: preview(v), checked: ub?.by === v.id, fn: () => pick({ by: v.id }) }));
    if (ub && ub.by !== "custom" && ub.by !== "flat" && ub.by !== "manuale" && vs.length > 1) list.push({ label: t("groups.then"), sub: ub.then ? t(`groups.var.${ub.then}`) : "", fn: () => draw("then") });
    list.push({ label: t("groups.pos"), checked: ub?.by === "manuale" && !!ub.manuale?.pos, fn: () => { close(false); dialogPos(); } });
    list.push({ label: t("groups.has"), checked: ub?.by === "manuale" && !!ub.manuale?.has, fn: () => { close(false); dialogHas(); } });
    list.push({ label: t("groups.custom"), checked: ub?.by === "custom", disabled: true, fn: () => { /* shown only: set by a manual edit */ } });
    return list;
  }
  async function pick(/** @type {any} */ spec) { close(); if (!(await GRUPPI.by(spec))) directNote("groups.error"); }
  /** @param {string} v */
  async function draw(v) {
    view = v;
    const ans = await GRUPPI.vars();
    if (!box) return;
    box.replaceChildren();
    if (ans && ans.comune && view === "main") { const n = document.createElement("div"); n.className = "gby-note"; n.setAttribute("role", "presentation"); n.textContent = t("groups.common", { text: ans.comune }); box.append(n); }
    if (!GRUPPI.names().length) { const n = document.createElement("div"); n.className = "gby-note"; n.setAttribute("role", "presentation"); n.textContent = t("groups.noFiles"); box.append(n); }
    items(ans).forEach(it => {
      const d = document.createElement("div");
      d.setAttribute("role", "menuitemradio"); d.setAttribute("aria-checked", String(!!it.checked)); d.tabIndex = -1;
      if (it.disabled) d.setAttribute("aria-disabled", "true");
      const lab = document.createElement("span"); lab.textContent = it.label; d.append(lab);
      if (it.sub) { const s = document.createElement("small"); s.textContent = it.sub; d.append(s); d.setAttribute("aria-label", it.label + " — " + it.sub); }
      if (!it.disabled) d.onclick = () => it.fn();
      box?.append(d);
    });
    /** @type {HTMLElement | null} */ (box.querySelector("[role=menuitemradio]:not([aria-disabled])"))?.focus();
  }
  /** @param {string} [from] id of the button to hang from */
  function open(from = "sb-by") {
    if (box) { const same = anchor === from; close(); if (same) return; }
    anchor = from;
    box = document.createElement("div"); box.className = "gby-menu"; box.setAttribute("role", "menu"); box.setAttribute("aria-label", t("groups.menu"));
    const r = btn().getBoundingClientRect();
    box.style.top = r.bottom + 2 + "px"; box.style.left = Math.max(4, Math.min(r.left, innerWidth - 280)) + "px";
    document.body.append(box); btn().setAttribute("aria-expanded", "true");
    document.addEventListener("pointerdown", away, true);
    box.onkeydown = e => {
      const all = /** @type {HTMLElement[]} */ ([...box?.querySelectorAll("[role=menuitemradio]:not([aria-disabled])") ?? []]), i = all.indexOf(/** @type {HTMLElement} */ (document.activeElement));
      const go = (/** @type {number} */ j) => { e.preventDefault(); all[(j + all.length) % all.length]?.focus(); };
      if (e.key === "ArrowDown") go(i + 1); else if (e.key === "ArrowUp") go(i - 1); else if (e.key === "Home") go(0); else if (e.key === "End") go(-1);
      else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); /** @type {HTMLElement} */ (document.activeElement).click(); }
      else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); if (view === "then") draw("main"); else close(); }
      else if (e.key === "Tab") close(false);
    };
    draw("main");
  }
  /** A small dialog: title, a body built by `fill(body, refresh)`, a preview list and Apply / Cancel; `spec()` = the rule to ask the engine about (null = not ready). @param {string} title @param {(body: HTMLElement, refresh: () => void) => void} fill @param {() => any} spec @param {(ans: any) => any[]} rows @param {(spec: any) => any} apply */
  function dialog(title, fill, spec, rows, apply) {
    const d = document.createElement("dialog"), id = "gby-t" + Date.now();
    d.setAttribute("aria-labelledby", id); d.className = "gby-dlg";
    const h = document.createElement("h2"); h.id = id; h.textContent = t(title);
    const body = document.createElement("div"), pv = document.createElement("ul"), warn = document.createElement("p");
    pv.className = "gby-prev"; pv.setAttribute("aria-label", t("groups.preview")); pv.setAttribute("role", "list"); warn.className = "muted sm"; warn.setAttribute("role", "status");
    const ok = document.createElement("button"), no = document.createElement("button"), row = document.createElement("div");
    ok.type = no.type = "button"; ok.textContent = t("groups.apply"); no.textContent = t("groups.cancel"); no.className = "sec"; row.className = "row"; row.append(no, ok);
    d.append(h, body, pv, warn, row);
    let tick = 0;
    const refresh = () => {
      const s = spec(), my = ++tick; ok.disabled = true; if (!s) { pv.replaceChildren(); warn.textContent = ""; return; }
      GRUPPI.vars(s).then(ans => {
        if (my !== tick) return;
        const man = ans?.manuale, shown = man ? rows(man) : [];
        pv.replaceChildren(...shown.map(t => { const li = document.createElement("li"); li.textContent = t; return li; }));
        if (!shown.length) { const li = document.createElement("li"); li.textContent = t("groups.preview.none"); pv.append(li); }
        warn.textContent = man?.multipli?.length ? t("groups.has.multi", { n: man.multipli.length }) : "";
        ok.disabled = !man || !man.valori.length;
      });
    };
    fill(body, refresh);
    no.onclick = () => d.close();
    ok.onclick = async () => { const s = spec(); d.close(); if (s && !(await GRUPPI.by(apply(s)))) directNote("groups.error"); };
    d.onclose = () => { d.remove(); btn().focus(); };
    document.body.append(d); d.showModal(); refresh();
    /** @type {HTMLElement | null} */ (d.querySelector("input"))?.focus();
  }
  const labeled = (/** @type {string} */ key, /** @type {HTMLElement} */ field) => { const l = document.createElement("label"); l.className = "gby-f"; const s = document.createElement("span"); s.textContent = t(key); l.append(s, field); return l; };
  const rowsOf = (/** @type {any} */ man) => [...man.valori.map((/** @type {any} */ v) => v.etichetta + " · " + v.file.length), ...(man.senza?.length ? [t("groups.other") + " · " + man.senza.length] : [])];
  function dialogPos() {
    const SEP = /** @type {[string, string][]} */ ([["_", "groups.sep.us"], ["-", "groups.sep.hy"], [".", "groups.sep.dot"], [" ", "groups.sep.sp"]]);
    /** @type {HTMLInputElement[]} */ let boxes = []; /** @type {HTMLInputElement} */ let n;
    dialog("groups.pos.title", (body, refresh) => {
      const fs = document.createElement("fieldset"), lg = document.createElement("legend"); lg.textContent = t("groups.pos.sep"); fs.append(lg);
      boxes = SEP.map(([c, k]) => { const i = document.createElement("input"); i.type = "checkbox"; i.checked = true; i.value = c; i.onchange = refresh; const l = document.createElement("label"); l.className = "gby-c"; l.append(i, " " + t(k)); fs.append(l); return i; });
      n = document.createElement("input"); n.type = "number"; n.value = "1"; n.step = "1"; n.oninput = refresh;
      body.append(fs, labeled("groups.pos.n", n));
    }, () => { const p = Number(n.value); return Number.isInteger(p) && p !== 0 ? { pos: p, sep: boxes.filter(b => b.checked).map(b => b.value).join("") || "_-. " } : null; }, rowsOf, m => ({ by: "manuale", manuale: m }));
  }
  function dialogHas() {
    /** @type {HTMLInputElement} */ let terms; /** @type {HTMLInputElement} */ let sub;
    dialog("groups.has.title", (body, refresh) => {
      terms = document.createElement("input"); terms.type = "text"; terms.oninput = refresh;
      sub = document.createElement("input"); sub.type = "checkbox"; sub.onchange = refresh;
      const l = document.createElement("label"); l.className = "gby-c"; l.append(sub, " " + t("groups.has.sub"));
      body.append(labeled("groups.has.terms", terms), l);
    }, () => { const has = terms.value.split(",").map(x => x.trim()).filter(Boolean); return has.length ? { has, substring: sub.checked } : null; }, rowsOf, m => ({ by: "manuale", manuale: m }));
  }
  return { open, close };
})();
