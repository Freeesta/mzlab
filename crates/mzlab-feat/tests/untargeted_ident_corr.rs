//! The opt-in profile correlation of `untargeted::ident` (`IdentParams::min_corr`, recipe `"ident": {"min_corr": r}`): a link by neutral mass and RT is kept only when
//! the area profiles of the two ions across the files correlate at r or more (Pearson over the files where both are detected, at least 4; fewer cannot judge,
//! the link stays). Off (0, the default) nothing changes: same links, same `Debug` text, so the recipe fingerprints do not move.

use mzlab_core::elements::ion_mz;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::untargeted::ident::{ident, IdentParams};
use mzlab_feat::untargeted::isotopes::IsoGroups;
use mzlab_feat::untargeted::pipeline::UntParams;

/// Profile of the anchor [M+H]+ across 6 files; the partner follows it (true adduct), or runs against it (an unrelated ion that only shares mass and RT).
const PROFILE: [f64; 6] = [1.0, 2.0, 4.0, 8.0, 5.0, 3.0];
const AGAINST: [f64; 6] = [8.0, 4.0, 2.0, 1.0, 2.0, 4.0];

/// Rows (m/z, RT, areas per file; 0 = not detected there).
fn table(rows: &[(f64, f64, [f64; 6])]) -> Aligned {
    let mut t = Aligned::with_capacity(6, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for v in a {
            t.area.push(*v);
            t.height.push((*v / 10.0) as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.1);
            t.state
                .push(if *v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn scaled(p: [f64; 6], k: f64) -> [f64; 6] {
    p.map(|v| v * k)
}

/// An [M+H]+ and an [M+Na]+ of the neutral mass 301.1234 at RT 4.2, the partner with the profile `q`.
fn pair(q: [f64; 6]) -> Aligned {
    let m = 301.1234;
    table(&[
        (ion_mz(m, "[M+H]+").unwrap(), 4.2, scaled(PROFILE, 1e6)),
        (ion_mz(m, "[M+Na]+").unwrap(), 4.21, scaled(q, 3e5)),
    ])
}

fn family(t: &Aligned, p: &IdentParams) -> Vec<u32> {
    ident(t, &IsoGroups::default(), 1, p).family
}

fn on(min_corr: f64) -> IdentParams {
    IdentParams {
        min_corr,
        ..IdentParams::default()
    }
}

/// Off by default: a pair whose profiles disagree is linked, as before, and `min_corr = 0` gives the very same identities.
#[test]
fn off_by_default_the_profiles_are_not_read() {
    let t = pair(AGAINST);
    assert_eq!(family(&t, &IdentParams::default()), vec![1, 1]);
    assert_eq!(
        ident(&t, &IsoGroups::default(), 1, &IdentParams::default()),
        ident(&t, &IsoGroups::default(), 1, &on(0.0))
    );
}

/// On: the pair that follows the same profile stays one identity, the one that runs against it falls apart (each ion alone, read as [M+H]+).
#[test]
fn a_link_needs_the_profiles_to_agree() {
    assert_eq!(family(&pair(PROFILE), &on(0.8)), vec![1, 1]);
    let noisy = [1.1, 1.9, 4.2, 7.6, 5.3, 2.8];
    assert_eq!(family(&pair(noisy), &on(0.8)), vec![1, 1]);
    assert_eq!(family(&pair(AGAINST), &on(0.8)), vec![0, 0]);
}

/// Fewer than 4 files where both ions are detected: the profiles cannot be judged and the link stays (also with a profile that runs against).
#[test]
fn with_few_common_files_the_link_stays() {
    let few = |keep: usize| {
        let mut q = scaled(AGAINST, 1.0);
        for v in &mut q[keep..] {
            *v = 0.0;
        }
        pair(q)
    };
    assert_eq!(family(&few(3), &on(0.8)), vec![1, 1], "3 common files");
    assert_eq!(family(&few(4), &on(0.8)), vec![0, 0], "4 common files");
}

/// The text of the parameters does not change when the option is off (it enters the fingerprint of the recipe), and shows the option when it is on.
#[test]
fn the_debug_text_is_the_old_one_when_off() {
    let off = format!("{:?}", IdentParams::default());
    assert_eq!(off, "IdentParams { rt_tol: 0.1, win: 5.0 }");
    assert_eq!(
        format!("{:#?}", IdentParams::default()),
        "IdentParams {\n    rt_tol: 0.1,\n    win: 5.0,\n}"
    );
    assert!(format!("{:?}", on(0.8)).contains("min_corr: 0.8"));
    assert!(!format!("{:?}", UntParams::default()).contains("min_corr"));
}

/// The recipe reads `ident.min_corr` (0 to 1); anything else is refused.
#[test]
fn the_recipe_sets_the_correlation() {
    let read = |j: &str| UntParams::from_json(&format!(r#"{{"engine":"untargeted","ident":{j}}}"#));
    let p = read(r#"{"min_corr":0.8}"#).unwrap();
    assert_eq!(p.iso.unwrap().1.min_corr, 0.8);
    assert_eq!(read("{}").unwrap().iso.unwrap().1.min_corr, 0.0);
    for bad in [
        r#"{"min_corr":1.5}"#,
        r#"{"min_corr":-0.1}"#,
        r#"{"min_corr":"x"}"#,
    ] {
        assert!(read(bad).is_err(), "{bad}");
    }
}
