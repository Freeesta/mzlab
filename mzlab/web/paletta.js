// @ts-check
"use strict";
// Command palette: Ctrl+K (⌘K on the Mac) or the 🔍 button of the toolbar. It searches by name, in Italian and English, without accents and
// by partial words, among the commands of BARRA.commands (tools, items of the HR menus, "Altro", method, gear, the "+" actions of the charts,
// side of the panel). Enter runs, ↑ ↓ move, Esc closes. The list of commands is the one of barra.js: nothing is registered here.

const PALETTE = (() => {
  const GROUPS = ["tools", "hr", "panel", "graphs", "app"];
  let box = null, input = null, list = null, live = null;
  let rows = [];
  let sel = 0;
  let prev = null;

  const t = (k, p) => I18N.t(k, p);
  const norm = s => String(s == null ? "" : s).normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const esc = s => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
  const groupOf = c => (c.group || (c.item || c.hr ? "hr" : "tools"));
  const kw = c => (c.teo ? "" : t(`cmd.${c.id}.kw`));

  function candidates() {
    if (BARRA.inTheory) return BARRA.theoryCommands().filter(c => !c.off);      // in the Teoria the palette searches the commands of the Teoria
    return BARRA.commands.filter(c => c.run && !c.items && c.id !== "search" && BARRA.visible(c));
  }

  // every word of the query must be inside the name, the short label or the keywords; names count more than keywords
  function search(q) {
    const toks = norm(q).split(/\s+/).filter(Boolean);
    const out = [];
    candidates().forEach((c, i) => {
      const name = norm(BARRA.name(c)), lab = norm(BARRA.lab(c)), k = norm(kw(c));
      let score = 0;
      for (const w of toks) {
        const inName = name.indexOf(w);
        if (inName >= 0) score += (inName === 0 || name[inName - 1] === " ") ? 0 : 1;
        else if (lab.includes(w)) score += 2;
        else if (k.includes(w)) score += 3;
        else return;
      }
      out.push({ c, score, i });
    });
    out.sort((a, b) => a.score - b.score || GROUPS.indexOf(groupOf(a.c)) - GROUPS.indexOf(groupOf(b.c)) || a.i - b.i);
    return out.map(x => x.c);
  }

  function build() {
    box = document.createElement("div");
    box.id = "paletta";
    box.hidden = true;
    box.setAttribute("role", "dialog");
    box.setAttribute("aria-modal", "true");
    box.setAttribute("aria-label", t("pal.title"));
    box.innerHTML = `<div class="pal-b"><input type="text" id="pal-in" role="combobox" aria-expanded="true" aria-controls="pal-l" aria-autocomplete="list" aria-label="${esc(t("pal.title"))}" placeholder="${esc(t("pal.placeholder"))}" autocomplete="off" autocapitalize="off" spellcheck="false">` +
      `<ul id="pal-l" role="listbox" aria-label="${esc(t("pal.title"))}"></ul><div class="pal-f"><span>${esc(t("pal.hint"))}</span><span id="pal-n" role="status" aria-live="polite" class="pal-sr"></span></div></div>`;
    document.body.appendChild(box);
    input = box.querySelector("#pal-in");
    list = box.querySelector("#pal-l");
    live = box.querySelector("#pal-n");
    input.addEventListener("input", () => render(input.value));
    input.addEventListener("keydown", onKey);
    box.addEventListener("pointerdown", e => { if (e.target === box) close(true); });
    list.addEventListener("click", e => { const li = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("li[data-i]")); if (li) run(+li.dataset.i); });
    list.addEventListener("pointermove", e => { const li = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("li[data-i]")); if (li && +li.dataset.i !== sel) mark(+li.dataset.i, false); });
  }

  function render(q) {
    rows = search(q);
    sel = 0;
    list.innerHTML = rows.map((c, i) => `<li role="option" id="pal-o-${i}" data-i="${i}" aria-selected="${i === 0}"><span class="pal-i" aria-hidden="true">${c.ic()}</span><span class="pal-t">${esc(BARRA.name(c))}</span><span class="pal-g">${esc(t(`pal.grp.${groupOf(c)}`))}</span></li>`).join("") ||
      `<li class="pal-e" role="presentation">${esc(t("pal.empty"))}</li>`;
    input.setAttribute("aria-activedescendant", rows.length ? "pal-o-0" : "");
    live.textContent = rows.length ? t("pal.count", { n: rows.length }) : t("pal.empty");
  }

  function mark(i, scroll = true) {
    if (!rows.length) return;
    sel = (i + rows.length) % rows.length;
    list.querySelectorAll("li[data-i]").forEach(li => li.setAttribute("aria-selected", String(+li.dataset.i === sel)));
    input.setAttribute("aria-activedescendant", "pal-o-" + sel);
    const li = list.querySelector(`#pal-o-${sel}`);
    if (li && scroll) li.scrollIntoView({ block: "nearest" });
  }

  function onKey(e) {
    if (e.key === "ArrowDown") { e.preventDefault(); mark(sel + 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); mark(sel - 1); }
    else if (e.key === "Enter") { e.preventDefault(); run(sel); }
    else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(true); }
    else if (e.key === "Tab") { e.preventDefault(); mark(sel + (e.shiftKey ? -1 : 1)); }
  }

  function run(i) {
    const c = rows[i];
    if (!c) return;
    close(false);
    setTimeout(() => { if (prev && document.contains(prev)) { try { prev.focus({ preventScroll: true }); } catch (_) { /* gone */ } } c.run(); }, 0);
  }

  function open() {
    if (!box) build();
    if (!box.hidden) { input.focus(); return; }
    prev = document.activeElement;
    box.hidden = false;
    input.value = "";
    render("");
    input.focus();
  }

  function close(restore) {
    if (!box || box.hidden) return;
    box.hidden = true;
    if (restore && prev && document.contains(prev)) { try { prev.focus({ preventScroll: true }); } catch (_) { /* gone */ } }
  }

  document.addEventListener("keydown", e => {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.key === "k" || e.key === "K")) {
      if (document.documentElement.classList.contains("phone")) return;
      e.preventDefault();
      if (box && !box.hidden) close(true); else open();
    }
  }, true);

  return { open, close, search, get isOpen() { return !!box && !box.hidden; } };
})();

if (typeof window !== "undefined") window.PALETTE = PALETTE;
