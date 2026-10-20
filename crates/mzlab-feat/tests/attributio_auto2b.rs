#![cfg(feature = "attributio")]
//! The optional automatic 2b: the rows it writes, the order that lets the chemist overrule them, the origin and audit log they leave, the default (off), and
//! the false-positive rate of the `2b-strong` rule on decoys (a wrong structure explains as many fragments as a decoy list does, by chance).

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::decide::{apply_decisions, feature_keys};
use mzlab_feat::attributio::annotate::{AnnotationSettings, BasicCuration, Feature};
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::readers::feature_decisions::load_feature_decisions;
use mzlab_feat::attributio::review::auto2b::{auto_2b_table, auto_rows, with_auto};
use mzlab_feat::attributio::review::propose::{propose, ProposeInput};
use mzlab_feat::attributio::review::settings::ReviewSettings;
use mzlab_feat::attributio::Campaign;

const HEAD: &str = "pol;feature_id;mz;rt;level;name;identity;decided_on;decided_by;justification\n";

fn level3(id: i64) -> Feature {
    Feature {
        pol: "POS".into(),
        id,
        mz: Some(152.0706),
        rt: Some(3.10),
        level: 3,
        level_sub: "a".into(),
        level_basis: "putative library".into(),
        ..Feature::default()
    }
}

fn why() -> Vec<String> {
    vec!["same-structure reference: cos_vis 0.91".into()]
}

#[test]
fn only_a_strong_level_3_proposal_earns_an_automatic_2b() {
    let (a, b, c) = (level3(1), level3(2), level3(3));
    let mut d = level3(4);
    d.level_sub = "b".into();
    let w = why();
    let t = auto_rows(&[
        (&a, "2b-strong", &w),
        (&b, "2b-check", &w),
        (&c, "3-keep", &w),
        (&d, "2b-strong", &w),
    ]);
    assert_eq!(t.len(), 1);
    assert_eq!(
        (
            t.get(0, "feature_id"),
            t.get(0, "level"),
            t.get(0, "decided_by")
        ),
        ("1", "2b", "auto")
    );
    assert!(t.get(0, "justification").starts_with("auto 2b (2b-strong)"));
}

fn applied(user_rows: &str) -> Vec<Feature> {
    let w = why();
    let f = level3(15);
    let auto = auto_rows(&[(&f, "2b-strong", &w)]);
    let user = load_feature_decisions(Some(&format!("{HEAD}{user_rows}")), "d.csv").expect("table");
    let table = with_auto(&user, &auto);
    let mut feats = vec![level3(15)];
    let c = fixture("campagna_sintetica").campaign;
    let s = AnnotationSettings::for_campaign(&c).expect("settings");
    let every = feature_keys(&feats);
    let mut warnings = Vec::new();
    apply_decisions(
        &mut feats,
        &table,
        &every,
        &c,
        &s,
        &BasicCuration,
        &mut warnings,
    );
    feats
}

#[test]
fn the_automatic_2b_is_marked_audited_and_overruled_by_the_chemist() {
    let f = &applied("")[0];
    assert!(f.decision.decision_2b && f.decision.auto_2b);
    assert_eq!((f.level, f.level_sub.as_str()), (2, "b"));
    assert_eq!(f.origin(), "auto-2b");
    assert_eq!(
        (
            f.audit[0].by,
            f.audit[0].auto.as_str(),
            f.audit[0].chosen.as_str()
        ),
        ("auto", "3a", "2b")
    );

    let kept = &applied("POS;15;152.0706;3.10;3a;;;2026-10-20;t;not convincing\n")[0];
    assert!(!kept.decision.auto_2b && !kept.decision.decision_2b);
    assert_eq!(kept.level, 3);
    assert_eq!(kept.origin(), "auto");
}

#[test]
fn the_default_is_off() {
    let fx = fixture("campagna_sintetica");
    assert!(!fx.campaign.auto_2b);
    with_io!(fx, io);
    assert!(auto_2b_table(&io, "campagna_sintetica")
        .expect("table")
        .is_empty());
    let text =
        String::from_utf8(fx.files.read("campaign.toml").expect("campaign.toml")).expect("utf-8");
    let on = Campaign::from_toml(
        &format!("{text}\n[levels]\nauto_2b = true\n"),
        "campaign.toml",
        "campagna_sintetica",
        Default::default(),
    )
    .expect("campaign");
    assert!(on.auto_2b);
}

/// A deterministic stream of numbers in [0, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

fn input(ppm: f64, frac: f64, decoy: f64, cos: f64, matched: i64, iso: bool) -> ProposeInput {
    ProposeInput {
        level: 3,
        level_sub: "a".into(),
        adduct: Some("[M+H]+".into()),
        ion_ppm: ppm,
        n_expl: (frac * 10.0).round() as i64,
        n_peaks: 10,
        frac_expl: frac,
        decoy_frac: decoy,
        level_basis: "putative library".into(),
        lib_peaks: matched,
        iso_ok: Some(iso),
        ref_cos_vis: Some(cos),
        ref_matched_vis: Some(matched),
        ..ProposeInput::default()
    }
}

#[test]
fn decoys_do_not_reach_2b_strong_and_genuine_features_do() {
    let s = ReviewSettings::default();
    let strong = |r: &ProposeInput| propose(r, &s).tier == "2b-strong";
    let mut g = Lcg(20261020);
    let n = 20000;
    let (mut fp, mut tp) = (0, 0);
    for _ in 0..n {
        // a wrong structure: the fragments it explains are what a decoy list explains by chance, the reference matches at best partly
        let decoy = g.between(0.0, 0.7);
        let frac = (decoy + g.between(-0.15, 0.15)).clamp(0.0, 1.0);
        let wrong = input(
            g.between(0.0, 20.0),
            frac,
            decoy,
            g.between(0.0, 0.85),
            (g.next() * 8.0) as i64,
            g.next() < 0.7,
        );
        fp += strong(&wrong) as i32;
        // a right structure
        let right = input(
            g.between(0.0, 4.0),
            g.between(0.8, 1.0),
            g.between(0.0, 0.4),
            g.between(0.8, 1.0),
            6 + (g.next() * 6.0) as i64,
            true,
        );
        tp += strong(&right) as i32;
    }
    eprintln!("auto 2b on {n} decoys: {fp} false positives ({:.3} %); on {n} genuine: {tp} promoted ({:.1} %)", 100.0 * fp as f64 / n as f64, 100.0 * tp as f64 / n as f64);
    assert_eq!(fp, 0, "a decoy reached 2b-strong");
    assert!(
        tp as f64 / n as f64 > 0.95,
        "the rule is too strict for genuine features"
    );
}
