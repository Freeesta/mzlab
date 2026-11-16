//! `mzlab-cli convert` on a `.wiff` pair with one broken unit (`tools/wiff_isolation.py`): the healthy samples are written byte for byte as from the
//! whole pair (the SHA-1 digests of the `.wiff.scan` and of the document aside), the damaged one fails alone with its cause; a NaN cps is never
//! written and its cycle is counted; `manual-table` lists a sample at position 0 of the table. Skipped without python3.
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::convert;
use crate::convert_fixtures::{folder, makers};
use crate::flow_run_tests::p;

/// `x.wiff` + `x.wiff.scan` copied from pair `name` of the isolation script into `dir/in`, or none without python3.
fn pair(dir: &Path, name: &str) -> Option<PathBuf> {
    let src = dir.join("src");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = Command::new("python3")
        .arg(root.join("tools/wiff_isolation.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP convert isolation: python3 not available");
        return None;
    }
    let inp = dir.join("in");
    std::fs::create_dir_all(&inp).unwrap();
    for ext in ["wiff", "wiff.scan"] {
        std::fs::copy(
            src.join(format!("{name}.{ext}")),
            inp.join(format!("x.{ext}")),
        )
        .unwrap();
    }
    Some(inp.join("x.wiff"))
}

/// The bytes with every run of exactly 40 hex digits (a SHA-1) blanked.
fn without_digests(b: &[u8]) -> Vec<u8> {
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        let n = b[i..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
        if n == 40 {
            out[i..i + n].fill(b'#');
        }
        i += n.max(1);
    }
    out
}

/// The document with the source identity left out (the index that closes an indexed mzML aside): the SHA-1 checksum lines, the identity line, and every SHA-1 / partial identity inside the method document.
fn without_identity(b: &[u8]) -> Vec<u8> {
    let whole = String::from_utf8_lossy(b);
    // the offsets of the index and the checksum of the file that close it move with the length of the identity: the document before them is compared
    let text = whole.split("<indexList").next().unwrap_or_default();
    let mut joined = text.to_string();
    // each element that carries a SHA-1 checksum or the identity of the source is cut out whole
    for marker in ["MS:1000569", "source-identity"] {
        while let Some(m) = joined.find(marker) {
            let from = joined[..m].rfind('<').unwrap_or(m);
            let to = joined[m..].find("/>").map_or(joined.len(), |e| m + e + 2);
            joined.replace_range(from..to, "");
        }
    }
    while let Some(i) = joined.find("partial(sample=") {
        let end = (i + "partial(sample=N):".len() + 64).min(joined.len());
        joined.replace_range(i..end, "#");
    }
    joined.retain(|c| c != '\n');
    let blanked = String::from_utf8_lossy(&without_digests(joined.as_bytes())).into_owned();
    // a blanked digest and the blanked partial identity stand for the same field
    let mut out = String::new();
    for c in blanked.chars() {
        if c != '#' || !out.ends_with('#') {
            out.push(c);
        }
    }
    out.into_bytes()
}

