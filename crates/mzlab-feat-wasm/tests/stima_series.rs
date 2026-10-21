//! `stima_json` (behind the export `stima_series`) reports the way and the memory that `slabs::stima` gives for the same series, and the page's budget
//! decides between memory, slabs and refusal. The files are synthetic `.mzfeatf` of a few features each.
#![cfg(feature = "detect")]

use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{rows_of, stima};
use mzlab_feat::store::{write_file, Origin};
use mzlab_feat::table::FileFeatures;
use mzlab_feat_wasm::stima::stima_json;

fn file(rows: usize) -> Vec<u8> {
    let mut f = FileFeatures {
        dt: 1.5,
        ..Default::default()
    };
    for i in 0..rows {
        f.mz.push(150.0 + i as f64 * 0.37);
        f.rt.push(1.0 + (i % 23) as f32 * 0.4);
        f.left.push(0.9);
        f.right.push(1.1);
        f.apex.push(i as u32);
        f.scan_lo.push(0);
        f.scan_hi.push(1);
        f.area.push(1e6);
        f.height.push(1e5);
        f.npts.push(9);
        f.flags.push(16);
        f.iso_group.push(0);
        f.charge.push(0);
        for v in [
            &mut f.shape,
            &mut f.snr,
            &mut f.fwhm,
            &mut f.asym,
            &mut f.purity,
        ] {
            v.push(0.5);
        }
        f.gaps.push(0);
        f.qreason.push(0);
        f.ms2_n.push(0);
        f.area_win.push(1.0);
        f.family_id.push(0);
        f.role.push(1);
        f.alt.push(0);
    }
    write_file(&f, &Recipe::default(), &Origin::default())
}

#[test]
fn the_report_is_the_estimate_of_the_slabs_module() {
    let mut series = vec![file(300), file(200), file(250)];
    let r = Recipe::default();
    let rows = rows_of(&mut series).unwrap();
    assert_eq!(rows, [300, 200, 250]);
    let want = stima(&rows, &r, 192 << 20);
    let got = stima_json(&mut series, &r, 192.0).unwrap();
    assert_eq!(
        got,
        format!(
            "{{\"mb\":{},\"via\":\"{}\",\"files\":3,\"rows\":750}}",
            want.bytes.div_ceil(1 << 20),
            want.via.name()
        )
    );
    assert!(got.contains("\"memory\""));
}

#[test]
fn a_budget_that_holds_nothing_says_refused() {
    let mut series = vec![file(300), file(200)];
    let got = stima_json(&mut series, &Recipe::default(), 0.001).unwrap();
    assert!(got.contains("\"refused\""), "{got}");
}
