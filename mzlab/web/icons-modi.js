// @ts-check
// @ts-strict
// Icons of the three acquisition modes: the triple quadrupole itself, Q1 - Q2 - Q3, with what each stage does.
//   sweep (<->, tinted box) = the stage scans a range of m/z;  dot = the stage is fixed on one m/z (filter);
//   X = collision cell (Q2) breaks the ions;  arrow = the stage lets everything through.
//   Full Scan   : Q1 sweeps, Q2 and Q3 let everything through          -> spectrum of the precursors
//   Product ion : Q1 fixed (the precursor), Q2 collides, Q3 sweeps      -> spectrum of the fragments
//   MRM         : Q1 fixed, Q2 collides, Q3 fixed (one transition)      -> chromatogram of one transition
// Colour = currentColor (follows the theme). Use: QICON.full / .prod / .mrm (SVG strings) or QICON.get("full", 20) (height in px).
const QICON = (() => {
  const box = (i, tint) => `<rect x="${1 + i * 16}" y="2" width="14" height="16" rx="3"${tint ? ' fill="currentColor" fill-opacity=".18"' : ""}/>`;
  const sweep = i => `<path d="M${4 + i * 16} 10h8M${6.5 + i * 16} 7.5L${4 + i * 16} 10l2.5 2.5M${9.5 + i * 16} 7.5L${12 + i * 16} 10l-2.5 2.5" stroke-width="1.5"/>`;
  const fixed = i => `<circle cx="${8 + i * 16}" cy="10" r="2.6" fill="currentColor" stroke="none"/>`;
  const hit = i => `<path d="M${5 + i * 16} 6.5l6 7m0-7l-6 7" stroke-width="1.8"/>`;
  const pass = i => `<path d="M${4.5 + i * 16} 10h7M${9 + i * 16} 7.5l2.5 2.5-2.5 2.5" stroke-width="1.5"/>`;
  const link = '<path d="M15 10h2M31 10h2" stroke-width="1.6"/>';
  const wrap = (body, label) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${label}">${link}${body}</svg>`;
  return {
    full: wrap(box(0, 1) + sweep(0) + box(1) + pass(1) + box(2) + pass(2), I18N.t("icon.full")),
    prod: wrap(box(0) + fixed(0) + box(1) + hit(1) + box(2, 1) + sweep(2), I18N.t("icon.prod")),
    mrm: wrap(box(0) + fixed(0) + box(1) + hit(1) + box(2) + fixed(2), I18N.t("icon.mrm")),
    // TP Mine (secret module): a pickaxe and a gem, square
    mine: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="TP Mine"><path d="M5 21.5L13 6.6"/><path d="M3.5 12.5C6 5.5 16 3.5 21 9" stroke-width="2.2"/><path d="M3.5 12.5l.9 2.4M21 9l-2.4-.8" stroke-width="1.4"/><path d="M16.5 16l2.2-2.8 2.2 2.8-2.2 3.2z" fill="currentColor" fill-opacity=".25" stroke-width="1.4"/></svg>`,
    // counter-clockwise arrow: back to the default (form of the Pipeline panel of mzFinder)
    reset: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 12a8 8 0 1 0 2.6-5.9"/><path d="M4 4v5h5"/></svg>`,
    // square icons for the header buttons (24x24): periodic table, adducts, new session
    pt: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.pt')}"><rect x="3" y="4" width="3.4" height="3.8" rx=".8" fill="currentColor" fill-opacity=".3"/><rect x="18" y="4" width="3.4" height="3.8" rx=".8" fill="currentColor" fill-opacity=".3"/><rect x="3" y="10" width="3.4" height="3.8" rx=".8"/><rect x="8" y="10" width="3.4" height="3.8" rx=".8"/><rect x="13" y="10" width="3.4" height="3.8" rx=".8"/><rect x="18" y="10" width="3.4" height="3.8" rx=".8"/><rect x="3" y="16" width="3.4" height="3.8" rx=".8"/><rect x="8" y="16" width="3.4" height="3.8" rx=".8"/><rect x="13" y="16" width="3.4" height="3.8" rx=".8"/><rect x="18" y="16" width="3.4" height="3.8" rx=".8"/></svg>`,
    iso: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" role="img" aria-label="${I18N.t('icon.iso')}"><path d="M5 20V5M11 20V12M17 20V16"/><path d="M3 20.5h18" stroke-width="1.5"/></svg>`,
    nl: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.nl')}"><circle cx="7" cy="12" r="4.5" fill="currentColor" fill-opacity=".18"/><path d="M12.5 12h5M15.5 9l3 3-3 3"/><circle cx="21" cy="12" r="1.6" fill="currentColor" stroke="none"/></svg>`,
    adduct: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.adduct')}"><circle cx="8" cy="14" r="5" fill="currentColor" fill-opacity=".18"/><path d="M8 11.5v5M5.5 14h5" stroke-width="1.5"/><circle cx="18" cy="6.5" r="2.5" fill="currentColor" stroke="none"/><path d="M12.2 10.2l3.6-2.2" stroke-dasharray="1.6 2.2"/></svg>`,
    calc: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.calc')}"><rect x="5" y="2.5" width="14" height="19" rx="2.5"/><rect x="8" y="5.5" width="8" height="4" rx="1" fill="currentColor" fill-opacity=".2"/><path d="M8.5 13.5h.01M12 13.5h.01M15.5 13.5h.01M8.5 17h.01M12 17h.01M15.5 17h.01" stroke-width="2.2"/></svg>`,
    liste: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.liste')}"><path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/></svg>`,
    newrun: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="${I18N.t('icon.newrun')}"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" fill="currentColor" fill-opacity=".1"/><path d="M14 3v5h5"/><path d="M12 11.5v6M9 14.5h6"/></svg>`,
    get(k, px = 20) {
      const w = ["mine", "pt", "adduct", "newrun", "iso", "nl", "calc", "liste", "reset"].includes(k) ? px : Math.round(px * 48 / 20);
      const svg = this[k] || "";
      return svg ? svg.replace("<svg ", `<svg width="${w}" height="${px}" `) : "";
    },
  };
})();
if (typeof module !== "undefined") module.exports = QICON;
