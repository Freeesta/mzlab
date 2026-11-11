#![cfg(feature = "waters")]
//! The folder of `comune_waters`, changed one member at a time: the SRM ids carry the polarity of `_extern.inf` and the decimals of `_FUNCTNS.INF`, a scan with fewer
//! channels gives no point (not 0), and a missing member or a count two sources disagree on is named in the notices of `metodo/1`.
mod common_waters;
use common_waters::*;
use mzlab_core::readers::{Limits, Reader};

fn open(files: Vec<(String, Vec<u8>)>) -> Box<dyn Reader> {
    mzlab_lettori::open_bare(file_set(files), &Limits::sito())
        .unwrap_or_else(|e| panic!("{}", e.key))
}

/// The `dove` of every notice of the method document.
fn notices(l: &mut Box<dyn Reader>) -> Vec<String> {
    let h = l.mzml_header(0).unwrap().unwrap();
    let doc = &h
        .run_params
        .iter()
        .find(|(k, _)| k == "mzLab metodo/1")
        .unwrap()
        .1;
    let v: serde_json::Value = serde_json::from_str(doc).unwrap();
    v["avvisi"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| x["dove"].as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn replace(files: &mut [(String, Vec<u8>)], name: &str, b: Vec<u8>) {
    files.iter_mut().find(|f| f.0 == name).unwrap().1 = b;
}

const EXTERN_NEG: &str = "Function Parameters - Function 1 - TOF MS\r\nPolarity\t\t\tES+\r\n\
Function Parameters - Function 2 - MRM of 2 channels\r\nPolarity\t\t\tES-\r\n";

#[test]
fn a_negative_function_says_so_and_the_targets_are_the_label_decimals() {
    let mut f = folder_file();
    f.push(("_extern.inf".into(), EXTERN_NEG.as_bytes().to_vec()));
    let mut l = open(f);
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(
        ids.contains(
            &"SRM SIC Q1=200.1 Q3=100.2 polarity=negative function=2 offset=0".to_string()
        ),
        "{ids:?}"
    );
    let c = l.chromatogram(0, &ids[1]).unwrap();
    assert_eq!(c.q1, Some(200.1));
    assert_eq!(c.q3, Some(100.2));
}

#[test]
fn a_positive_or_unknown_polarity_adds_no_word() {
    let mut l = open(folder_file());
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids[1], "SRM SIC Q1=200.1 Q3=100.2 function=2 offset=0");
    assert!(
        notices(&mut l).iter().all(|n| !n.contains("channels")),
        "{:?}",
        notices(&mut l)
    );
}

#[test]
fn a_scan_with_fewer_channels_gives_no_point_and_is_counted() {
    let mut f = folder_file();
    // scan 2 of function 2 holds channel 1 only: same DAT, one record less in the index
    let (mut dat, mut idx) = (Vec::new(), Vec::new());
    for (k, rt) in RT.iter().enumerate() {
        let v = mrm_channels(k as u32);
        let n = if k == 1 { 1 } else { 2 };
        idx.extend(idx_row(
            dat.len() as u32,
            n,
            v[..n as usize].iter().sum::<u32>() as f32,
            *rt,
        ));
        for x in &v[..n as usize] {
            dat.extend(((21u32 << 22) | x).to_le_bytes());
        }
    }
    replace(&mut f, "_FUNC002.DAT", dat);
    replace(&mut f, "_FUNC002.IDX", idx);
    let mut l = open(f);
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    let one = l.chromatogram(0, &ids[1]).unwrap();
    let two = l.chromatogram(0, &ids[2]).unwrap();
    assert_eq!(one.time.len(), 3);
    assert_eq!(
        two.time,
        vec![f64::from(RT[0]), f64::from(RT[2])],
        "no invented 0 at the short scan"
    );
    assert_eq!(two.intensity, vec![3.0, 17.0]);
    let n = notices(&mut l);
    assert!(
        n.iter()
            .any(|x| x.ends_with("function 2: 1 of 3 scans with fewer than 2 channels")),
        "{n:?}"
    );
}

#[test]
fn channels_of_the_function_table_and_of_the_index_are_cross_checked() {
    let mut f = folder_file();
    let mut inf = function_record(0x12, 0x25, 1.0, &[(100.0, 1000.0, 0.0)]);
    inf.extend(function_record(
        0x29,
        0x01,
        1.0,
        &[
            (200.1, 100.2, 0.05),
            (300.3, 150.4, 0.05),
            (400.4, 200.5, 0.05),
        ],
    ));
    replace(&mut f, "_FUNCTNS.INF", inf);
    let mut l = open(f);
    let n = notices(&mut l);
    assert!(
        n.iter()
            .any(|x| x.ends_with("function 2: 3 channels in _FUNCTNS.INF, 2 in _FUNC002.IDX")),
        "{n:?}"
    );
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(
        ids.iter().all(|i| !i.contains("Q1=400.4")),
        "a channel without data is not shown: {ids:?}"
    );
}

#[test]
fn a_missing_member_of_a_function_is_named() {
    for gone in ["_FUNC002.DAT", "_FUNC002.IDX"] {
        let f: Vec<_> = folder_file().into_iter().filter(|x| x.0 != gone).collect();
        let mut l = open(f);
        let n = notices(&mut l);
        assert!(
            n.iter()
                .any(|x| x.ends_with(&format!("function 2: {gone} missing"))),
            "{gone}: {n:?}"
        );
    }
}

#[test]
fn a_function_of_extern_inf_past_the_function_table_is_named() {
    let mut f = folder_file();
    let t =
        format!("{EXTERN_NEG}Function Parameters - Function 3 - TOF MS\r\nPolarity\t\t\tES+\r\n");
    f.push(("_extern.inf".into(), t.into_bytes()));
    let mut l = open(f);
    let n = notices(&mut l);
    assert!(
        n.iter()
            .any(|x| x.ends_with("function 3 in _extern.inf, 2 in _FUNCTNS.INF")),
        "{n:?}"
    );
}

#[test]
fn an_ion_mode_ending_in_the_sign_gives_the_polarity() {
    let mut f = folder_file();
    let t = "Function Parameters - Function 2 - MRM of 2 channels\r\nPolarity\t\t\tESCi-\r\n";
    f.push(("_extern.inf".into(), t.as_bytes().to_vec()));
    let mut l = open(f);
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(ids[1].contains(" polarity=negative "), "{ids:?}");
}

#[test]
fn scans_past_a_dat_cut_short_are_counted_in_a_notice() {
    let mut f = folder_file();
    let dat = f.iter().find(|x| x.0 == "_FUNC001.DAT").unwrap().1.clone();
    replace(&mut f, "_FUNC001.DAT", dat[..dat.len() - 8].to_vec());
    let mut l = open(f);
    assert_eq!(l.missing(0), 1);
    let n = notices(&mut l);
    assert!(
        n.iter()
            .any(|x| x.ends_with("function 1: 1 of 3 scans past the end of _FUNC001.DAT")),
        "{n:?}"
    );
}

#[test]
fn a_missing_header_says_the_m_z_are_uncalibrated() {
    let f: Vec<_> = folder_file()
        .into_iter()
        .filter(|x| x.0 != "_HEADER.TXT")
        .collect();
    let mut l = open(f);
    let n = notices(&mut l);
    assert!(
        n.iter()
            .any(|x| x.ends_with("_HEADER.TXT missing: m/z without the calibration polynomial")),
        "{n:?}"
    );
    let mut l = open(folder_file());
    assert!(notices(&mut l).iter().all(|x| !x.contains("_HEADER.TXT")));
}
