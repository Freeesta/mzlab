//! `mzlab-cli normative build` against invented fixtures served by a fake `Fetch`: canonical bytes, resolution steps, pins, stale lists and warm-cache
//! rebuilds. No network; the fixtures in `tests/data/normative/` are invented (CAS-shaped numbers that fail the checksum, InChIKey-shaped strings).
#![cfg(not(target_family = "wasm"))]

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mzlab_cli::normative::fetch::Fetch;
use mzlab_cli::normative::{run_with, NormError};
use mzlab_feat::attributio::reglists::schema::read_list;

fn data(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/normative")
            .join(name),
    )
    .expect("fixture")
}

fn gz(b: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(b).expect("gzip");
    e.finish().expect("gzip")
}

/// The fake network: URL → body, with the count of requests.
#[derive(Clone, Default)]
struct Fake {
    pages: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    fn standard() -> Fake {
        let f = Fake::default();
        for (u, n) in [
            ("cellar", "cellar.xhtml"),
            ("ecfr", "ecfr.xml"),
            ("ppp?page=1", "ppp_1.json"),
            ("ppp?page=2", "ppp_2.json"),
            ("susdat", "susdat.csv"),
            ("epa", "epa.html"),
            ("voci", "voci.tsv"),
            ("dsstox", "dsstox.csv"),
            ("props/9101,9102", "pug_props.csv"),
        ] {
            f.set(u, data(n));
        }
        f.set("ids", gz(&data("cid_identifiers.tsv")));
        f.set("parent", gz(&data("cid_parent.tsv")));
        f
    }
    fn set(&self, u: &str, b: Vec<u8>) {
        self.pages
            .lock()
            .expect("lock")
            .insert(format!("https://fixture.test/{u}"), b);
    }
    fn empty() -> Fake {
        Fake::default()
    }
    fn count(&self) -> usize {
        self.calls.lock().expect("lock").len()
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str, _limit: u64, out: &mut dyn Write) -> Result<(), NormError> {
        self.calls.lock().expect("lock").push(url.into());
        let p = self.pages.lock().expect("lock");
        let b = p.get(url).ok_or_else(|| NormError::fetch(url, "404"))?;
        out.write_all(b)
            .map_err(|e| NormError::fetch(url, &e.to_string()))
    }
}

struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("mzlab_norm_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("out")).expect("dir");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/normative/sources.json"),
            dir.join("out/sources.json"),
        )
        .expect("sources");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/data/normative/curated_overrides.tsv"),
            dir.join("out/overrides.tsv"),
        )
        .expect("overrides");
        Work { dir }
    }
    fn args(&self, extra: &[&str]) -> Vec<String> {
        let mut a: Vec<String> = ["build", "--cache"].iter().map(|s| s.to_string()).collect();
        a.push(self.dir.join("cache").display().to_string());
        a.push("--out".into());
        a.push(self.dir.join("out").display().to_string());
        a.extend(extra.iter().map(|s| s.to_string()));
        a
    }
    fn out(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.join("out").join(name)).unwrap_or_default()
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn rows(w: &Work, id: &str) -> Vec<Vec<String>> {
    let text = w.out(&format!("lists/{id}.csv"));
    mzlab_cli::normative::csvrd::Records::new(text.as_bytes(), b';')
        .skip(1)
        .collect()
}

