//! The sentence templates of `report.methods@1`: one entry per step (and per algorithm where the method differs), the only place that holds the words.
//! `{name}` is a parameter of the step as it ran; `{algorithm}` and `{preset}` are the choices of the instance. A step without an entry gets the generic sentence of `render`.

/// `(step id, algorithm id or "" for any, sentence)`. The lookup tries the algorithm first.
pub(super) const TEMPLATES: &[(&str, &str, &str)] = &[
    ("import.raw", "", "Vendor files were recognised and checked against the limits of the platform, without any change."),
    ("convert.mzml", "", "Vendor files were converted to mzML (peaks: {peaks}, zlib level {zlib_level}, Numpress {numpress}, diode-array traces {dad})."),
    ("index.mzidx", "", "Each mzML file was indexed for random access (memory budget {budget} bytes)."),
    ("spectra.centroid", "", "Profile spectra were centroided (FWHM {fwhm}, threshold {threshold})."),
    ("features.estimate", "", "The detection parameters were estimated from the data (polarity {polarity}, mass axis {axis})."),
    ("features.detect", "centwave", "Features were detected with the centWave algorithm (polarity {polarity}, mass axis {axis}, intensity floor {floor}, minimum height {min_height}, minimum width {min_width}, minimum S/N {min_snr}, minimum points {min_points})."),
    ("features.detect", "masstrace", "Features were detected by building mass traces (polarity {polarity}, mass axis {axis}, intensity floor {floor}, minimum height {min_height}, minimum points {min_points}, maximum gap {max_gap} scans)."),
    ("features.detect", "threshold", "Features were detected by intensity thresholding (polarity {polarity}, mass axis {axis}, intensity floor {floor}, minimum height {min_height})."),
    ("features.detect", "", "Features were detected (polarity {polarity}, mass axis {axis}, intensity floor {floor}, minimum height {min_height}, minimum points {min_points})."),
    ("features.align", "", "Features were aligned across the files in {mode} mode (m/z tolerance {ppm} ppm, RT gap {rt_gap} min, RT warping {rt_warp})."),
    ("features.gapfill", "", "Missing values were filled from the raw data (intensity floor {floor}, m/z tolerance {ppm} ppm, RT tolerance {rt_tol} min, minimum points {min_points})."),
    ("table.filter", "", "The feature table was filtered (minimum detections {min_detected}, minimum area {min_area}, blank ratio {blank_ratio}, maximum QC CV {qc_cv_max}, normalisation {normalize})."),
    ("table.drift", "", "The drift of the areas was corrected on the QC injections of each batch with a {method} curve (LOESS span {span})."),
    ("table.stats", "", "Per-feature statistics were computed (floor {floor}, pseudocount {pseudocount})."),
    ("table.pca", "", "A principal component analysis was run ({ncomp} components, {transform} transform, {scaling} scaling, minimum detections {min_detect})."),
    ("table.kmd", "", "Series of the {unit} unit were searched on the Kendrick mass defect (tolerance {tol_mda} mDa, at least {min_members} members)."),
    ("table.plausible", "", "Plausibility checks were applied to the features (m/z {mz_min} to {mz_max}, RT up to {rt_max}, width up to {width_max}, ppm up to {ppm_max})."),
    ("spectra.ms2", "", "MS2 spectra were collected (minimum intensity {min_intensity}, m/z tolerance {tol_da} Da, RT tolerance {rt_tol} min)."),
    ("formula.ms1", "", "Formula candidates were listed from the MS1 data (ion {ion}, tolerance {tol_ppm} ppm, at most {max_results} candidates, ring and double bond equivalents at least {rdb_min})."),
    ("formula.ms2", "", "Fragment formulas were assigned from the MS2 data (precursor tolerance {ppm_ion} ppm, fragment tolerance {ppm_frag} ppm)."),
    ("spectra.match", "", "Spectra were matched against the library with the {algorithm} score (fragment tolerance {tol_da} Da, precursor tolerance {precursor_ppm} ppm, top {top_n} hits per spectrum)."),
    ("annot.level", "", "Identification levels were proposed by the cascade rules (minimum spectral similarity {spec_sim_min}, minimum matched peaks {spec_peaks_min})."),
    ("screen.suspects", "", "The features were screened against the suspect list (tolerance {ppm} ppm, adducts {adducts})."),
    ("qc.mass_accuracy", "", "The mass accuracy was checked on reference ions (search window {search_ppm} ppm, minimum intensity {min_intensity}, minimum references {min_refs})."),
    ("qc.run_order", "", "The injection sequence was checked (at most {qc_every_max} injections between QC injections, at least {std_levels_min} standard levels, blank in every batch: {blank_per_batch})."),
    ("export.table_csv", "", "The feature table was exported as a CSV file ({kind} layout)."),
    ("export.mztab", "", "The feature table was exported as mzTab-M 2.0."),
    ("report.methods", "", "This methods text was written from the record of the run."),
];
