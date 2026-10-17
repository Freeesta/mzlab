// Integration and verification tests for the HRMS target screening engine.
// Tests synthetic chromatograms with known substances, prediction proposal levels
// ("Prev. 1", "Prev. 2", "Prev. 3"), decoy spectra for zero false positives, and result sorting.

use std::fs::File;
use std::path::PathBuf;

use mzlab_core::mzml::Spectrum;
use mzlab_target::{
    cerca_targets, parse_target_csv, AdductType, CheckFlags, TargetEntry, TargetList,
    TargetSearchOptions,
};

/// Helper to create a single target entry.
fn make_entry(
    id: &str,
    name_en: &str,
    neutral_formula: &str,
    monoisotopic_mass: f64,
) -> TargetEntry {
    let mz_mh = monoisotopic_mass + AdductType::MH.delta_mass();
    let mz_mna = monoisotopic_mass + AdductType::MNa.delta_mass();
    let mz_mnh4 = monoisotopic_mass + AdductType::MNH4.delta_mass();

    TargetEntry {
        id: id.to_string(),
        name_en: name_en.to_string(),
        name_it: name_en.to_string(),
        neutral_formula: neutral_formula.to_string(),
        monoisotopic_mass,
        mz_mh,
        mz_mna,
        mz_mnh4,
        smiles: String::new(),
        inchikey: String::new(),
        inchikey_connectivity: String::new(),
        cas: None,
        pubchem_cid: None,
        effect_class: "Test".to_string(),
        chemical_class: "Test".to_string(),
        role: "Target".to_string(),
        psychoactive: false,
        synonyms: Vec::new(),
        check: None,
        check_flags: CheckFlags::default(),
    }
}

/// Specifications for a synthetic chromatographic peak in memory.
struct SyntheticPeakSpec {
    target_mz: f64,
    rt_apex: f64,
    sigma: f64,
    height: f64,
    include_m1: bool,
    m1_ratio: f64,
}

/// Generate a synthetic chromatogram with MS1 scans and optional MS2 scan.
fn generate_chromatogram(
    peaks: &[SyntheticPeakSpec],
    ms2_precursors: &[(f64, f64)],
    baseline_noise: f64,
) -> Vec<Spectrum> {
    let mut spectra = Vec::new();
    let num_scans = 61;
    let rt_start = 1.0;
    let rt_step = 0.1;

    for i in 0..num_scans {
        let rt = rt_start + (i as f64) * rt_step;
        let mut mzs = Vec::new();
        let mut intensities = Vec::new();

        // Background noise ions far from targets
        mzs.push(115.05);
        intensities.push(baseline_noise as f32);
        mzs.push(350.20);
        intensities.push((baseline_noise * 0.8) as f32);
        mzs.push(580.45);
        intensities.push((baseline_noise * 1.2) as f32);

        // Add chromatographic peaks
        for p in peaks {
            let dt = rt - p.rt_apex;
            let gauss = (-0.5 * (dt / p.sigma).powi(2)).exp();
            let inten = p.height * gauss;
            if inten >= 5.0 {
                mzs.push(p.target_mz);
                intensities.push(inten as f32);

                if p.include_m1 {
                    let m1_mz = p.target_mz + 1.003355;
                    let m1_inten = inten * p.m1_ratio;
                    mzs.push(m1_mz);
                    intensities.push(m1_inten as f32);
                }
            }
        }

        spectra.push(Spectrum {
            index: i,
            id: format!("scan_{}", i),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "FTMS + p ESI Full ms".to_string(),
            precursor: None,
            mz: mzs,
            intensity: intensities,
            wide: None,
        });
    }

    // Add MS2 spectra if specified
    for (idx, &(ms2_rt, prec_mz)) in ms2_precursors.iter().enumerate() {
        spectra.push(Spectrum {
            index: num_scans + idx,
            id: format!("ms2_scan_{}", idx),
            level: 2,
            rt: ms2_rt,
            polarity: 1,
            profile: false,
            filter: format!("FTMS + p ESI d Full ms2 {:.4}@hcd", prec_mz),
            precursor: Some(prec_mz),
            mz: vec![prec_mz - 58.0, prec_mz - 18.0],
            intensity: vec![8000.0, 15000.0],
            wide: None,
        });
    }

    spectra
}

