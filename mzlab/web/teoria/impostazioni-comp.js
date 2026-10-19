/* Building blocks of the settings panels, shared by the Teoria (impostazioni.js) and the program (settings.js): option buttons (segmented, font, background),
   wide switches, sections and folding groups. They only return HTML; each panel binds its own events. Labels come from the caller (the Teoria is Italian only,
   the program is bilingual), so nothing here is text for the user. Styles: impostazioni-comp.css. Classic script, no library. */
(function () {
  "use strict";
  const ico = d => `<svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
  const TICON = {
    auto: ico('<circle cx="10" cy="10" r="6.5"/><path d="M10 3.5a6.5 6.5 0 0 1 0 13z" fill="currentColor"/>'),
    light: ico('<circle cx="10" cy="10" r="3.2"/><path d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M4.7 15.3l1.4-1.4M13.9 6.1l1.4-1.4"/>'),
    dark: ico('<path d="M16 11.6A6.3 6.3 0 0 1 8.4 4 6.3 6.3 0 1 0 16 11.6z"/>'),
  };
  // icons of the groups and of the single rows of the program (same 20x20 line style as the theme icons)
  const GICON = {
    look: ico('<circle cx="10" cy="10" r="7"/><path d="M10 3a7 7 0 0 0 0 14z" fill="currentColor"/>'),
    read: ico('<path d="M3 4.5h5a2 2 0 0 1 2 2V16a2 2 0 0 0-2-2H3zM17 4.5h-5a2 2 0 0 0-2 2V16a2 2 0 0 1 2-2h5z"/>'),
    data: ico('<ellipse cx="10" cy="5" rx="6" ry="2.5"/><path d="M4 5v10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5V5M4 10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5"/>'),
    app: ico('<rect x="3" y="3.5" width="14" height="13" rx="2"/><path d="M3 7.5h14"/>'),
    lang: ico('<circle cx="10" cy="10" r="7"/><path d="M3 10h14M10 3c2.3 2 2.3 12 0 14M10 3c-2.3 2-2.3 12 0 14"/>'),
    play: ico('<path d="M6 4l10 6-10 6z" fill="currentColor"/>'),
    pal: ico('<path d="M10 3a7 7 0 1 0 0 14c1.2 0 1.6-.8 1.2-1.6-.5-1 .1-2 1.2-2H14a3 3 0 0 0 3-3A7 7 0 0 0 10 3z"/><circle cx="6.8" cy="9" r=".6"/><circle cx="9" cy="6.3" r=".6"/><circle cx="12.4" cy="6.8" r=".6"/>'),
    side: ico('<rect x="3" y="4" width="14" height="12" rx="1.5"/><path d="M10 4v12"/>'),
    cont: ico('<path d="M3 17V9M7 17V4M11 17v-6M15 17V7"/>'),
    lib: ico('<path d="M5 3.5v13M9 3.5v13M13 4l3 12"/>'),
    update: ico('<path d="M16 10a6 6 0 1 1-2-4.5M16 3.5v3h-3"/>'),
    ext: ico('<path d="M8 4H4.5v11.5H16V12M11 4h5v5M16 4l-7 7"/>'),
    motion: ico('<path d="M3 10h3l2-5 3 10 2-5h4"/>'),
    hc: ico('<circle cx="10" cy="10" r="7"/><path d="M10 3v14" /><path d="M10 3a7 7 0 0 1 0 14z" fill="currentColor"/>'),
    ppm: ico('<path d="M4 15l4-9 4 9M5.5 12h5M14 6v9M14 6l3 0"/>'),
    chev: ico('<path d="M5 8l5 5 5-5"/>'),
  };
  const TICK = '<svg class="tick" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 7.5l2.7 2.7L11 4.5"/></svg>';
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  /**
   * One row of big option buttons. The radios stay real (arrows move the choice); the label draws the option and the input covers it.
   * @param {string} name input name   @param {Array<[string, string, string?, string?]>} list [value, label, tip?, icon svg?]   @param {string} cur   @param {"seg"|"font"|"bg"} kind
   */
  function radios(name, list, cur, kind) {
    return list.map(([v, t, tip, icon]) => {
      const mark = kind === "seg" ? (icon || TICON[v] || "") + `<span>${t}</span>` : kind === "font" ? `<b class="aa" data-f="${v}">Aa</b><span>${t}</span>` : `<i class="chip" data-v="${v}">${TICK}</i><span>${t}</span>`;
      return `<label class="opt ${kind}"${tip ? ` title="${esc(tip)}"` : ""}><input type="radio" name="${name}" value="${esc(v)}"${(cur || "") === v ? " checked" : ""}>${mark}</label>`;
    }).join("");
  }
  /** A section with a small-caps title: `grid` is the class of the row of buttons (seg2, seg3, f3, f4). */
  const section = (title, body) => `<fieldset><legend>${title}</legend>${body}</fieldset>`;
  const opts = (grid, html) => `<div class="opts ${grid}">${html}</div>`;
  /** A wide switch; `attr` is what the panel binds on (data-k="…", id="…"); `icon` an optional key of GICON. */
  const sw = (text, hint, attr, on, icon) => `<label class="swrow"${hint ? ` title="${esc(hint)}"` : ""}><span>${icon && GICON[icon] ? `<span class="ic">${GICON[icon]}</span>` : ""}${text}</span><input type="checkbox" role="switch" ${attr}${on ? " checked" : ""}></label>`;
  /** A folding group with an icon; `id` is also the key under which the panel remembers if it was left open. */
  const group = (id, icon, title, body, open) => `<details class="imp-g" id="${id}"${open ? " open" : ""}><summary><span class="gi">${GICON[icon] || ""}</span>${title}<span class="car">${GICON.chev}</span></summary><div class="imp-b">${body}</div></details>`;
  window.IMPCOMP = { ico, TICON, GICON, TICK, radios, section, opts, sw, group, esc };
})();