/// The CSV text with the retrieval date (the clock of the day) replaced by `DATE`.
fn undated(w: &Work, id: &str) -> String {
    w.out(&format!("lists/{id}.csv"))
        .lines()
        .map(|l| {
            let mut f: Vec<&str> = l.split(';').collect();
            if f.len() > 2 && f[0] == id {
                f[2] = "DATE";
            }
            f.join(";")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn cell<'a>(r: &'a [String], col: &str, header: &[&str]) -> &'a str {
    &r[header.iter().position(|h| *h == col).expect("column")]
}

const HEADER: [&str; 25] = [
    "list_id",
    "list_version",
    "access_date",
    "source",
    "name",
    "cas",
    "pubchem_cid",
    "inchikey",
    "inchikey_skeleton",
    "smiles",
    "formula",
    "mono_mass",
    "list_attribute",
    "is_tp",
    "parent",
    "tp_reference",
    "lc_esi_amenable",
    "amenability_rule",
    "xlogp",
    "resolved_by",
    "entry_ref",
    "ec",
    "dtxsid",
    "form",
    "status",
];

fn built() -> Work {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let w = Work::new(&format!(
        "b{}",
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let r = run_with(w.args(&["--update-pins"]), Fake::standard());
    assert_eq!(r.code, 0, "{}", r.stderr);
    w
}

#[test]
fn normative_canonical_bytes_of_a_list() {
    let w = Work::new("golden");
    let r = run_with(w.args(&["--update-pins"]), Fake::standard());
    assert_eq!(r.code, 0, "{}", r.stderr);
    let golden = String::from_utf8(data("golden_T_EU_WATER.csv")).expect("utf8");
    assert_eq!(undated(&w, "T_EU_WATER"), golden);
    let text = String::from_utf8_lossy(&r.stdout).to_string();
    assert!(
        text.contains("T_EU_WATER entries=4 screenable=2 groups=1 uvcb=0 inorganic=0 unresolved=1"),
        "{text}"
    );
}

#[test]
fn normative_anion_collapses_to_its_parent_skeleton() {
    let w = built();
    let rs = rows(&w, "T_EU_WATER");
    let q = rs
        .iter()
        .find(|r| cell(r, "name", &HEADER) == "Quillate")
        .expect("row");
    assert_eq!(cell(q, "inchikey_skeleton", &HEADER), "QUILLICACIDXAA");
    assert_eq!(cell(q, "form", &HEADER), "anion");
    assert_eq!(cell(q, "smiles", &HEADER), "CC(=O)O");
    assert_eq!(cell(q, "formula", &HEADER), "C2H4O2");
    assert_eq!(cell(q, "resolved_by", &HEADER), "susdat");
}

#[test]
fn normative_group_entries_are_counted_and_not_screenable() {
    let w = built();
    let rs = rows(&w, "T_EU_WATER");
    let g = rs
        .iter()
        .find(|r| cell(r, "name", &HEADER).starts_with("Sum of zorb"))
        .expect("row");
    assert_eq!(cell(g, "status", &HEADER), "group");
    assert_eq!(cell(g, "inchikey_skeleton", &HEADER), "");
    assert!(w
        .out("provenance.json")
        .contains("\"coverage\":{\"entries\":4,\"screenable\":2,\"groups\":1,"));
    assert!(!w.out("unresolved.tsv").contains("Sum of zorb"));
}

#[test]
fn normative_two_skeletons_for_one_cas_are_unresolved_with_both_logged() {
    let w = built();
    let rs = rows(&w, "T_EU_WATER");
    let a = rs
        .iter()
        .find(|r| cell(r, "name", &HEADER) == "Ambiguol")
        .expect("row");
    assert_eq!(cell(a, "resolved_by", &HEADER), "unresolved");
    assert_eq!(cell(a, "inchikey_skeleton", &HEADER), "");
    let tsv = w.out("unresolved.tsv");
    let line = tsv
        .lines()
        .find(|l| l.contains("Ambiguol"))
        .expect("logged");
    assert!(
        line.contains("AMBIGUOLONEXAA") && line.contains("AMBIGUOLTWOXAA"),
        "{line}"
    );
}

#[test]
fn normative_each_step_of_the_resolution_order_is_used() {
    let w = built();
    let by = |list: &str, name: &str| {
        let rs = rows(&w, list);
        let r = rs
            .iter()
            .find(|r| cell(r, "name", &HEADER) == name)
            .expect("row");
        (
            cell(r, "resolved_by", &HEADER).to_string(),
            cell(r, "inchikey_skeleton", &HEADER).to_string(),
            cell(r, "form", &HEADER).to_string(),
        )
    };
    assert_eq!(by("T_VOCI", "Lawol").0, "law");
    assert_eq!(by("T_EU_WATER", "Zorbamide").0, "susdat");
    // PubChem: the listed CID is a salt of the parent, whose skeleton is the match key
    assert_eq!(
        by("T_EU_PPP", "Pubchemol"),
        ("pubchem".into(), "PUBCHEMOLXXXAA".into(), "salt".into())
    );
    assert_eq!(by("T_US_EPA", "Dsstoxol").0, "dsstox");
    assert_eq!(
        by("T_US_ECFR", "Curatol"),
        (
            "curated".into(),
            "CURATOLXXXXXAA".into(),
            "as_listed".into()
        )
    );
    assert_eq!(by("T_US_ECFR", "Ecfrol").0, "unresolved");
    // a micro-organism is counted and not screenable
    assert_eq!(by("T_EU_PPP", "Bacillus fakeus").2, "");
    assert!(w.out("provenance.json").contains("\"uvcb\":1"));
}

#[test]
fn normative_output_is_read_by_read_list() {
    let w = built();
    for id in ["T_EU_WATER", "T_EU_PPP", "T_US_ECFR", "T_US_EPA", "T_VOCI"] {
        let list = read_list(&w.out(&format!("lists/{id}.csv"))).expect("readable");
        assert!(!list.is_empty(), "{id}");
        assert!(list.iter().all(|r| !r.get("name").is_empty()));
    }
    let prov = w.out("provenance.json");
    assert!(prov.starts_with("{\"schema\":\"normative-prov/1\""));
}

#[test]
fn normative_warm_cache_rebuild_is_offline_and_byte_identical() {
    let w = Work::new("warm");
    let net = Fake::standard();
    assert_eq!(run_with(w.args(&["--update-pins"]), net.clone()).code, 0);
    let first: Vec<String> = [
        "lists/T_EU_WATER.csv",
        "lists/T_EU_PPP.csv",
        "lists/T_US_ECFR.csv",
        "lists/T_US_EPA.csv",
        "lists/T_VOCI.csv",
        "provenance.json",
        "unresolved.tsv",
        "sources.json",
    ]
    .iter()
    .map(|f| w.out(f))
    .collect();
    assert!(net.count() > 0);
    let none = Fake::empty();
    let r = run_with(w.args(&[]), none.clone());
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(none.count(), 0);
    let second: Vec<String> = [
        "lists/T_EU_WATER.csv",
        "lists/T_EU_PPP.csv",
        "lists/T_US_ECFR.csv",
        "lists/T_US_EPA.csv",
        "lists/T_VOCI.csv",
        "provenance.json",
        "unresolved.tsv",
        "sources.json",
    ]
    .iter()
    .map(|f| w.out(f))
    .collect();
    assert_eq!(first, second);
}

#[test]
fn normative_pins_are_written_and_a_changed_source_stops_the_build() {
    let w = Work::new("pins");
    assert_eq!(
        run_with(w.args(&["--update-pins"]), Fake::standard()).code,
        0
    );
    assert!(w.out("sources.json").contains("\"sha256\": \""));
    let before = w.out("lists/T_EU_WATER.csv");
    // a new cache sees a changed document
    let net = Fake::standard();
    net.set(
        "cellar",
        String::from_utf8(data("cellar.xhtml"))
            .expect("utf8")
            .replace("Zorbamide", "Zorbamide II")
            .into_bytes(),
    );
    let mut a = w.args(&[]);
    let i = a.iter().position(|s| s == "--cache").expect("cache") + 1;
    a[i] = w.dir.join("cache2").display().to_string();
    let r = run_with(a.clone(), net.clone());
    assert_eq!(r.code, 3, "{}", r.stderr);
    assert!(
        r.stderr.contains("err.normative.pin") && r.stderr.contains("--update-pins"),
        "{}",
        r.stderr
    );
    assert_eq!(w.out("lists/T_EU_WATER.csv"), before);
    a.push("--update-pins".into());
    assert_eq!(run_with(a, net).code, 0);
    assert!(w.out("lists/T_EU_WATER.csv").contains("Zorbamide II"));
}

#[test]
fn normative_a_failing_source_keeps_the_last_good_file_and_marks_it_stale() {
    let w = Work::new("stale");
    assert_eq!(
        run_with(w.args(&["--update-pins"]), Fake::standard()).code,
        0
    );
    let before = w.out("lists/T_EU_PPP.csv");
    let net = Fake::standard();
    net.pages
        .lock()
        .expect("lock")
        .remove("https://fixture.test/ppp?page=2");
    let mut a = w.args(&[]);
    let i = a.iter().position(|s| s == "--cache").expect("cache") + 1;
    a[i] = w.dir.join("cache2").display().to_string();
    let r = run_with(a, net);
    assert_eq!(r.code, 1, "{}", r.stderr);
    assert_eq!(w.out("lists/T_EU_PPP.csv"), before);
    let prov = w.out("provenance.json");
    assert!(
        prov.contains("\"stale\":true") && prov.contains("\"error\""),
        "{prov}"
    );
    assert!(String::from_utf8_lossy(&r.stdout).contains("T_EU_PPP stale"));
}

#[test]
fn normative_only_builds_the_named_lists() {
    let w = Work::new("only");
    let r = run_with(
        w.args(&["--update-pins", "--only", "T_US_EPA"]),
        Fake::standard(),
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(!w.out("lists/T_US_EPA.csv").is_empty());
    assert!(w.out("lists/T_EU_WATER.csv").is_empty());
}

#[test]
fn normative_usage_errors_have_exit_code_2() {
    let r = run_with(
        vec!["build".into(), "--cache".into(), "x".into()],
        Fake::empty(),
    );
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("err.normative.usage"));
    let r = run_with(vec!["build".into(), "--bogus".into()], Fake::empty());
    assert_eq!(r.code, 2);
}

#[test]
fn normative_a_file_url_reads_the_users_own_download() {
    let w = Work::new("file");
    let own = w.dir.join("prop65.csv");
    std::fs::write(&own, "Chemical,CAS No.,Type of Toxicity,Date Listed\nInventol,9100-16-8,cancer,2001-01-01\nGroup of inventols,,cancer,2002-02-02\n").expect("write");
    let mut src = std::fs::read_to_string(w.dir.join("out/sources.json")).expect("sources");
    let list = format!(
        "{{\"id\":\"T_PROP\",\"reader\":\"ca_prop65\",\"urls\":[{{\"url\":\"file:{}\",\"sha256\":null}}],\"params\":{{}},\"tags\":{{\"jurisdiction\":\"XX\",\"instrument\":\"Act 6\"}}}}",
        own.display()
    );
    src = src.replacen("\"lists\": [", &format!("\"lists\": [{list},"), 1);
    std::fs::write(w.dir.join("out/sources.json"), src).expect("write");
    let r = run_with(
        w.args(&["--only", "T_PROP", "--update-pins", "--no-resolve"]),
        mzlab_cli::normative::fetch::Http::default(),
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    let rs = rows(&w, "T_PROP");
    assert_eq!(rs.len(), 2);
    assert!(rs
        .iter()
        .any(|r| cell(r, "name", &HEADER) == "Group of inventols"
            && cell(r, "status", &HEADER) == "group"));
}

#[test]
fn normative_cas_numbers_are_found_by_shape_and_checked_apart() {
    use mzlab_cli::normative::entry::{cas_check_ok, cas_tokens};
    assert_eq!(
        cas_tokens("a 9100-11-3 and 9100-12-4; 9100-11-3 again, v2-34-5x"),
        ["9100-11-3", "9100-12-4"]
    );
    assert_eq!(cas_tokens("n/a"), Vec::<String>::new());
    // water and ethanol have the right check digit, the invented ones do not
    assert!(cas_check_ok("7732-18-5") && cas_check_ok("64-17-5"));
    assert!(!cas_check_ok("9100-11-3") && !cas_check_ok("not-a-cas"));
}

#[test]
fn normative_markup_rows_carry_rowspans_and_decode_entities() {
    use mzlab_cli::normative::markup::{parse, rows};
    let root = parse(
        "<table><tr><td rowspan=\"2\">7</td><td>A &amp; B</td></tr><tr><td>C</td></tr></table>",
    );
    let t = root.find_all("table")[0];
    let r = rows(t);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0][1].text(), "A & B");
    assert_eq!(
        (r[1][0].text(), r[1][0].carried, r[1][1].text()),
        ("7".to_string(), true, "C".to_string())
    );
}

#[test]
fn normative_csv_records_keep_quotes_and_line_breaks() {
    use mzlab_cli::normative::csvrd::Records;
    let text = "a,b\n\"x, \"\"y\"\"\",\"two\nlines\"\n";
    let r: Vec<Vec<String>> = Records::new(text.as_bytes(), b',').collect();
    assert_eq!(r[1], ["x, \"y\"", "two\nlines"]);
}
