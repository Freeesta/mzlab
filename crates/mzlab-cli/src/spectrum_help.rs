//! The help texts of `spectrum get` and `spectrum list` (the usage line of each is what `commands --json` reads its flags from).

pub const HELP_GET: &str = "mzlab-cli spectrum get: one mass spectrum as JSON (spectrum/1) or CSV (mz,intensity), the answer of the page's api/spectrum.

usage: mzlab-cli spectrum get FILE (--rt MIN | --rt A,B | --scan NO) [--ms-level N] [--id CHANNEL] [--sample ID] [--sum] [--peak auto [--set NAME=VALUE] [--precursor MZ]] [--fill-ms T] [--minus FILE] [--minus-sample ID] [--minus-rt A,B] [--format json|csv] [--out PATH]

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
  --format F  json (default) or csv with the header mz,intensity
  --out PATH  write there instead of the standard output";

pub const HELP_LIST: &str = "mzlab-cli spectrum list: the scans of a file as JSON (scanlist/1) or CSV, the answer of the page's api/scanlist.

usage: mzlab-cli spectrum list FILE [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]

  --ms-level N one level; default every level, in time order
  --id CHANNEL only the scans of an ms:<key> or dd:<level> channel
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv: scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity
  --out PATH  write there instead of the standard output";
