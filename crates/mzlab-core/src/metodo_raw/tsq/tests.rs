use super::*;

// The layout of the method summaries of three models (names invented, numbers in the style of the real files: comma as thousands separator, closing bar).
const ALTIS: &str = "TSQ Altis Method Summary\r\n  \r\nGlobal Settings\r\n\tMethod Duration (min)= 32\r\n\r\nExperiment Type: SRM\r\nUse Cycle Time: True\r\nCycle Time (sec): 0.600\r\nChromatographic Peak Width (sec): 6.0\r\nData Mode: Centroid\r\nCollision Gas Pressure (mTorr): 1.5\r\nQ1 Resolution (FWHM): 0.7\r\nQ3 Resolution (FWHM): 1.2\r\n\r\n\r\nCompound Name | Start Time (min) | End Time (min) | Polarity | Precursor (m/z) | Product (m/z) | Collision Energy (V) | Min Dwell Time (ms) | \r\nPeptide A(+3) |            8.130 |         11.130 | Positive |         549.272 |       246.108 |               18.200 |               2.598 | \r\n  Peptide A(+3) |            8.130 |         11.130 | Positive |         549.272 |     1,058.548 |               24.200 |               2.598 | \r\n\r\nNext section\r\n";
const QUANTIVA: &str = "Experiment Type: SRM\nCompound Name | Start Time (min) | End Time (min) | Polarity | Precursor (m/z) | Product (m/z) | Collision Energy (V) | RF Lens | \n  Glycine |            0.000 |          0.800 | Negative |          76.039 |        30.190 |               12.960 |      46.000 | \n";
const NO_TABLE: &str = "Experiment Type: SRM\nCycle Time (sec): 0.5\n";

#[test]
fn the_altis_table_and_settings_are_read() {
    let t = parse(ALTIS).unwrap();
    assert_eq!(t.kind.as_deref(), Some("SRM"));
    assert_eq!(t.rows.len(), 2);
    let r = &t.rows[1];
    assert_eq!(r.name, "Peptide A(+3)");
    assert_eq!((r.q1, r.q3), (549.272, 1058.548));
    assert_eq!(
        (r.ce_ev, r.dwell_ms, r.rf_lens_v),
        (Some(24.2), Some(2.598), None)
    );
    assert_eq!(
        (r.start_min, r.end_min, r.polarity),
        (Some(8.13), Some(11.13), Some("positive"))
    );
    let get = |id: &str| {
        t.settings
            .iter()
            .find(|s| s.0 == id)
            .map(|s| (s.2, s.3.clone()))
    };
    assert_eq!(get("cycle_time"), Some((0.6, "sec".into())));
    assert_eq!(get("collision_gas_pressure"), Some((1.5, "mTorr".into())));
    assert_eq!(get("q1_resolution"), Some((0.7, "FWHM".into())));
    // `Use Cycle Time: True` and `Data Mode: Centroid` are not numbers
    assert!(get("use_cycle_time").is_none() && get("data_mode").is_none());
}

#[test]
fn the_quantiva_columns_are_found_by_name() {
    let t = parse(QUANTIVA).unwrap();
    let r = &t.rows[0];
    assert_eq!((r.q1, r.q3, r.polarity), (76.039, 30.19, Some("negative")));
    assert_eq!(
        (r.rf_lens_v, r.dwell_ms, r.ce_ev),
        (Some(46.0), None, Some(12.96))
    );
}

#[test]
fn a_text_without_a_table_gives_none() {
    assert!(parse(NO_TABLE).is_none());
    assert!(parse("").is_none());
    // a header without the two m/z columns is not a transition table
    assert!(parse("A | Precursor | B\n1 | 2 | 3 |\n").is_none());
}

#[test]
fn the_experiment_carries_the_transitions() {
    let t = parse(ALTIS).unwrap();
    let e = experiment(&t, 1).unwrap().json();
    assert!(e.contains("\"tipo\":\"SRM\""));
    assert!(e.contains("\"q1\":549.272,\"q3\":246.108,\"dwell_ms\":2.598,\"ce_eV\":18.2,"));
    assert!(e.contains("\"polarita\":\"positive\",\"t_inizio_min\":8.13,\"t_fine_min\":11.13,\"nome\":\"Peptide A(+3)\""));
    assert!(e.contains(
        "\"id\":\"cycle_time\",\"etichetta\":\"Cycle Time\",\"valore\":0.6,\"unita\":\"sec\""
    ));
}

#[test]
fn a_torn_header_before_the_real_one_is_skipped() {
    let t = format!("tart Time (min) | End Time (min) | Precursor (m/z) | Product (m/z) | TSQ Quantiva Method Summary\n\n{QUANTIVA}");
    assert_eq!(parse(&t).unwrap().rows.len(), 1);
}

#[test]
fn a_table_cut_inside_a_row_is_told() {
    let cut = format!("{QUANTIVA}  Alanine |  0.000 |  0.800 | Negative |  90.055 |  44.0");
    let t = parse(&cut).unwrap();
    assert!(t.cut && t.rows.len() == 1);
    assert!(!parse(QUANTIVA).unwrap().cut);
}

/// Junk and torn texts never panic and never invent a section.
#[test]
fn hostile_texts_do_not_panic() {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let words = [
        "Lock Masses:",
        "Pos List Name:",
        "Mass List:",
        "Use lock masses",
        "UV_VIS_1.Wavelength =",
        "3DFIELD.MinWavelength =",
        "1e999",
        "NaN",
        "|",
        "\r\n",
        "[",
        "=",
        "Precursor (m/z)",
        "Product (m/z)",
    ];
    for _ in 0..500 {
        let mut t = String::new();
        for _ in 0..40 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            t.push_str(words[(x >> 33) as usize % words.len()]);
            t.push(if x & 1 == 0 { ' ' } else { '\n' });
        }
        let _ = parse(&t);
        let _ = announces_transitions(&t);
    }
}
