//! `normative pack` on two invented lists: deterministic bytes, one row per skeleton with its memberships in catalogue order, pieces that inflate to the
//! raw file with matching SHA-256, and `listings` reading a folder or a `.mzsus` alike. No network.

use std::io::Read;
use std::path::{Path, PathBuf};

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Value};
use mzlab_feat::suspects::Suspects;

use super::fetch::Fetch;
use super::run_with;
use super::NormError;

/// `pack` and `listings` never touch the network: any request is a failure.
struct NoNet;

impl Fetch for NoNet {
    fn get(&self, url: &str, _limit: u64, _out: &mut dyn std::io::Write) -> Result<(), NormError> {
        panic!("network used: {url}")
    }
}

fn golden() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/normative/golden_T_EU_WATER.csv"),
    )
    .expect("fixture")
    .replace("DATE", "2000-01-02")
}

/// An input folder with the golden list and a second list that repeats its first compound and adds one of its own.
fn input(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mzlab_pack_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lists")).expect("dir");
    let g = golden();
    std::fs::write(dir.join("lists/T_EU_WATER.csv"), &g).expect("csv");
    let mut second = vec![g.lines().next().unwrap().to_string()];
    second.push(
        g.lines()
            .nth(1)
            .unwrap()
            .replace("T_EU_WATER", "T_US_RULE")
            .replace("Act 1, Annex I, no. 1", "Table 2, no. 5"),
    );
    second.push(
        "T_US_RULE;2000-01-02;2000-01-02;Act 2;Fenwick;9100-77-7;;FENWICKXXXXXAA-UHFFFAOYSA-N;FENWICKXXXXXAA;CCO;C2H6O;46.041865;MCL 4 ng/L;;;;;;;law;Table 2, no. 6;;;parent;screenable".into(),
    );
    std::fs::write(dir.join("lists/T_US_RULE.csv"), second.join("\n") + "\n").expect("csv");
    let tags = |a: &str| {
        format!(
            "{{\"jurisdiction\":\"EU\",\"version_date\":\"2000-01-01\",\"attribution\":\"{a}\"}}"
        )
    };
    std::fs::write(
        dir.join("provenance.json"),
        format!(
            "{{\"schema\":\"normative-prov/1\",\"lists\":[{{\"id\":\"T_US_RULE\",\"tags\":{},\"coverage\":{{\"entries\":2,\"screenable\":2}},\"csv_sha256\":\"b\",\"stale\":false}},\
             {{\"id\":\"T_EU_WATER\",\"tags\":{},\"coverage\":{{\"entries\":4,\"screenable\":2}},\"csv_sha256\":\"a\",\"stale\":false}}]}}",
            tags("Invented US"),
            tags("Invented EU")
        ),
    )
    .expect("prov");
    dir
}

fn pack(i: &Path, o: &Path) {
    let r = run_with(
        vec![
            "pack".into(),
            i.display().to_string(),
            o.display().to_string(),
        ],
        NoNet,
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
}

#[test]
fn pack_is_deterministic_and_complete() {
    let i = input("a");
    let (o1, o2) = (i.join("out1"), i.join("out2"));
    pack(&i, &o1);
    pack(&i, &o2);
    for f in ["normative.mzsus", "catalogo.json", "000.bin"] {
        assert_eq!(
            std::fs::read(o1.join(f)).unwrap(),
            std::fs::read(o2.join(f)).unwrap(),
            "{f}"
        );
    }
    let raw = std::fs::read(o1.join("normative.mzsus")).unwrap();
    assert_eq!(
        sha256_hex(&raw),
        "e0c2bf73a26d310124fb64ebe52fffbfc22e88777b281f69999db29a22fe61d2",
        "the bytes of `normative pack` do not change"
    );
    let s = Suspects::open(&raw).unwrap();
    // three skeletons: the two screenable ones of the golden list (the group and the unresolved rows stay out) and Fenwick; Zorbamide is on both lists
    assert_eq!(s.len(), 3);
    let l = s.listings().expect("listings");
    assert_eq!(l.list_count(), 2);
    let zorb = s.search(59.037114, 1.0)[0].clone();
    assert_eq!(&zorb.key, b"ZORBAMIDEXXXAA");
    let row = (0..s.len()).find(|&r| l.name(r) == "Zorbamide").unwrap();
    let m = l.memberships(row);
    // ids outside the catalogue come after the known ones in alphabetical order: T_EU_WATER (0) then T_US_RULE (1)
    assert_eq!(m.iter().map(|x| x.list).collect::<Vec<_>>(), vec![0, 1]);
    assert!(l.entry(m[0].entry).contains("Annex I, no. 1"));
    assert!(l.entry(m[1].entry).contains("Table 2, no. 5"));
    assert_eq!(l.full_key(row), "ZORBAMIDEXXXAA-UHFFFAOYSA-N");
    // the pieces inflate to the raw file and match the catalogue
    let cat = json::parse(&std::fs::read(o1.join("catalogo.json")).unwrap()).unwrap();
    assert_eq!(cat.get("format").and_then(Value::str), Some("normative/1"));
    assert_eq!(
        cat.get("sha256").and_then(Value::str),
        Some(sha256_hex(&raw).as_str())
    );
    let mut z = Vec::new();
    for p in cat.get("pieces").unwrap().arr() {
        let b = std::fs::read(o1.join(p.get("file").and_then(Value::str).unwrap())).unwrap();
        assert_eq!(
            p.get("sha256").and_then(Value::str),
            Some(sha256_hex(&b).as_str())
        );
        z.extend(b);
    }
    let mut back = Vec::new();
    flate2::read::ZlibDecoder::new(&z[..])
        .read_to_end(&mut back)
        .unwrap();
    assert_eq!(back, raw);
    assert_eq!(cat.get("attributions").unwrap().arr().len(), 2);
    let _ = std::fs::remove_dir_all(i);
}

#[test]
fn listings_reads_folder_and_file() {
    let i = input("b");
    let o = i.join("out");
    pack(&i, &o);
    let by_dir = run_with(vec!["listings".into(), o.display().to_string()], NoNet);
    let by_file = run_with(
        vec![
            "listings".into(),
            o.join("normative.mzsus").display().to_string(),
        ],
        NoNet,
    );
    assert_eq!((by_dir.code, by_file.code), (0, 0));
    assert_eq!(by_dir.stdout, by_file.stdout);
    assert!(String::from_utf8(by_dir.stdout)
        .unwrap()
        .contains("\"Fenwick\""));
    let bad = run_with(vec!["pack".into(), i.display().to_string()], NoNet);
    assert_eq!(bad.code, 2);
    let _ = std::fs::remove_dir_all(i);
}
