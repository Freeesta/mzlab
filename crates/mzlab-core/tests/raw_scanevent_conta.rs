//! A Fusion MSn chain (scan events that grow with the reaction count, up to MS8) is read with the right level of every scan and the whole chain of
//! precursors. Oracle: the counts of ThermoRawFileParser on the same infusion. Skipped when the file is missing (`MZLAB_RAW_MSN`, or
//! `MZLAB_DATI/dati/infusioni/`): the file stays on the external disk and no compound name is kept here.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use opentfraw::mzml::extract_spectrum;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("MZLAB_RAW_MSN").map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    let dir = dati_percorsi::percorso("dati/infusioni")?;
    let p = dir.join("Infusione_Timololo_19_3_2024.raw");
    p.is_file().then_some(p)
}

#[test]
fn msn_chain_scans_keep_their_level_and_precursors() {
    let Some(path) = file() else {
        eprintln!("SKIP raw_scanevent_conta: no MSn infusion file");
        return;
    };
    let mut src = BufReader::new(File::open(&path).unwrap());
    let raw = opentfraw::RawFileReader::open(&mut src).unwrap();
    assert_eq!(raw.num_scans, 9000);
    let mut levels: BTreeMap<u32, u32> = BTreeMap::new();
    let mut deepest = String::new();
    for idx in 0..raw.num_scans {
        let rec = extract_spectrum(&raw, &mut src, idx, false).unwrap();
        *levels.entry(rec.ms_level).or_default() += 1;
        let filter = rec.filter.clone().unwrap_or_default();
        assert!(filter.starts_with("FTMS + p ESI"), "scan {idx}: {filter}");
        if rec.ms_level == 8 && deepest.is_empty() {
            deepest = filter;
        }
    }
    let want: BTreeMap<u32, u32> = [
        (1, 90),
        (2, 277),
        (3, 392),
        (4, 1940),
        (5, 2383),
        (6, 2880),
        (7, 760),
        (8, 278),
    ]
    .into();
    assert_eq!(levels, want);
    let mut at = 0;
    for p in [
        "317.0000@",
        "261.0000@",
        "244.0000@",
        "200.0000@",
        "172.0000@",
        "145.0000@",
        "112.0000@",
    ] {
        at += deepest[at..]
            .find(p)
            .unwrap_or_else(|| panic!("{p} missing after {at} in {deepest}"))
            + p.len();
    }
}