#[test]
fn the_healthy_samples_of_a_pair_with_a_damaged_one_are_written_as_from_the_whole_pair() {
    let (good, bad) = (folder("iso_sano"), folder("iso_guasto"));
    let (Some(g), Some(b)) = (pair(&good, "sano"), pair(&bad, "guasto")) else {
        return;
    };
    let o = convert(&[p(&g), "--out", p(&good.join("out"))]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = convert(&[p(&b), "--out", p(&bad.join("out"))]);
    assert_ne!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("1 of 3 failed (2 converted"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains("err.file.wiff detail=TOF block"),
        "{}",
        o.stderr
    );
    assert!(!bad.join("out/x-1.mzML").exists());
    for s in [2, 3] {
        let read = |d: &Path| std::fs::read(d.join(format!("out/x-{s}.mzML"))).unwrap();
        let (want, got) = (read(&good), read(&bad));
        assert_eq!(without_digests(&got), without_digests(&want), "sample {s}");
    }
    let _ = (
        std::fs::remove_dir_all(&good),
        std::fs::remove_dir_all(&bad),
    );
}

#[test]
fn a_nan_cps_is_never_written_and_its_cycle_is_counted() {
    let dir = folder("iso_nan");
    let Some(w) = pair(&dir, "mrm_nan") else {
        return;
    };
    let report = dir.join("r.json");
    let o = convert(&[p(&w), "--out", p(&dir.join("out")), "--report", p(&report)]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert!(
        o.stderr
            .contains("partial read, 9 of 10 MRM cycles (warn.wiff.damaged)"),
        "{}",
        o.stderr
    );
    let r = std::fs::read_to_string(&report).unwrap();
    assert!(r.contains("\"unita\":\"cicli\""), "{r}");
    let f = std::fs::File::open(dir.join("out/x-1.mzML")).unwrap();
    let size = f.metadata().unwrap().len();
    let mut m = mzlab_core::mzmlhead::Mzml::new(f, size);
    let chroms = m.chromatograms(0).unwrap();
    assert_eq!(chroms.len(), 4, "TIC and three transitions");
    for c in &chroms {
        let (t, y) = m.chromatogram(c).unwrap();
        assert_eq!((t.len(), y.len()), (9, 9), "9 of 10 cycles");
        assert!(
            t.iter().chain(&y).all(|x| x.is_finite()),
            "a NaN or infinity in the mzML"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn manual_table_lists_a_sample_at_position_zero_of_the_table() {
    let dir = folder("iso_vuoto0");
    let Some(w) = pair(&dir, "vuoto0") else {
        return;
    };
    let mut warn = String::new();
    let files = crate::manual_table_cmd::files_json(&[w], &mut warn).unwrap();
    assert_eq!(files.matches("{\"name\":").count(), 2, "{files}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--samples` writes only the samples asked for, byte for byte the files the whole pair gives (the choice of the page: the same samples, the same bytes);
/// an id the pair does not hold is an error that writes nothing.
#[test]
fn samples_writes_only_the_chosen_ones_as_the_whole_pair_does() {
    let (all, some) = (folder("sel_tutti"), folder("sel_alcuni"));
    let (Some(a), Some(s)) = (pair(&all, "sano"), pair(&some, "sano")) else {
        return;
    };
    assert_eq!(convert(&[p(&a), "--out", p(&all.join("out"))]).code, 0);
    let o = convert(&[p(&s), "--out", p(&some.join("out")), "--samples", "3,2"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!some.join("out/x-1.mzML").exists());
    let mut identities = Vec::new();
    for id in [2, 3] {
        let read = |d: &Path| std::fs::read(d.join(format!("out/x-{id}.mzML"))).unwrap();
        let (whole, chosen) = (read(&all), read(&some));
        // the same file but for the source identity: whole-file SHA-1 checksums for the whole pair, a named partial identity for the chosen samples
        let (w, c) = (without_identity(&whole), without_identity(&chosen));
        let at = w
            .iter()
            .zip(&c)
            .position(|(a, b)| a != b)
            .unwrap_or(w.len().min(c.len()));
        assert!(
            w == c,
            "sample {id} differs at {at}: {:?} / {:?}",
            String::from_utf8_lossy(&w[at.saturating_sub(80)..(at + 120).min(w.len())]),
            String::from_utf8_lossy(&c[at.saturating_sub(80)..(at + 120).min(c.len())])
        );
        let text = String::from_utf8_lossy(&chosen).into_owned();
        let found: Vec<&str> = text
            .match_indices("partial(sample=")
            .map(|(i, _)| &text[i..i + "partial(sample=N):".len() + 64])
            .collect();
        assert!(!found.is_empty(), "sample {id} names its partial identity");
        assert!(found
            .iter()
            .all(|f| f.starts_with(&format!("partial(sample={id}):"))));
        assert!(!String::from_utf8_lossy(&whole).contains("partial(sample="));
        identities.push(found[0].to_string());
    }
    assert_ne!(
        identities[0], identities[1],
        "another sample, another identity"
    );
    let o = convert(&[p(&s), "--out", p(&some.join("out2")), "--samples", "9"]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    assert!(o.stderr.contains("sample 9"), "{}", o.stderr);
}

/// `--samples 1` takes a dataset of one sample (it is its sample 1), another number is the exit 4, and an input the readers cannot open is not dropped for
/// lack of a sample number: it fails with its own error.
#[test]
fn samples_one_takes_a_one_sample_dataset_and_a_broken_input_still_fails() {
    let d = folder("sel_uno");
    let inp = makers(&d, true);
    let (a, bad) = (inp.join("a.mzXML"), inp.join("rotto.raw"));
    let o = convert(&[p(&a), p(&bad), "--out", p(&d.join("out")), "--samples", "1"]);
    assert!(d.join("out/a.mzML").exists(), "{}", o.stderr);
    assert_ne!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("rotto"), "{}", o.stderr);
    let o = convert(&[p(&a), "--out", p(&d.join("out2")), "--samples", "2"]);
    assert_eq!(o.code, 4, "{}", o.stderr);
}
