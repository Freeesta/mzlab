//! `libmatch/1`: grouping by compound, badges, shifted peaks of an analogue, in silico libraries never exact, the Stein cosine without precursor and the
//! hybrid entropy equal to the Flash hybrid search; the JSON is validated against docs/schemi/libmatch-1.json.
mod common;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::flash::FlashMode;
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::libinfo::{Declared, Kind};
use mzlab_lib::libset::LibSet;
use mzlab_lib::matchres::{match_query, Badge, MatchParams, PairKind};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::{hybrid_entropy, ri_factor, stein_cosine, Spec, SteinOpts};
use mzlab_lib::search::{flash_scores, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};
use serde_json::Value;

fn sp(name: &str, key: &str, prec: f64, mz: &[f64], it: &[f64]) -> Spectrum {
    Spectrum {
        prec,
        pol: 1,
        mz: mz.to_vec(),
        it: it.to_vec(),
        meta: Meta {
            name: name.into(),
            inchikey: key.into(),
            accession: format!("ACC-{name}-{prec}"),
            adduct: "[M+H]+".into(),
            ..Meta::default()
        },
        rt: None,
        ri: None,
        ei: false,
    }
}

fn open(set: &[Spectrum], kind: Option<Kind>) -> Library<Vec<u8>> {
    let o = BuildOpts {
        declared: Declared {
            kind,
            ..Declared::default()
        },
        ..BuildOpts::default()
    };
    let (b, _) = build_in_memory(set, o).unwrap();
    Library::open(b, CACHE_BYTES).unwrap()
}

const KA: &str = "AAAAAAAAAAAAAA-AAAAAAAAAA-N";
const KB: &str = "BBBBBBBBBBBBBB-BBBBBBBBBB-N";
const MZ_A: [f64; 6] = [60.1, 80.2, 100.3, 120.4, 150.5, 170.6];
const IT_A: [f64; 6] = [100.0, 400.0, 900.0, 300.0, 700.0, 200.0];
const MZ_B: [f64; 6] = [70.1, 90.2, 110.3, 130.4, 180.5, 200.6];
const IT_B: [f64; 6] = [500.0, 200.0, 800.0, 300.0, 1000.0, 400.0];

fn params(mask: u16) -> MatchParams {
    let mut p = MatchParams::new(Tol::Ppm(10.0), FragTol::from(0.01), mask);
    p.lib_names = vec!["one".into(), "two".into(), "silico".into()];
    p
}

fn libs() -> LibSet<Vec<u8>> {
    let l0 = open(
        &[
            sp("Alpha", KA, 200.1, &MZ_A, &IT_A),
            sp("Beta", KB, 300.2, &MZ_B, &IT_B),
        ],
        Some(Kind::Experimental),
    );
    let it2: Vec<f64> = IT_A.iter().map(|v| v * 0.9 + 20.0).collect();
    let l1 = open(
        &[sp("alpha", KA, 200.1, &MZ_A, &it2)],
        Some(Kind::Experimental),
    );
    let l2 = open(
        &[sp(
            "Gamma",
            "CCCCCCCCCCCCCC-CCCCCCCCCC-N",
            200.1,
            &MZ_A,
            &IT_A,
        )],
        Some(Kind::InSilico),
    );
    LibSet::new(vec![l0, l1, l2]).unwrap()
}

#[test]
fn same_compound_in_two_libraries_is_one_compound_with_two_spectra() {
    let set = libs();
    let q = Query::new(200.1, 1, &MZ_A, &IT_A).unwrap();
    let r = match_query(&set, &q, &params(0b011)).unwrap();
    assert_eq!(r.compounds.len(), 1);
    let c = &r.compounds[0];
    assert_eq!(
        (c.badge, c.inchikey14.as_str()),
        (Badge::Exact, "AAAAAAAAAAAAAA")
    );
    assert_eq!(c.spectra.len(), 2);
    assert!(c.spectra[0].score >= c.spectra[1].score && c.spectra[0].score > 0.99);
    assert_eq!((c.spectra[0].matched, c.spectra[0].of), (6, 6));
}

#[test]
fn analogue_plus_oxygen_has_shifted_peaks() {
    let set = libs();
    let d = 15.9949;
    let mz: Vec<f64> = MZ_B
        .iter()
        .map(|m| if *m > 150.0 { m + d } else { *m })
        .collect();
    let q = Query::new(300.2 + d, 1, &mz, &IT_B).unwrap();
    let r = match_query(&set, &q, &params(0b001)).unwrap();
    let c = r
        .compounds
        .iter()
        .find(|c| c.name == "Beta")
        .expect("Beta found");
    assert_eq!(c.badge, Badge::Analog);
    let s = &c.spectra[0];
    assert!(
        s.shifted >= 2 && s.direct >= 4,
        "{} direct, {} shifted",
        s.direct,
        s.shifted
    );
    assert!((s.delta_mz - d).abs() < 1e-9);
    assert!(s
        .pairs
        .iter()
        .any(|p| p.kind == PairKind::Shifted && (p.query_mz - p.library_mz - d).abs() < 0.02));
}

#[test]
fn in_silico_library_is_never_exact() {
    let set = libs();
    let q = Query::new(200.1, 1, &MZ_A, &IT_A).unwrap();
    let r = match_query(&set, &q, &params(0b100)).unwrap();
    assert_eq!(r.compounds.len(), 1);
    assert_eq!(r.compounds[0].badge, Badge::InSilico);
    assert!(!r.compounds[0].experimental);
    let all = match_query(&set, &q, &params(0b111)).unwrap();
    assert_eq!(
        all.compounds[0].badge,
        Badge::Exact,
        "exact before in silico"
    );
    assert_eq!(all.compounds.last().unwrap().badge, Badge::InSilico);
}

