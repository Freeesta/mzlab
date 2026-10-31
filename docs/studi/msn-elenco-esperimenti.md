# MSn tree of a DDA file: the list of the MSn experiments (design note)

Goal (brief `dev-msn-experiments-list`): the «MSn tree» tool (`COMP.tree`, header button of #2222) also serves chromatographic DDA files, whose tree is shallow (precursor → MS2, rarely MS3) and wide (thousands of precursors). One tool, two views: «Tree» (the fragmentation paths, as today) and «Experiments» (every MSn scan).

## How the vendor programs show it
- **Thermo Xcalibur / FreeStyle**: the «Scan Filters» list (one line per filter string, `ms2 445.12@hcd30.00`) and the «MSn Browser» (precursor tree on the left, scan list in the middle, spectrum on the right); «Nearby Precursors» shows the MS2 of the same ion within ±1 min. Strong: every scan reachable in one click. Weak: the filter list explodes on DDA (one filter per precursor), no grouping within ppm, no sort by intensity.
- **Waters MassLynx**: the chromatogram of the function (MS^E / DDA functions) and the «Spectrum list» per function; a precursor is found through the function's TIC, not through a list of precursors.
- **SCIEX Analyst / PeakView**: the IDA «Explorer» lists the dependent experiments per cycle (period, cycle, experiment) with precursor m/z and charge; a click shows the spectrum, and a precursor's XIC is a separate action.

## Proposal
- A tab bar in the MSn tree window: «Tree» | «Experiments». A DDA file opens on «Experiments»; an infusion file keeps «Tree» alone.
- «Experiments»: one virtualised table (fixed row height, only the rows in view in the DOM, as the scan list of the side panel) grouped by precursor (within the file's tolerance, `DDA.groups`), then by scan. Group row: precursor m/z, charge, number of scans, RT range, most intense precursor; it opens and closes. Scan row: scan number, RT, MS level, precursor m/z, charge, activation and collision energy, base peak intensity. Sort by every column (groups keep their scans together), filters by m/z range, RT range and MS level; the count of the shown scans and groups is on top.
- Click (or Enter) on a scan row: the window closes and that scan opens in the DDA panels (the MS2 panel next to the Full Scan, `DDA.showMs2`), with the Full Scan of its parent.
- Per group, two buttons: «TIC of this precursor» (the MS2 tab chromatogram with `prec`, the precursor window = the file's tolerance, `ptol`), and «XIC of the precursor» (the usual XIC with the ppm/Da window of the file, `xicDirect`). Both are ordinary chromatogram panels: a click on a point opens its spectrum as usual.

## Reuse and parity
- Data: `/api/dda` (Rust `hrroutes.rs`, the same core in `.wasm` and in the desktop app) gains two columns, `lev` (MS level) and `z` (charge); the base peak intensity comes from `/api/scanlist` per level. No new route, no new Python function: the reference `explore.dda` gains the same two columns so `hr_parity` keeps comparing.
- The precursor TIC uses `/api/chrom` with `prec` and the new optional `ptol` (default 0.6 Da = the behaviour of today, so the parity queries do not change).
- Code: one lazy ES module `msn-elenco.js` (loaded on the first opening of the window; the start-up scripts only gain the tab bar in `composizione.js`).
