// @ts-strict
// @ts-check
"use strict";
// Search box (the 🔍 button of the toolbar, Ctrl+K / ⌘K): one field for commands, numbers, formulas and the Teoria.
// Words are looked up among the commands of BARRA.commands (by name, in Italian and English, without accents, by partial words); a number or a
// formula also gets "compatible with" rows from ricerca.js (shown above the commands) and words get pages of the Teoria (below). Empty: the recent commands.
// Enter runs, ↑ ↓ move, Esc closes. The list of commands is the one of barra.js: nothing is registered here.

// Pure helpers of the box, outside the module closure (the closure keeps only the state and the DOM).
const PAL_GROUPS = ["tools", "hr", "panel", "graphs", "app"], PAL_RECENT = "mzlab.pal.recent", PAL_RECENT_N = 5;
const palNorm = (/** @type {unknown} */ s) => String(s == null ? "" : s).normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
const palEsc = (/** @type {unknown} */ s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
const palGroupOf = (/** @type {any} */ c) => (c.group || (c.item || c.hr ? "hr" : "tools"));
const palKw = (/** @type {any} */ c) => (c.teo ? "" : I18N.t(`cmd.${c.id}.kw`));

// every word of the query must be inside the name, the short label or the keywords; names count more than keywords
function palSearch(/** @type {string} */ q, /** @type {any[]} */ cands) {
  const toks = palNorm(q).split(/\s+/).filter(Boolean);
  const out = [];
  cands.forEach((c, i) => {
    const name = palNorm(BARRA.name(c)), lab = palNorm(BARRA.lab(c)), k = palNorm(palKw(c));
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
  out.sort((a, b) => a.score - b.score || PAL_GROUPS.indexOf(palGroupOf(a.c)) - PAL_GROUPS.indexOf(palGroupOf(b.c)) || a.i - b.i);
  return out.map(x => x.c);
}

// recent commands (ids in localStorage, optional: the page works without it)
const palRecentIds = () => { try { const a = JSON.parse(localStorage.getItem(PAL_RECENT) || "[]"); return Array.isArray(a) ? a.filter(x => typeof x === "string") : []; } catch (_) { return []; } };
const palRemember = (/** @type {string} */ id) => { try { localStorage.setItem(PAL_RECENT, JSON.stringify([id, ...palRecentIds().filter(x => x !== id)].slice(0, PAL_RECENT_N))); } catch (_) { /* no storage */ } };

// the key of a command that is also an action of the registry (field `azione`): the text of its shortcut, as the system writes it
const palKey = (/** @type {any} */ c) => { const a = c.azione && typeof AZIONI !== "undefined" ? AZIONI.voce(c.azione) : null; return a && a.scorciatoia ? `<kbd class="pal-k">${palEsc(AZIONI.testo(a.scorciatoia))}</kbd>` : ""; };

// a selectable row: icon, text (html), tail (html), accessible name, action
const palCmdRow = (/** @type {any} */ c) => ({ ic: c.ic(), html: palEsc(BARRA.name(c)), tail: palKey(c) + `<span class="pal-g">${palEsc(I18N.t(`pal.grp.${palGroupOf(c)}`))}</span>`, aria: "", run: () => { if (!c.teo) palRemember(c.id); c.run(); } });
const palResRow = (/** @type {any} */ r) => ({ ic: r.ic, html: r.html, tail: `<span class="pal-c">${palEsc(r.chip)}</span>` + (r.detail ? `<span class="pal-g">${palEsc(r.detail)}</span>` : ""), aria: r.aria, run: r.run });

// blocks [{title, rows}] -> the html of the list (group titles only with several blocks or an empty query) and its selectable rows
function palList(/** @type {{title: string, rows: any[]}[]} */ blocks, /** @type {string} */ query, /** @type {boolean} */ pending) {
  const full = blocks.filter(b => b.rows.length), rows = [];
  let html = "";
  for (const b of full) {
    if (full.length > 1 || !query) html += `<li class="pal-h" role="presentation">${palEsc(b.title)}</li>`;
    for (const r of b.rows) {
      const i = rows.push(r) - 1;
      html += `<li role="option" id="pal-o-${i}" data-i="${i}" aria-selected="false"${r.aria ? ` aria-label="${palEsc(r.aria)}"` : ""}><span class="pal-i" aria-hidden="true">${r.ic}</span><span class="pal-t">${r.html}</span>${r.tail}</li>`;
    }
  }
  if (!rows.length && !pending) html += `<li class="pal-e" role="presentation">${palEsc(query ? I18N.t("pal.empty") : I18N.t("pal.try"))}</li>`;
  else if (!query) html += `<li class="pal-e" role="presentation">${palEsc(I18N.t("pal.try"))}</li>`;
  return { html, rows };
}

const PALETTE = (() => {
  let box = null, input = null, list = null, live = null;
  let rows = [];
  let sel = 0;
  let prev = null;
  let seq = 0, timer = 0;
  const MAX_CMD = 12;

  const t = (k, p) => I18N.t(k, p);

  function candidates() {
    // in the Teoria the palette searches the commands of the Teoria
    if (BARRA.inTheory) return BARRA.theoryCommands().filter(c => !c.off);
    return BARRA.commands.filter(c => c.run && !c.items && c.id !== "search" && BARRA.visible(c));
  }
  const search = q => palSearch(q, candidates());

  function build() {
    box = document.createElement("div");
    box.id = "paletta";
    box.hidden = true;
    box.setAttribute("role", "dialog");
    box.setAttribute("aria-modal", "true");
    box.setAttribute("aria-label", t("pal.title"));
    box.innerHTML = `<div class="pal-b"><input type="text" id="pal-in" role="combobox" aria-expanded="true" aria-controls="pal-l" aria-autocomplete="list" aria-label="${palEsc(t("pal.title"))}" placeholder="${palEsc(t("pal.placeholder"))}" autocomplete="off" autocapitalize="off" spellcheck="false">` +
      `<ul id="pal-l" role="listbox" aria-label="${palEsc(t("pal.title"))}"></ul><div class="pal-f"><span>${palEsc(t("pal.hint"))}</span><span id="pal-n" role="status" aria-live="polite" class="pal-sr"></span></div></div>`;
    document.body.appendChild(box);
    input = box.querySelector("#pal-in");
    list = box.querySelector("#pal-l");
    live = box.querySelector("#pal-n");
    input.addEventListener("input", () => render(input.value));
    input.addEventListener("keydown", onKey);
    box.addEventListener("pointerdown", e => { if (e.target === box) close(true); });
    list.addEventListener("click", e => { const li = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("li[data-i]")); if (li) run(+(li.dataset.i || 0)); });
    list.addEventListener("pointermove", e => { const li = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("li[data-i]")); if (li && +(li.dataset.i || 0) !== sel) mark(+(li.dataset.i || 0), false); });
  }

  function paint(blocks, pending) {
    const r = palList(blocks, input.value.trim(), pending);
    rows = r.rows;
    list.innerHTML = r.html;
    sel = Math.min(sel, Math.max(rows.length - 1, 0));
    if (rows.length) mark(sel, false); else input.setAttribute("aria-activedescendant", "");
    live.textContent = rows.length ? t("pal.count", { n: rows.length }) : pending ? "" : t("pal.empty");
  }

  function render(q) {
    const my = ++seq, s = q.trim();
    clearTimeout(timer);
    sel = 0;
    const cmds = s ? search(q).slice(0, MAX_CMD).map(palCmdRow) : palRecentIds().map(id => candidates().find(c => c.id === id)).filter(Boolean).map(palCmdRow);
    const cmdBlock = { title: t(s ? "pal.grp.commands" : "pal.grp.recent"), rows: cmds };
    const rich = s && !BARRA.inTheory && window.RICERCA;
    paint([cmdBlock], !!rich);
    if (!rich) return;
    // numbers and formulas ask the engine: a short wait so that a fast typist does not send every key
    timer = window.setTimeout(async () => {
      const r = await RICERCA.risultati(s);
      if (my !== seq) return;
      const above = r.groups.filter(g => g.id !== "theory"), below = r.groups.filter(g => g.id === "theory");
      const blk = g => ({ title: g.title, rows: g.rows.map(palResRow) });
      paint([...above.map(blk), cmdBlock, ...below.map(blk)], false);
    }, 150);
  }

  function mark(i, scroll = true) {
    if (!rows.length) return;
    sel = (i + rows.length) % rows.length;
    list.querySelectorAll("li[data-i]").forEach(li => li.setAttribute("aria-selected", String(+(li.dataset.i || 0) === sel)));
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
    const r = rows[i];
    if (!r) return;
    close(false);
    setTimeout(() => { if (prev && document.contains(prev)) { try { prev.focus({ preventScroll: true }); } catch (_) { /* gone */ } } r.run(); }, 0);
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
