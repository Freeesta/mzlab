//! The help text of `spectrum get` (the usage line is what `commands --json` reads its flags from) and the flags of `spectrum get` that take a value.

pub const HELP_GET: &str = "mzlab-cli spectrum get: one mass spectrum as JSON (spectrum/1) or CSV (mz,intensity), the answer of the page's api/spectrum.

usage: mzlab-cli spectrum get FILE (--rt MIN | --rt A,B | --scan NO) [--ms-level N] [--id CHANNEL] [--sample ID] [--sum] [--peak auto [--set NAME=VALUE] [--precursor MZ]] [--fill-ms T] [--minus FILE] [--minus-sample ID] [--minus-rt A,B] [--bin nominal|0.1 [--mz-offset DA|auto:MZ[@RT]]] [--format json|csv] [--out PATH]

  --rt MIN    the scan nearest to MIN (minutes) at the level; the evidence names it
  --rt A,B    the mean spectrum of the scans between A and B (minutes)
  --sum       with --rt A,B: add the scans instead of averaging them (`mzlab-cli spectrum get f.mzML --rt 6,7 --ms-level 2 --sum`); `evidence.sum` gives the
              scans added, their counts (cps summed), the ions (counts x fill time) per scan type and the ion quantum, so a reader can judge whether the
              spectrum rests on enough ions; no background is subtracted (`evidence.sum.background`); not with --minus or --id
  --peak auto with --rt A,B: find the apex of the chromatogram of the level (of --precursor MZ at a product-ion level) between A and B, sum the scans of
              a window of `factor` x FWHM around it and subtract the mean of the two side windows (`mzlab-cli spectrum get f.mzML --rt 4,9 --ms-level 2
              --precursor 250.3 --peak auto`); `evidence.peak` names the apex time, FWHM, window, side windows, background level, `n_scans`, `total_ions`
              and the rule of the verdict (peak, no_apex, no_data, window_only); without an apex no spectrum is made (no points; the reason on stderr)
  --set N=V   with --peak auto: factor=2 (window = factor x FWHM), side_min=0.3 and gap_min=0.05 (minutes: width of each side, gap between window and side),
              min_sn=3 and min_points=3 (the apex must reach them); repeatable or comma separated
  --precursor MZ the product-ion scans of this precursor only (with --peak auto)
  --fill-ms T the fill time of a scan in ms for --sum and --peak auto when the file does not give it (the ions are null without it; the user states it, it is never guessed)
  --scan NO   the scan with that number (`spectrum list` prints it as scan_no); its level and precursor are used; no such scan: exit 5
  --ms-level N the level (default: the level of --scan; the file's only level; else 1, and a file without level 1 names its levels: exit 5)
  --id CHANNEL only the scans of an ms:<key> or dd:<level> channel `file channels` lists
  --sample ID the sample of a file with several (.wiff); default the first
  --minus FILE a blank: its mean spectrum is subtracted (the engine's `/api/spectrum` with a background: lines where the difference is above zero,
              factor 1, 5 ppm at high resolution; the evidence's `minus` says the window and the scan used); with --minus-sample and no FILE, the blank is
              another sample of FILE
  --minus-sample ID the sample of the blank (a file of several samples)
  --minus-rt A,B the blank's window in minutes; default the window of --rt, or the blank's scan nearest to a single --rt or --scan
  --bin RULE  unit-resolution data cut into bins (`mzlab-cli spectrum get f.mzML --rt 6,7 --bin nominal --mz-offset 0.24`): the points are summed into
              half-open bins [n - 0.5, n + 0.5) around integers (`nominal`) or [k/10 - 0.05, k/10 + 0.05) around tenths (`0.1`) AFTER the offset is taken off;
              the output (spectrum-bins/1, or CSV bin,mz,counts,points) gives the bin, its centre of mass on the corrected scale (intensity-weighted) and the
              counts summed; the rule and the offset (also 0) are in the document and on stderr; not with --minus, --sum or --peak; a high-resolution file is refused
  --mz-offset DA|auto:MZ[@RT]  with --bin: the offset (observed - true, Da) taken off every m/z before the cut: a number (limit 0.6), or measured on the known ion MZ
              (near RT min) as in `trace xic`; the evidence of an estimate is in the document
  --format F  json (default) or csv with the header mz,intensity
  --out PATH  write there instead of the standard output";

/// The flags of `spectrum get` that take a value.
pub(crate) const GET_VALUES: &[&str] = &[
    "--rt",
    "--scan",
    "--ms-level",
    "--id",
    "--sample",
    "--minus",
    "--minus-sample",
    "--minus-rt",
    "--fill-ms",
    "--bin",
    "--mz-offset",
    "--peak",
    "--set",
    "--precursor",
    "--format",
    "--out",
];