#[test]
fn test_target_found_prev1_with_ms2_and_isotope() {
    let caffeine = make_entry("caffeine", "Caffeine", "C8H10N4O2", 194.080376);
    let list = TargetList::from_entries(vec![caffeine]);

    let target_mz = 194.080376 + AdductType::MH.delta_mass(); // 195.087652
    let peak_spec = SyntheticPeakSpec {
        target_mz,
        rt_apex: 4.20,
        sigma: 0.12,
        height: 50_000.0,
        include_m1: true,
        m1_ratio: 0.105,
    };

    let spectra = generate_chromatogram(&[peak_spec], &[(4.20, target_mz)], 50.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(matches.len(), 1, "Expected exactly 1 match for caffeine");
    let m = &matches[0];
    assert_eq!(m.id, "caffeine");
    assert_eq!(m.name_en, "Caffeine");
    assert_eq!(m.neutral_formula, "C8H10N4O2");
    assert_eq!(m.adduct, AdductType::MH);
    assert_eq!(m.proposal, "Prev. 1", "Expected Prev. 1 with full evidence");
    assert!(m.ms2_matched, "MS2 precursor should be confirmed");
    assert!(m.sn_ratio >= 10.0, "S/N should be high");
    assert!(
        m.isotope_score.unwrap_or(0.0) >= 0.70,
        "Isotope score should be >= 0.70"
    );
    assert!((m.rt_apex - 4.20).abs() < 0.05);
    assert!(m.motif.contains("Peak at RT 4.20 min"));
    assert!(m.motif.contains("MS2 precursor matched"));
    assert!(m.motif.contains("isotope pattern confirmed"));
}

#[test]
fn test_target_found_prev2_without_ms2() {
    let caffeine = make_entry("caffeine", "Caffeine", "C8H10N4O2", 194.080376);
    let list = TargetList::from_entries(vec![caffeine]);

    let target_mz = 194.080376 + AdductType::MH.delta_mass();
    let peak_spec = SyntheticPeakSpec {
        target_mz,
        rt_apex: 3.80,
        sigma: 0.10,
        height: 40_000.0,
        include_m1: true,
        m1_ratio: 0.105,
    };

    // No MS2 precursor added
    let spectra = generate_chromatogram(&[peak_spec], &[], 50.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(matches.len(), 1);
    let m = &matches[0];
    assert_eq!(m.id, "caffeine");
    assert_eq!(m.proposal, "Prev. 2", "Expected Prev. 2 without MS2");
    assert!(!m.ms2_matched, "MS2 should not be matched");
    assert!(m.isotope_score.unwrap_or(0.0) >= 0.70);
    assert!(m.motif.contains("no MS2 precursor matched"));
}

#[test]
fn test_target_found_prev3_without_isotope_and_ms2() {
    let caffeine = make_entry("caffeine", "Caffeine", "C8H10N4O2", 194.080376);
    let list = TargetList::from_entries(vec![caffeine]);

    let target_mz = 194.080376 + AdductType::MH.delta_mass();
    let peak_spec = SyntheticPeakSpec {
        target_mz,
        rt_apex: 5.10,
        sigma: 0.10,
        height: 15_000.0,
        include_m1: false,
        m1_ratio: 0.0,
    };

    // No isotope M+1 and no MS2
    let spectra = generate_chromatogram(&[peak_spec], &[], 100.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(matches.len(), 1);
    let m = &matches[0];
    assert_eq!(m.id, "caffeine");
    assert_eq!(
        m.proposal, "Prev. 3",
        "Expected Prev. 3 when isotope unconfirmed"
    );
    assert!(!m.ms2_matched);
    assert!(m.motif.contains("isotope pattern unconfirmed"));
}

#[test]
fn test_decoy_dataset_produces_zero_false_positives() {
    let caffeine = make_entry("caffeine", "Caffeine", "C8H10N4O2", 194.080376);
    let paracetamol = make_entry("paracetamol", "Paracetamol", "C8H9NO2", 151.063329);
    let cocaine = make_entry("cocaine", "Cocaine", "C17H21NO4", 303.147058);
    let list = TargetList::from_entries(vec![caffeine, paracetamol, cocaine]);

    // Synthetic dataset with ONLY noise and decoy peaks far away from all target m/z
    let decoy_peak = SyntheticPeakSpec {
        target_mz: 489.1234, // Far from targets 195, 152, 304
        rt_apex: 3.50,
        sigma: 0.15,
        height: 60_000.0,
        include_m1: true,
        m1_ratio: 0.20,
    };

    let spectra = generate_chromatogram(&[decoy_peak], &[(3.50, 489.1234)], 40.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(
        matches.len(),
        0,
        "Decoy dataset must produce 0 false positives"
    );
}

#[test]
fn test_decoy_targets_produce_zero_matches() {
    // Target list containing only decoy molecules not present in data
    let decoy1 = make_entry("decoy_1", "Decoy Molecule 1", "C22H28N2O3", 368.2100);
    let decoy2 = make_entry("decoy_2", "Decoy Molecule 2", "C30H40O8", 528.2723);
    let list = TargetList::from_entries(vec![decoy1, decoy2]);

    // Dataset has caffeine at 195.0877
    let caffeine_mz = 194.080376 + AdductType::MH.delta_mass();
    let peak_spec = SyntheticPeakSpec {
        target_mz: caffeine_mz,
        rt_apex: 4.0,
        sigma: 0.10,
        height: 50_000.0,
        include_m1: true,
        m1_ratio: 0.10,
    };

    let spectra = generate_chromatogram(&[peak_spec], &[(4.0, caffeine_mz)], 50.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(
        matches.len(),
        0,
        "Decoy targets not in data must yield 0 matches"
    );
}

#[test]
fn test_proposal_and_sn_sorting_order() {
    let mol_a = make_entry("substance_a", "Substance A", "C8H10N4O2", 194.080376);
    let mol_b = make_entry("substance_b", "Substance B", "C9H13NO", 151.099714);
    let mol_c = make_entry("substance_c", "Substance C", "C10H15N", 149.120449);
    let list = TargetList::from_entries(vec![mol_a, mol_b, mol_c]);

    let mz_a = 194.080376 + AdductType::MH.delta_mass();
    let mz_b = 151.099714 + AdductType::MH.delta_mass();
    let mz_c = 149.120449 + AdductType::MH.delta_mass();

    // A: Prev. 1 (MS1 + M1 + MS2)
    let peak_a = SyntheticPeakSpec {
        target_mz: mz_a,
        rt_apex: 2.5,
        sigma: 0.10,
        height: 30_000.0,
        include_m1: true,
        m1_ratio: 0.10,
    };
    // B: Prev. 2 (MS1 + M1, no MS2)
    let peak_b = SyntheticPeakSpec {
        target_mz: mz_b,
        rt_apex: 4.0,
        sigma: 0.10,
        height: 60_000.0,
        include_m1: true,
        m1_ratio: 0.10,
    };
    // C: Prev. 3 (MS1 only, no M1, no MS2)
    let peak_c = SyntheticPeakSpec {
        target_mz: mz_c,
        rt_apex: 5.5,
        sigma: 0.10,
        height: 20_000.0,
        include_m1: false,
        m1_ratio: 0.0,
    };

    let spectra = generate_chromatogram(&[peak_a, peak_b, peak_c], &[(2.5, mz_a)], 50.0);
    let opts = TargetSearchOptions {
        adducts: vec![AdductType::MH],
        ..Default::default()
    };

    let matches = cerca_targets(&list, &spectra, &opts);
    assert_eq!(matches.len(), 3);

    assert_eq!(matches[0].id, "substance_a");
    assert_eq!(matches[0].proposal, "Prev. 1");

    assert_eq!(matches[1].id, "substance_b");
    assert_eq!(matches[1].proposal, "Prev. 2");

    assert_eq!(matches[2].id, "substance_c");
    assert_eq!(matches[2].proposal, "Prev. 3");
}

#[test]
fn test_real_645_substances_with_decoy_data() {
    let csv_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/liste/sostanze_hrms.csv");
    if !csv_path.exists() {
        return;
    }

    let file = File::open(&csv_path).expect("Failed to open real 645 substance CSV");
    let entries = parse_target_csv(file).expect("Failed to parse target CSV");
    assert_eq!(entries.len(), 645);
    let list = TargetList::from_entries(entries);

    // Generate dataset containing only random non-target peaks (e.g. at 1200.50, 1350.70)
    let decoy_peak = SyntheticPeakSpec {
        target_mz: 1250.60,
        rt_apex: 3.0,
        sigma: 0.12,
        height: 80_000.0,
        include_m1: true,
        m1_ratio: 0.50,
    };

    let spectra = generate_chromatogram(&[decoy_peak], &[(3.0, 1250.60)], 50.0);
    let opts = TargetSearchOptions::default();
    let matches = cerca_targets(&list, &spectra, &opts);

    assert_eq!(
        matches.len(),
        0,
        "Decoy data against 645 substances must produce exactly 0 false positives"
    );
}
