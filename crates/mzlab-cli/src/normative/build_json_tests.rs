//! `db normative build --json`: the `normative-build/1` document carries the lists, the sources with their pins and the stale lists.

use super::tests::{data, Fake, Work};
use super::*;
use crate::normative::fetch::Fetch;

/// `--json`: one `normative-build/1` document with the lists, the sources and their pins; a stale list is exit 1 and the document stays on stdout.
#[test]
fn normative_build_json_carries_sources_pins_and_stale_lists() {
    let w = Work::new("json");
    let r = run_with(w.args(&["--update-pins", "--json"]), Fake::standard());
    assert_eq!(r.code, 0, "{}", r.stderr);
    let text = String::from_utf8(r.stdout).unwrap();
    assert!(text.starts_with("{\"schema\":\"normative-build/1\"") && text.ends_with("}\n"));
    let v = mzlab_core::json::parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("pins_updated").and_then(|x| x.bool()), Some(true));
    let lists: Vec<&str> = v
        .get("lists")
        .unwrap()
        .arr()
        .iter()
        .map(|l| l.get("id").unwrap().str().unwrap())
        .collect();
    assert!(lists.contains(&"T_EU_WATER"));
    let sources = v.get("sources").unwrap().arr();
    assert!(!sources.is_empty());
    for s in sources {
        assert_eq!(s.get("sha256").unwrap().str().map(str::len), Some(64));
    }
    // a second run on the same cache reads the same bytes: every pin matches and nothing is written
    let again = run_with(w.args(&["--json"]), Fake::standard());
    assert_eq!(again.code, 0, "{}", again.stderr);
    let v = mzlab_core::json::parse(&again.stdout).unwrap();
    assert!(v
        .get("sources")
        .unwrap()
        .arr()
        .iter()
        .all(|s| s.get("pin_ok").and_then(|x| x.bool()) != Some(false)
            && s.get("pin_updated").and_then(|x| x.bool()) == Some(false)));
    assert_eq!(v.get("pins_updated").and_then(|x| x.bool()), Some(false));
}

/// The arguments as the dispatcher passes them to the contract: the command path first.
fn full(args: &[String]) -> Vec<String> {
    ["db", "normative"]
        .iter()
        .map(|s| s.to_string())
        .chain(args.iter().cloned())
        .collect()
}

/// A second cache that sees `net` instead of the pinned sources.
fn other_cache(w: &Work, extra: &[&str]) -> Vec<String> {
    let mut a = w.args(extra);
    let i = a.iter().position(|s| s == "--cache").expect("cache") + 1;
    a[i] = w.dir().join("cache2").display().to_string();
    a
}

/// A stale list under `--json`: exit 1, and the `normative-build/1` document on stdout says which list and why.
#[test]
fn a_stale_list_is_exit_1_with_the_document_on_stdout() {
    let w = Work::new("jsonstale");
    assert_eq!(
        run_with(w.args(&["--update-pins"]), Fake::standard()).code,
        0
    );
    let net = Fake::standard();
    net.drop_page("ppp?page=2");
    let args = other_cache(&w, &["--json"]);
    // through the contract, as the binary runs it: a verdict keeps its own document on stdout
    let r = crate::contract::finish(
        &full(&args),
        "db normative build",
        run_with(args.clone(), net),
    );
    assert_eq!(r.code, 1, "{}", r.stderr);
    let text = String::from_utf8(r.stdout).unwrap();
    assert!(!text.contains("\"schema\":\"error/1\""), "{text}");
    assert!(text.starts_with("{\"schema\":\"normative-build/1\"") && text.ends_with("}\n"));
    let v = mzlab_core::json::parse(text.as_bytes()).unwrap();
    assert!(v.get("stale_lists").and_then(|x| x.f64()).unwrap() >= 1.0);
    let stale = v.get("stale").unwrap().arr();
    assert!(stale
        .iter()
        .any(|f| f.get("id").unwrap().str() == Some("T_EU_PPP")
            && !f.get("why").unwrap().str().unwrap().is_empty()));
}

/// A changed source without `--update-pins`: exit 3, one `error/1` on stdout under the contract (its text on stderr), and no build document.
#[test]
fn a_pin_mismatch_is_exit_3_with_one_error_document() {
    let w = Work::new("jsonpin");
    assert_eq!(
        run_with(w.args(&["--update-pins"]), Fake::standard()).code,
        0
    );
    let net = Fake::standard();
    net.set(
        "cellar",
        String::from_utf8(data("cellar.xhtml"))
            .unwrap()
            .replace("Zorbamide", "Zorbamide II")
            .into_bytes(),
    );
    let args = other_cache(&w, &["--json"]);
    let o = crate::contract::finish(
        &full(&args),
        "db normative build",
        run_with(args.clone(), net),
    );
    assert_eq!(o.code, 3, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(
        text.starts_with("{\"schema\":\"error/1\"") && text.ends_with("}\n"),
        "{text}"
    );
    assert_eq!(text.matches("\"schema\"").count(), 1, "{text}");
    assert!(text.contains("err.normative.pin") && !text.contains("normative-build/1"));
    assert!(
        o.stderr.contains("err.normative.pin") || o.stderr.contains("pin"),
        "{}",
        o.stderr
    );
}

/// A download above the limit is refused: exit 6, one `error/1` under `--json`.
#[test]
fn a_refused_download_is_exit_6() {
    struct TooBig;
    impl Fetch for TooBig {
        fn get(
            &self,
            url: &str,
            limit: u64,
            _out: &mut dyn std::io::Write,
        ) -> Result<(), NormError> {
            Err(NormError::large(url, limit))
        }
    }
    let w = Work::new("jsonlarge");
    let args = w.args(&["--json"]);
    let o = crate::contract::finish(
        &full(&args),
        "db normative build",
        run_with(args.clone(), TooBig),
    );
    assert_eq!(o.code, 6, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(
        text.starts_with("{\"schema\":\"error/1\"") && text.contains("err.normative.large"),
        "{text}"
    );
}

/// `pack` and `listings` take no option: any flag is a usage error (exit 2), also with `--json`.
#[test]
fn pack_and_listings_refuse_unknown_flags() {
    for op in ["pack", "listings"] {
        for flag in ["--bogus", "--json"] {
            let r = run_with(
                vec![op.into(), "x".into(), "y".into(), flag.into()],
                Fake::standard(),
            );
            assert_eq!(r.code, 2, "{op} {flag}: {}", r.stderr);
            assert!(r.stdout.is_empty(), "{op} {flag}");
        }
    }
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
    // an unknown operation is a usage error too
    let r = run_with(vec!["bogus".into()], Fake::empty());
    assert_eq!(r.code, 2);
    assert!(r.stdout.is_empty());
}
