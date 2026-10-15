// @ts-check
// The families of the readers modules (docs/agenti/lettori-modulo.md): one .wasm per family in wasm-lettori/<family>/, loaded only when a file of the family arrives.
// `ext` = every extension the family reads (a test of crates/mzlab-lettori checks that the extensions of each reader are in its row), `membri` = names of the members of a dataset
// (lower case, a part of the path) that tell the family, `errori` = the keys of the engine's errors that send a sample to the module, `soloModulo` = the extensions only the module reads
// (the engine reads the others as they are): a file with one of these is taken by the module and is not refused as «cannot be opened». The errors a module raises itself reach the page
// as `{error_key, params}` and are shown through the catalogs (`err.file.netcdf4` for the family `andi`).
// `sonda` = the module looks at every file of the family before the engine (its `probe` answers «variante» for the instruments and experiments the engine does not read).
/** @typedef {{ ext: string[], membri: string[], errori: string[], soloModulo: string[], sonda?: boolean }} Famiglia */
/** @type {Record<string, Famiglia>} */
const LETTORI_FAM = {
  aperti: { ext: [".mzml", ".mzxml"], membri: [], errori: ["err.file.numpress"], soloModulo: [".mzxml"] },
  andi: { ext: [".cdf", ".nc"], membri: [], errori: [], soloModulo: [".cdf", ".nc"] },
  thermo: { ext: [".raw"], membri: [], errori: [], soloModulo: [] },
  agilent: { ext: [".d"], membri: ["acqdata/", ".d/"], errori: ["err.file.agilent"], soloModulo: [] },
  sciex: { ext: [".wiff", ".wiff.scan"], membri: [], errori: ["err.file.wiff"], soloModulo: [], sonda: true },
  waters: { ext: [], membri: [], errori: [], soloModulo: [] },
  shimadzu: { ext: [], membri: [], errori: [], soloModulo: [] },
  bruker: { ext: [], membri: [], errori: [], soloModulo: [] },
};
/** @type {any} */ (window).LETTORI_FAM = LETTORI_FAM;
export default LETTORI_FAM;
