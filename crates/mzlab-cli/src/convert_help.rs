//! The `--help` text of `mzlab-cli convert`.

pub(crate) const HELP: &str = "mzlab-cli convert: vendor files to indexed mzML, the step convert.mzml of the Flow page.

usage: mzlab-cli convert INPUT... --out FOLDER [--sample LIST] [--compression KIND] [--zlib N] [--numpress] [--peaks WHICH] [--workers N] [--report FILE.json] [--fresh] [--set conv.NAME=VALUE...]

  INPUT...             a file, a maker's folder (.D, .d, .raw) or a folder to look through; a .wiff is taken with its .wiff.scan, and a dataset
                       with several samples gives one mzML per sample
  --out FOLDER         where the mzML files are written: NAME.mzML (NAME-ID.mzML for the sample ID of a dataset with several); the cache is in
                       FOLDER/.convert
  --sample LIST        only these samples of a dataset with several (.wiff): ids and ranges, 1,3,5 or 2-4; the others are not read. The mzML of a
                       sample is the same bytes as in a run without it. An id the dataset does not have stops the run (exit 5); a dataset with one
                       sample is converted as it is. `mzlab-cli samples select FILE --sample LIST` checks a list without converting
  --compression KIND   standard (zlib level 2, default), smallest (zlib and MS-Numpress) or none (uncompressed): the short form of the two options below
  --zlib N, --numpress zlib level of the arrays, 0 (none) to 9 (smallest), default 2; MS-Numpress coding of m/z and intensities
  --peaks WHICH        the peaks written: file (those of the file, default; a profile file keeps its points and the run warns), profile (the
                       points of the profile) or mzlab (centroids of mzLab)
  --workers N          files converted at once (default: the cores but one, 1 to 4, fewer when the memory budget of the computer does not give
                       each its share, with a note; the bytes written do not depend on it)
  --report FILE.json   one object per input: input, output, sha256, spectra, seconds, skipped, conv/1, warnings (centroids with --peaks mzlab); or input, error, cause
  --fresh              converts every file again; without it a file already converted from the same bytes, with the same parameters and the same
                       program is not converted again
  --set conv.NAME=V    a parameter of the step convert.mzml (zlib_level, numpress, peaks, dad); the options above are the short forms
  --help               this text

progress goes to stderr (one line per file), stdout lists the mzML files; exit codes: 0 done, 1 a file failed (the others are written), 2 usage, 3 a file was written with scans missing (conv/1 parziale; a
warning line names it), 4 an input is missing or holds nothing to convert, 5 a --sample id the dataset does not have";
