// @ts-nocheck
// A UV/Vis row of the MRM / SIM picker or of the MS2 chromatogram opens (or shows again) a chromatogram of that detector right under the panel,
// in the same tab and time-linked as every new chromatogram. Classic script, loaded on demand (carica) by canali.js.
function CANALIUV(p, v) {
  const back = () => p.el.querySelector('[data-o="canali"],[data-o="kind"]')?.focus();
  if (!/^(pda|press|an:)/.test(v)) return back();      // the «MS2» row of the MS2 chromatogram: nothing to open
  let q = E.panels.find(x => x.type === "chrom" && x.tab === p.tab && x.kind === v && x.mz0 == null);
  if (!q) {
    q = apriAnnullabile(addPanel("chrom", { kind: v, tab: p.tab }));
    q.el.classList.add("drag"); q.y = p.y + p.h; q.el.style.top = q.y + "px"; void q.el.offsetHeight; q.el.classList.remove("drag");
    stackAfter(q, p); relayout(); fitHost(); uiSave();
  }
  reveal(q);
  const f = () => q.el.querySelector('[data-o="kind"]'), b = f(); if (b) b.focus(); else return back();
  // a redraw of the controls (the channels of the files arrive) replaces the button: the focus follows it until the user moves it
  const mo = new MutationObserver(() => { if (document.activeElement === document.body) f()?.focus({ preventScroll: true }); });
  mo.observe(q.el, { childList: true, subtree: true });
  document.addEventListener("focusout", function h(e) { if (e.relatedTarget) { mo.disconnect(); document.removeEventListener("focusout", h); } });
}
