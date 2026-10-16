// @ts-check
// How the loading of one sample of a maker's file (rust-nativo.js) is divided: the share of the bar of each phase (sum 1). The worker and the page (lr.js) read it from here.
/** copy of the members to the browser storage (their fingerprints are taken in the same pass: impronta.js), SHA-256 of the members (only for a sample whose members have no fingerprints in the stub: a phase that disappears when they are there, its share goes to `scans`), opening of the dataset (the reader reads the table of the scans of every sample), index of the scans, chromatograms and index */
export const W = { copy: 0.2, sha: 0.1, open: 0.1, scans: 0.35, chrom: 0.25 };
