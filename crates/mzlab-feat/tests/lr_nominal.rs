//! The `nominal` preset of `features.detect` on a synthetic nominal-resolution file: a parent compound with unit-mass companions and three low-mass products (a few
//! thousand counts per second) whose centroid scatters. With the `lr` values the products are lost; with the preset they are kept, and an ion seen on both grids with a
//! centroid scatter above 0.1 Da stays one row. Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::step::find;
use mzlab_core::upload::prepare;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::series::analyse_file;
use mzlab_feat::steps::{recipe_map, registry};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
    }
}

fn file(comps: &[(f64, f64, f64, f64)], seed: u64, jit: f64, inoise: f64) -> Vec<u8> {
    file_with(comps, seed, jit, inoise, 40)
}

/// `file` with `noise` random peaks per scan (the signature tests count rows, so they take none).
fn file_with(
    comps: &[(f64, f64, f64, f64)],
    seed: u64,
    jit: f64,
    inoise: f64,
    noise: usize,
) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "n".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 270;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f32)> = (0..noise)
            .map(|_| {
                (
                    60.0 + 340.0 * rng.next(),
                    (1.5e3 * (0.8 * rng.normal()).exp()) as f32,
                )
            })
            .collect();
        for &(m, c, s, a) in comps {
            let v = a * (-0.5 * ((rt - c) / s).powi(2)).exp() * (1.0 + inoise * rng.normal());
            if v > 50.0 {
                peaks.push((m + 0.3 + jit * rng.normal(), v as f32));
            }
        }
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ p ESI Q1MS [50.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "n.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

const COMPS: [(f64, f64, f64, f64); 6] = [
    (301.10, 6.0, 0.07, 3.0e6),
    (323.08, 6.0, 0.07, 4.0e5),
    (283.09, 6.0, 0.07, 3.0e5),
    (105.03, 6.0, 0.07, 9.0e3),
    (91.05, 6.0, 0.07, 5.0e3),
    (77.04, 6.0, 0.07, 3.5e3),
];

/// The recipe of the file with the values of the preset `id` of `features.detect` written on top, as a plan does.
fn recipe(idx: &[u8], preset: Option<&str>) -> Recipe {
    let mut ix = Index::open(idx.to_vec(), idx.len() as u64).unwrap();
    let mut r = Recipe::for_index(&mut ix).unwrap();
    if let Some(id) = preset {
        let reg = registry();
        let step = find(&reg, "features.detect").unwrap();
        let p = step.presets.iter().find(|p| p.id == id).unwrap();
        let v: Vec<(String, Value)> = p
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        recipe_map::apply(&mut r, "features.detect", &v).unwrap();
    }
    r
}

/// The rows within 0.5 Da of `mz` (nominal, shifted by the calibration offset of 0.3 Da) at the retention time of the parent.
fn rows_at(idx: &[u8], r: &Recipe, mz: f64) -> usize {
    let mut ix = Index::open(idx.to_vec(), idx.len() as u64).unwrap();
    let f = analyse_file(&mut ix, r).unwrap();
    (0..f.len())
        .filter(|&i| (f.mz[i] - (mz + 0.3)).abs() <= 0.5 && (f.rt[i] - 6.0).abs() <= 0.2)
        .count()
}

#[test]
fn low_mass_products_under_the_lr_height_are_lost_and_the_preset_keeps_them() {
    let idx = file(&COMPS, 5, 0.06, 0.04);
    let lr = recipe(&idx, None);
    for mz in [91.05, 77.04] {
        assert_eq!(
            rows_at(&idx, &lr, mz),
            0,
            "the lr values drop the product at {mz}"
        );
    }
    let nominal = recipe(&idx, Some("nominal"));
    for mz in [105.03, 91.05, 77.04] {
        assert_eq!(
            rows_at(&idx, &nominal, mz),
            1,
            "the preset keeps the product at {mz}"
        );
    }
    for mz in [301.10, 323.08, 283.09] {
        assert_eq!(rows_at(&idx, &nominal, mz), 1, "{mz} stays one row");
    }
}

#[test]
fn a_scattering_centroid_splits_an_ion_unless_the_grid_merge_reaches_it() {
    let idx = file(&COMPS, 5, 0.4, 0.04);
    let lr = recipe(&idx, None);
    assert!(
        rows_at(&idx, &lr, 323.08) >= 2,
        "the default reach of 0.1 Da leaves one ion as two rows"
    );
    let nominal = recipe(&idx, Some("nominal"));
    for mz in [323.08, 91.05] {
        assert_eq!(
            rows_at(&idx, &nominal, mz),
            1,
            "the preset merges the two grids of {mz}"
        );
    }
}

#[test]
fn the_default_recipe_text_does_not_name_the_reach_and_a_changed_one_does() {
    let idx = file(&COMPS, 5, 0.06, 0.04);
    let lr = recipe(&idx, None);
    assert!(!format!("{:?}", lr.detect).contains("grid_touch"));
    assert!(format!("{:?}", recipe(&idx, Some("nominal")).detect).contains("grid_touch"));
}

/// The signature of a file without random peaks, under the `lr` recipe or, with `nominal`, under the profile.
fn signature(
    comps: &[(f64, f64, f64, f64)],
    jit: f64,
    nominal: bool,
) -> Option<mzlab_feat::nominal_hint::Signature> {
    let idx = file_with(comps, 5, jit, 0.04, 0);
    let mut ix = Index::open(idx.clone(), idx.len() as u64).unwrap();
    let mut r = Recipe::for_index(&mut ix).unwrap();
    if nominal {
        r.nominal();
    }
    mzlab_feat::nominal_hint::signature(&mut ix, &r, Default::default()).unwrap()
}

#[test]
fn low_mass_products_between_the_two_heights_give_the_signature() {
    let s = signature(&COMPS, 0.06, false).unwrap();
    // the products at 91.05 (5e3) and 77.04 (3.5e3) lie between 2.5e3 and the lr height of 7e3
    assert_eq!(s.between, 2, "{s:?}");
    assert_eq!(s.rows, 6, "{s:?}");
    assert_eq!((s.height_nominal, s.height_now), (2.5e3, 7e3));
    assert!(s.between_share() > s.cuts.between && s.hint());
}

#[test]
fn an_ion_split_across_the_grids_gives_the_signature_and_a_tidy_one_does_not() {
    let scattering = signature(&COMPS, 0.4, false).unwrap();
    assert!(scattering.split >= 1, "{scattering:?}");
    assert_eq!(
        (scattering.reach_now, scattering.reach_nominal),
        (0.1, 0.25)
    );
    // without products under the lr height and with a steady centroid there is nothing to name
    let tidy: Vec<_> = COMPS[..3].to_vec();
    let s = signature(&tidy, 0.02, false).unwrap();
    assert_eq!((s.between, s.split), (0, 0), "{s:?}");
    assert!(!s.hint());
}

#[test]
fn the_profile_in_force_has_no_signature_to_name() {
    assert!(signature(&COMPS, 0.06, true).is_none());
}