#[test]
fn stein_cosine_finds_an_ei_spectrum_without_precursor() {
    let spec = |mz: &[f64], it: &[f64]| Spec {
        mz: mz.to_vec(),
        it: it.to_vec(),
        prec: 0.0,
    };
    let (a, b) = (spec(&MZ_A, &IT_A), spec(&MZ_B, &IT_B));
    let o = SteinOpts::default();
    let own = stein_cosine(&a, &a, 0.01, &o, None, None);
    assert!((own.score - 1.0).abs() < 1e-12 && own.pairs.len() == 6);
    let other = stein_cosine(&a, &b, 0.01, &o, None, None);
    assert_eq!((other.score, other.pairs.len()), (0.0, 0));
    // a library spectrum with an extra peak: forward (query side complete) is higher than reverse
    let mut c = a.clone();
    c.mz.push(190.7);
    c.it.push(800.0);
    let s = stein_cosine(&a, &c, 0.01, &o, None, None);
    assert!(s.forward > s.reverse && s.forward > 0.99);
    // the retention-index factor: one sigma away is exp(-1/2)
    let r = stein_cosine(&a, &a, 0.01, &o, Some(1000.0), Some(1010.0));
    assert!((r.score - (-0.5f64).exp()).abs() < 1e-12 && r.ri_factor == ri_factor(-10.0, 10.0));
}

#[test]
fn hybrid_entropy_equals_the_flash_hybrid_search() {
    let set = [
        sp("Alpha", KA, 200.1, &MZ_A, &IT_A),
        sp("Beta", KB, 300.2, &MZ_B, &IT_B),
    ];
    let lib = open(&set, None);
    let d = 15.9949;
    let mz: Vec<f64> = MZ_B
        .iter()
        .map(|m| if *m > 150.0 { m + d } else { *m })
        .collect();
    let q = Query::new(300.2 + d, 1, &mz, &IT_B).unwrap();
    let ranges = lib.ranges(1, f64::NEG_INFINITY, f64::INFINITY).unwrap();
    let flash = flash_scores(&lib, &q, FlashMode::Hybrid, 0.01, &ranges).unwrap();
    assert!(!flash.is_empty());
    for f in flash {
        let (mz, it) = lib.peaks(f.index).unwrap();
        let rec = lib.rec(f.index).unwrap();
        let (h, pairs) = hybrid_entropy(
            &q.spec,
            &Spec {
                mz,
                it,
                prec: rec.prec,
            },
            0.01,
        );
        assert_eq!(pairs.len() as u32, f.pairs);
        assert!((h - f.score).abs() < 1e-3, "{h} vs {}", f.score);
    }
}

/// The subset of JSON Schema the schema file uses: type, const, enum, required, properties, additionalProperties, items, minimum, minItems.
fn check(v: &Value, s: &Value, at: &str) {
    if let Some(c) = s.get("const") {
        assert_eq!(v, c, "{at}");
    }
    if let Some(e) = s.get("enum") {
        assert!(e.as_array().unwrap().contains(v), "{at}: {v}");
    }
    if let Some(t) = s.get("type").and_then(Value::as_str) {
        let ok = match t {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            "integer" => v.is_i64() || v.is_u64(),
            "number" => v.is_number(),
            _ => panic!("type {t}"),
        };
        assert!(ok, "{at}: {t} expected, got {v}");
    }
    if let (Some(m), Some(x)) = (s.get("minimum").and_then(Value::as_f64), v.as_f64()) {
        assert!(x >= m, "{at}: {x} < {m}");
    }
    if let (Some(m), Some(a)) = (s.get("maximum").and_then(Value::as_f64), v.as_f64()) {
        assert!(a <= m, "{at}: {a} > {m}");
    }
    if let Some(o) = v.as_object() {
        for r in s
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            assert!(o.contains_key(r.as_str().unwrap()), "{at}: missing {r}");
        }
        let props = s.get("properties").and_then(Value::as_object);
        for (k, x) in o {
            match props.and_then(|p| p.get(k)) {
                Some(ps) => check(x, ps, &format!("{at}.{k}")),
                None => assert!(
                    s.get("additionalProperties") != Some(&Value::Bool(false)),
                    "{at}: unexpected {k}"
                ),
            }
        }
    }
    if let Some(a) = v.as_array() {
        if let Some(m) = s.get("minItems").and_then(Value::as_u64) {
            assert!(a.len() as u64 >= m, "{at}: too short");
        }
        if let Some(is) = s.get("items") {
            for (i, x) in a.iter().enumerate() {
                check(x, is, &format!("{at}[{i}]"));
            }
        }
    }
}

#[test]
fn json_follows_the_schema_and_keeps_its_field_order() {
    let set = libs();
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/libmatch-1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let d = 15.9949;
    let mz: Vec<f64> = MZ_B
        .iter()
        .map(|m| if *m > 150.0 { m + d } else { *m })
        .collect();
    for (q, mask) in [
        (Query::new(200.1, 1, &MZ_A, &IT_A).unwrap(), 0b111),
        (Query::new(300.2 + d, 1, &mz, &IT_B).unwrap(), 0b001),
    ] {
        let r = match_query(&set, &q, &params(mask)).unwrap();
        let text = r.to_json();
        let v: Value = serde_json::from_str(&text).unwrap();
        check(&v, &schema, "$");
        assert!(text.starts_with("{\"schema\":\"libmatch/1\",\"hr\":true,\"mask\":"));
        assert_eq!(
            text,
            match_query(&set, &q, &params(mask)).unwrap().to_json()
        );
        assert!(!v["compounds"].as_array().unwrap().is_empty());
    }
}
