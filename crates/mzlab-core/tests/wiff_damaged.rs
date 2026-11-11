//! A `.wiff.scan` with one byte changed (`one` and `two` of `tools/wiff_riferimento.py`, every byte, two flips) gives one keyed error, or every
//! scan of the unchanged pair (the oracle) with its number of points, or a count in `losses`: a scan is never left out or read short unsaid.
//! A value changed with its point count kept cannot be seen (the format has no checksum). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::Spectrum;
use mzlab_core::readers::{self, FileSet, Limits, Losses, Member, DAMAGED};

fn reference() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_danno_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_damaged: python3 not available");
        return None;
    }
    Some(out)
}

fn pair(wiff: &[u8], scan: &[u8]) -> FileSet {
    let member = |name: &str, b: &[u8]| Member {
        nome: name.into(),
        dim: b.len() as u64,
        dati: Box::new(b.to_vec()),
        sha1: None,
    };
    FileSet {
        membri: vec![member("x.wiff", wiff), member("x.wiff.scan", scan)],
    }
}

/// The scans of every sample and its losses (`Err` = the key of the first error).
fn read(wiff: &[u8], scan: &[u8]) -> Result<Vec<(Vec<Spectrum>, Losses)>, &'static str> {
    let mut l = readers::apri(pair(wiff, scan), &Limits::sito()).map_err(|e| e.key)?;
    let mut out = Vec::new();
    for c in 0..l.samples().len() {
        let n = l.index(c).map_err(|e| e.key)?.len();
        let scans = (0..n)
            .map(|k| l.scan(c, k))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.key)?;
        out.push((scans, l.losses(c).map_err(|e| e.key)?));
    }
    Ok(out)
}

/// The scans of `want` left out of `have` (by id) and those given with another number of points.
fn changed(have: &[Spectrum], want: &[Spectrum]) -> (usize, usize) {
    let find = |w: &Spectrum| have.iter().find(|h| h.id == w.id);
    (
        want.iter().filter(|w| find(w).is_none()).count(),
        want.iter()
            .filter(|w| find(w).is_some_and(|h| h.mz.len() != w.mz.len()))
            .count(),
    )
}

#[test]
fn a_scan_file_with_one_byte_changed_gives_a_keyed_error_or_counts_what_it_changed() {
    let Some(dir) = reference() else { return };
    let (mut silent, mut told) = (Vec::new(), 0);
    for name in ["one", "two"] {
        let wiff = std::fs::read(dir.join(format!("{name}.wiff"))).unwrap();
        let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
        let full = read(&wiff, &scan).unwrap();
        assert!(
            full.iter().all(|(_, l)| !l.partial()),
            "{name}: the whole pair loses nothing"
        );
        for at in 0..scan.len() {
            for flip in [0xff_u8, 0x01] {
                let mut s = scan.clone();
                s[at] ^= flip;
                let Ok(got) = read(&wiff, &s) else { continue };
                for (c, ((have, l), (want, _))) in got.iter().zip(&full).enumerate() {
                    told += usize::from(l.cause == DAMAGED);
                    let (out, short) = changed(have, want);
                    if out + short > 0 && l.lost() == 0 {
                        silent.push(format!("{name} byte {at} ^{flip:02x}: sample {c}, {out} left out, {short} with other points, unsaid"));
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        silent.is_empty(),
        "{} silent changes, first: {:?}",
        silent.len(),
        &silent[..silent.len().min(8)]
    );
    assert!(told > 0, "no changed byte was counted as a damaged block");
}
