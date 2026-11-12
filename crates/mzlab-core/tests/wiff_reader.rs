//! The `.wiff` pair read through the `Lettore` contract gives the same spectra and chromatograms as `Wiff::run`, and is recognised by the registry.
//! Synthetic pairs need python3 (`tools/wiff_riferimento.py`), the real ones the data repository (`MZLAB_DATI`); both skip when missing.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, samples, Wiff};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn same(dir: &Path, name: &str) -> usize {
    let mut l = readers::apri(set(dir, name), &Limits::sito()).unwrap();
    assert_eq!(l.format().id(), "sciex-wiff");
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    let ids = samples(&ole);
    assert_eq!(l.samples().len(), ids.len(), "{name}: samples");
    let mut checked = 0;
    for (c, s) in ids.into_iter().enumerate() {
        let mut w = Wiff::open(&ole, name, Some(scan.len() as u64), s).unwrap();
        w.wiff_sha1 = Some(ole.sha1_hex());
        let run = w.run(&mut scan).unwrap();
        let index = l.index(c).unwrap().to_vec();
        assert_eq!(index.len(), run.spectra.len(), "{name}/{s}: index");
        for (k, want) in run.spectra.iter().enumerate() {
            assert_eq!(
                (index[k].id.as_str(), index[k].rt, index[k].livello),
                (want.id.as_str(), want.rt, want.level),
                "{name}/{s} row {k}"
            );
            assert!(index[k]
                .position
                .is_some_and(|p| p.membro == 1 && p.lunghezza > 0));
            let got = l.scan(c, k).unwrap();
            assert_eq!(
                (
                    &got.id,
                    got.level,
                    got.polarity,
                    got.profile,
                    got.precursor,
                    got.rt
                ),
                (
                    &want.id,
                    want.level,
                    want.polarity,
                    want.profile,
                    want.precursor,
                    want.rt
                )
            );
            assert_eq!(
                (&got.mz, &got.wide),
                (&want.mz, &want.wide),
                "{name}/{s}: {}",
                want.id
            );
            checked += 1;
        }
        // the BPC is listed only after the mzML writer has gone through the scans (`extra_scansione`): this test reads them one by one
        let listed = l.chromatograms(c).unwrap();
        let chroms: Vec<_> = run
            .chromatograms
            .iter()
            .filter(|c| c.kind != mzlab_core::mzml::ChromKind::Bpc)
            .collect();
        assert_eq!(listed.len(), chroms.len(), "{name}/{s}: chromatograms");
        for want in chroms {
            assert_eq!(&l.chromatogram(c, &want.id).unwrap(), want);
        }
        let via = mzlab_core::readers_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        let back = mzlab_core::mzml::read(&via[..]).unwrap();
        assert_eq!(back.spectra.len(), run.spectra.len(), "{name}/{s}: mzML");
        assert_eq!(
            back.chromatograms, run.chromatograms,
            "{name}/{s}: mzML chromatograms"
        );
        let doc = l.method(c).unwrap();
        assert!(
            doc.contains("sciex-wiff") && doc.contains(&format!("{name}.wiff")),
            "{doc}"
        );
    }
    checked
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_synthetic_pairs_read_the_same_through_the_registry() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_lettore_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/wiff_riferimento.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_reader: python3 not available");
        return;
    }
    let n: usize = ["one", "two", "serie"]
        .iter()
        .map(|name| same(&out, name))
        .sum();
    assert!(n > 0);
    one_sample(&out, "two");
    mzml_of_one_sample(&out, "two");
    mzml_with_known_sha1(&out, "two");
    truncated(&out, "serie");
    no_idx(&out, "buchi");
}

/// A full scan with a DAD (`tools/wiff_dad_mrm.py`, `dadscan`): the TWC carries the same place in the list on the reader the site uses and in `Wiff::run`,
/// though the reader lists the BPC only after the scans went through the mzML writer.
#[test]
fn a_full_scan_with_a_dad_reads_the_same_through_the_registry() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_dad_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/wiff_dad_mrm.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_reader: python3 not available");
        return;
    }
    assert!(same(&out, "dadscan") > 0);
    let _ = std::fs::remove_dir_all(&out);
}

/// An injection in the sample table with no `Idx` stream (it acquired nothing) is listed with 0 scans and its number, not dropped: the samples after it keep theirs.
fn no_idx(dir: &Path, name: &str) {
    let mut l = readers::apri(set(dir, name), &Limits::sito()).unwrap();
    let got: Vec<(u32, usize)> = l.samples().iter().map(|c| (c.id, c.scansioni)).collect();
    assert_eq!(got, [(1, 1), (2, 0), (3, 1)], "{name}: samples");
    assert!(
        l.index(1).unwrap().is_empty(),
        "{name}: the empty one has no scans"
    );
    assert_eq!(
        l.index(2).unwrap().len(),
        1,
        "{name}: the one after it reads"
    );
}

/// A `.wiff.scan` cut in half: the scans beyond its end are counted by `mancanti` and left out of the index, the others still read.
fn truncated(dir: &Path, name: &str) {
    let mut whole = readers::apri(set(dir, name), &Limits::sito()).unwrap();
    let mut fs = set(dir, name);
    let scan = &mut fs.membri[1];
    let mut bytes = vec![0u8; scan.dim as usize / 2];
    scan.dati.read_at(0, &mut bytes).unwrap();
    scan.dim = bytes.len() as u64;
    scan.dati = Box::new(bytes);
    let mut cut = mzlab_core::wiff::reader::open_only(fs, &Limits::sito(), Some(0)).unwrap();
    let (n, k) = (whole.index(0).unwrap().len(), cut.index(0).unwrap().len());
    assert_eq!(whole.missing(0), 0, "{name}: the whole pair lacks nothing");
    assert!(
        k < n && cut.missing(0) > 0,
        "{name}: {k} of {n} scans, {} missing",
        cut.missing(0)
    );
    assert!(
        k + cut.missing(0) >= n,
        "{name}: every scan left out is counted"
    );
    for s in 0..k {
        cut.scan(0, s).unwrap();
    }
}

