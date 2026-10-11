// @ts-check
"use strict";
// «Statistica» panel of the drawer: sections added by the modules that own them (STAT.add), in the order of the scripts.
//   { id, title (key of lang/), hr: true when the section exists only in high resolution, visible(), mount(box) }
// A section is a <details> with its own box; the panel is painted again at every opening (the files may have changed).
const STAT = (() => {
  const secs = [];
  const t = k => I18N.t(k);
  const isHr = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  const add = s => { if (!secs.some(x => x.id === s.id)) secs.push(s); };
  const open = {};                               // sections the user closed: they stay closed while the panel is painted again
  function mount(box) {
    const live = secs.filter(s => (!s.hr || isHr()) && (!s.visible || s.visible()));
    box.innerHTML = live.map(s => `<details class="stat-sec" data-s="${s.id}"${open[s.id] === false ? "" : " open"}><summary>${t(s.title)}</summary><div id="stat-${s.id}"></div></details>`).join("");
    live.forEach(s => { s.mount(box.querySelector("#stat-" + s.id)); });
    box.querySelectorAll("details.stat-sec").forEach(d => { d.ontoggle = () => { open[d.dataset.s] = d.open; }; });
  }
  return { add, mount, ids: () => secs.filter(s => !s.hr || isHr()).map(s => s.id) };
})();
window.STAT = STAT;
