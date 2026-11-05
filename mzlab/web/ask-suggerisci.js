// @ts-strict
"use strict";
// Suggestions under the text field of the small question dialog (#askin): a combobox with a listbox (no <datalist>: Safari and iPad draw it badly).
// Lazy (carica("ask-suggerisci.js")): ask() in explore.js attaches it when its caller passes `suggest`.
// The caller gives the rows for a text; a pick returns the text to put in the field. Keyboard: up/down move, Enter picks, Esc closes the list only.
const ASKSUG = (() => {
  /** @param {HTMLInputElement} input
   *  @param {{ rows: (q: string) => Bag[], text: (r: Bag) => string, ready?: () => Promise<any> }} o */
  function attach(input, o) {
    const T = I18N.t.bind(I18N), ul = document.createElement("ul"), live = document.createElement("div");
    ul.id = "asksug"; ul.className = "asksug"; ul.setAttribute("role", "listbox"); ul.setAttribute("aria-label", T("ctx.ask.sug.label")); ul.hidden = true;
    live.className = "sr-only"; live.setAttribute("role", "status");
    input.after(ul, live);
    input.setAttribute("role", "combobox"); input.setAttribute("aria-autocomplete", "list"); input.setAttribute("aria-controls", "asksug");
    input.setAttribute("aria-expanded", "false"); input.autocomplete = "off";
    let /** @type {Bag[]} */ cur = [], at = -1, alive = true;
    const close = () => { cur = []; at = -1; ul.hidden = true; ul.textContent = ""; input.setAttribute("aria-expanded", "false"); input.removeAttribute("aria-activedescendant"); };
    const mark = () => {
      [...ul.children].forEach((li, i) => { li.setAttribute("aria-selected", String(i === at)); if (i === at) { input.setAttribute("aria-activedescendant", li.id); li.scrollIntoView({ block: "nearest" }); } });
      if (at < 0) input.removeAttribute("aria-activedescendant");
    };
    const pick = (/** @type {number} */ i) => { const r = cur[i]; if (!r) return; input.value = o.text(r); close(); live.textContent = ""; input.focus(); input.setSelectionRange(input.value.length, input.value.length); };
    function refresh() {
      if (!alive) return;
      const q = input.value.split("[")[0].trim(), rows = q.length >= 2 ? o.rows(q) : [];
      close();
      if (!rows.length) { live.textContent = ""; return; }
      cur = rows; ul.hidden = false; input.setAttribute("aria-expanded", "true");
      rows.forEach((r, i) => {
        const li = document.createElement("li"), n = document.createElement("span"), f = document.createElement("span");
        li.id = "asksug-" + i; li.setAttribute("role", "option"); li.setAttribute("aria-selected", "false"); li.tabIndex = -1;
        n.className = "nm"; n.textContent = r.name; f.className = "fm"; f.textContent = r.formula;
        li.append(n, f);
        li.addEventListener("pointerdown", e => { e.preventDefault(); pick(i); });      // before the blur: the field keeps the focus
        ul.append(li);
      });
      live.textContent = T("ctx.ask.sug.n", { n: rows.length });
    }
    const onInput = () => refresh();
    input.addEventListener("input", onInput);
    if (o.ready) o.ready().then(refresh).catch(() => {});
    return {
      /** True when the key was used by the list (the dialog then does nothing with it). */
      key(/** @type {KeyboardEvent} */ e) {
        if (ul.hidden) return false;
        if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          e.preventDefault();
          const n = cur.length;          // -1 = no row (the field itself): the arrows wrap through it
          at = e.key === "ArrowDown" ? (at + 1 >= n ? -1 : at + 1) : (at < 0 ? n - 1 : at - 1);
          mark(); return true;
        }
        if (e.key === "Enter" && at >= 0) { e.preventDefault(); pick(at); return true; }
        if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); return true; }
        return false;
      },
      detach() {
        alive = false; input.removeEventListener("input", onInput); ul.remove(); live.remove();
        ["role", "aria-autocomplete", "aria-controls", "aria-expanded", "aria-activedescendant"].forEach(a => input.removeAttribute(a));
      }
    };
  }
  return { attach };
})();