/// `apri_solo` (the builder of one index): sample `c` gives the index of the whole opening, the others are listed with 0 scans and give their own index when asked.
fn one_sample(dir: &Path, name: &str) {
    let mut all = readers::apri(set(dir, name), &Limits::sito()).unwrap();
    let n = all.samples().len();
    assert!(n > 1, "{name}: a container with more samples");
    for c in 0..n {
        let mut only =
            mzlab_core::wiff::reader::open_only(set(dir, name), &Limits::sito(), Some(c)).unwrap();
        let want: Vec<_> = (0..n)
            .map(|k| all.samples()[k].scansioni * usize::from(k == c))
            .collect();
        let got: Vec<_> = only.samples().iter().map(|x| x.scansioni).collect();
        assert_eq!(got, want, "{name}: counts with sample {c} alone");
        for k in (c..n).chain(0..c) {
            let a: Vec<_> = all
                .index(k)
                .unwrap()
                .iter()
                .map(|v| (v.id.clone(), v.rt, v.position))
                .collect();
            let b: Vec<_> = only
                .index(k)
                .unwrap()
                .iter()
                .map(|v| (v.id.clone(), v.rt, v.position))
                .collect();
            assert_eq!(a, b, "{name}: index of sample {k} opened for {c}");
            assert_eq!(
                only.samples()[k].scansioni,
                all.samples()[k].scansioni,
                "{name}: scans listed for sample {k} once its index is built"
            );
            if let Some(s) = a.len().checked_sub(1) {
                assert_eq!(all.scan(k, s).unwrap().mz, only.scan(k, s).unwrap().mz);
            }
        }
    }
}

/// The mzML of sample `c` is the same bytes whether the pair was opened for all the injections or for `c` alone (the header carries the SHA-1 of the `.wiff.scan` either way).
fn mzml_of(mut l: Box<dyn readers::Reader>, c: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let conv = mzlab_core::readers_mzml::Conversion {
        steps: Some(&[]),
        ..Default::default()
    };
    conv.write(&mut *l, c, &mut out, &mut |_, _| {}).unwrap();
    out
}

fn mzml_of_one_sample(dir: &Path, name: &str) {
    let n = readers::apri(set(dir, name), &Limits::sito())
        .unwrap()
        .samples()
        .len();
    for c in 0..n {
        let all = mzml_of(readers::apri(set(dir, name), &Limits::sito()).unwrap(), c);
        let only = mzml_of(
            mzlab_core::wiff::reader::open_only(set(dir, name), &Limits::sito(), Some(c)).unwrap(),
            c,
        );
        assert!(!all.is_empty());
        assert!(
            all == only,
            "{name}: mzML of sample {c} opened for it alone"
        );
    }
}

/// The mzML is the same bytes when the members carry the SHA-1 their fingerprints gave (`upload::impronte`: the reader then does not hash the
/// `.wiff.scan` again for the header), an empty one (hashed by the reader) or none.
fn mzml_with_known_sha1(dir: &Path, name: &str) {
    let mut known = set(dir, name);
    mzlab_core::upload::impronte(&mut known).unwrap();
    assert!(known.membri[1].sha1.as_ref().is_some_and(|s| s.len() == 40));
    let mut empty = set(dir, name);
    empty
        .membri
        .iter_mut()
        .for_each(|m| m.sha1 = Some(String::new()));
    let want = mzml_of(readers::apri(set(dir, name), &Limits::sito()).unwrap(), 0);
    for (what, fs) in [("known", known), ("empty", empty)] {
        let got = mzml_of(readers::apri(fs, &Limits::sito()).unwrap(), 0);
        assert!(got == want, "{name}: mzML with a {what} SHA-1");
    }
}

#[test]
fn the_real_files_read_the_same_through_the_registry() {
    let Some(folder) = dati_percorsi::percorso("dati/3200 qtrap/wiff") else {
        eprintln!("SKIP wiff_reader: no data repository (MZLAB_DATI)");
        return;
    };
    let n: usize = ["FullMass", "MS2", "MRM"]
        .iter()
        .filter(|n| folder.join(format!("{n}.wiff")).exists())
        .map(|n| same(&folder, n))
        .sum();
    assert!(n > 0);
}

#[test]
fn a_wiff_alone_is_not_sure_and_not_a_wiff_is_refused() {
    let wiff = |bytes: Vec<u8>, name: &str| FileSet {
        membri: vec![Member {
            nome: name.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    };
    let mut ole_head = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    ole_head.resize(600, 0);
    let mut alone = wiff(ole_head, "x.wiff");
    assert_eq!(
        readers::riconosci(&mut alone).map(|v| v.id),
        Some("sciex-wiff")
    );
    assert_eq!(
        readers::apri(alone, &Limits::sito()).err().unwrap().key,
        "err.file.wiffNoScan"
    );
    let mut text = wiff(b"hello world, not an OLE container".to_vec(), "x.wiff");
    assert!(readers::riconosci(&mut text).is_none());
}
