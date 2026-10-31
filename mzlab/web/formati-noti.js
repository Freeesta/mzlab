// @ts-check
// The one table of the instrument file formats mzLab recognises by name (vendor, format, can we read it): the load notice (shell.js, reportLoad) tells
// a known format we cannot open from an unknown file with it. Lazy (carica), shared by the site and the desktop; checked against ordine.rs and
// converti.py by tests/test_formati_noti.py. Facts per format: docs/studi/formati-matrice.md.

/** st: "read" = a reader opens it (the notice appears only where that reader could not run), "unsupported" = recognised, no reader and no converter yet.
 * @typedef {{ re: RegExp, ext: string, vendors: string[], fmt: string, st: "read" | "unsupported" }} Formato */
/** @type {Formato[]} */
export const FORMATI = [
  { re: /\.raw$/i, ext: ".raw", vendors: ["Thermo", "Waters"], fmt: ".raw", st: "read" },
  { re: /\.wiff\.scan$/i, ext: ".wiff.scan", vendors: ["SCIEX"], fmt: ".wiff.scan", st: "read" },
  { re: /\.wiff$/i, ext: ".wiff", vendors: ["SCIEX"], fmt: ".wiff", st: "read" },
  // encrypted, no public specification (docs/studi/ROSINA.md)
  { re: /\.wiff2$/i, ext: ".wiff2", vendors: ["SCIEX"], fmt: ".wiff2", st: "unsupported" },
  { re: /\.d$/i, ext: ".d", vendors: ["Agilent", "Bruker"], fmt: ".d", st: "read" },
  { re: /\.(tdf|tsf)$/i, ext: ".tdf", vendors: ["Bruker"], fmt: ".tdf / .tsf", st: "read" },
  { re: /\.baf$/i, ext: ".baf", vendors: ["Bruker"], fmt: ".baf", st: "unsupported" },
  { re: /\.yep$/i, ext: ".yep", vendors: ["Bruker"], fmt: ".yep", st: "unsupported" },
  { re: /\.(lcd|qgd)$/i, ext: ".lcd", vendors: ["Shimadzu"], fmt: ".lcd / .qgd", st: "read" },
  { re: /\.lcdx$/i, ext: ".lcdx", vendors: ["Shimadzu"], fmt: ".lcdx", st: "unsupported" },
  { re: /\.(peg|smp)$/i, ext: ".peg", vendors: ["LECO"], fmt: ".peg / .smp", st: "unsupported" },
  { re: /\.mzdata$/i, ext: ".mzdata", vendors: ["PSI"], fmt: "mzData", st: "unsupported" },
  { re: /\.cdf$/i, ext: ".cdf", vendors: ["ANDI"], fmt: ".cdf", st: "read" },
  { re: /\.mzxml$/i, ext: ".mzxml", vendors: ["ISB"], fmt: "mzXML", st: "read" },
];

/** The recognised format of a file name, or null for a file we do not know. `ext` is the extension to show, `label` is «SCIEX .wiff2».
 * @param {string} name */
export function classify(name) {
  const f = FORMATI.find(x => x.re.test(name));
  return f ? { ext: f.ext, label: `${f.vendors.join(" / ")} ${f.fmt}`, st: f.st } : null;
}

/** @type {any} */ (window).FORMATI_NOTI = { FORMATI, classify };
