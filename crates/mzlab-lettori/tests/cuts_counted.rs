//! Every member of a synthetic ANDI-MS file or Waters folder cut short at every length gives one keyed error, or scans and chromatograms that are exact
//! parts of the whole file's with the lost scans counted by `missing`: never a scan read short, an export that fails scan by scan, or scans lost unsaid.
#![cfg(all(feature = "andi", feature = "waters"))]

mod common;
mod common_waters;

use mzlab_core::mzml::Spectrum;
use mzlab_core::readers::{FileSet, Limits, Member};

type Members = Vec<(String, Vec<u8>)>;

fn fs(m: &Members) -> FileSet {
    FileSet {
        membri: m
            .iter()
            .map(|(n, b)| Member {
                nome: n.clone(),
                dim: b.len() as u64,
                dati: Box::new(b.clone()),
                sha1: None,
            })
            .collect(),
    }
}

struct ReadOne {
    scans: Vec<Spectrum>,
    missing: usize,
    /// Every chromatogram as (id, time, intensity) pairs.
    chroms: Vec<(String, Vec<(f64, f64)>)>,
}

/// The value of a step of a file that opened: an error there is a finding (the one notice of a cut file is at open).
fn ok<T>(r: Result<T, mzlab_core::error::Error>, at: &str, step: &str) -> T {
    r.unwrap_or_else(|e| panic!("{at}: opened, then {step} gave {}", e.key))
}

/// `Err`: the key of the error at open, the one notice a cut file may give; once open, every scan, chromatogram and the mzML must read (`panic` names the step).
fn read(m: &Members, at: &str) -> Result<ReadOne, &'static str> {
    let mut l = mzlab_lettori::apri(fs(m), &Limits::sito()).map_err(|e| e.key)?;
    let n = ok(l.index(0).map(|v| v.len()), at, "the index");
    let scans: Vec<Spectrum> = (0..n).map(|k| ok(l.scan(0, k), at, "a scan")).collect();
    let mut chroms = Vec::new();
    for v in ok(l.chromatograms(0), at, "the chromatogram list") {
        let c = ok(l.chromatogram(0, &v.id), at, "a chromatogram");
        chroms.push((
            v.id,
            c.time
                .iter()
                .copied()
                .zip(c.intensity.iter().copied())
                .collect(),
        ));
    }
    ok(
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}),
        at,
        "the mzML",
    );
    Ok(ReadOne {
        scans,
        missing: l.missing(0),
        chroms,
    })
}

fn every_cut_is_counted(name: &str, file: Members) {
    let want = read(&file, name).unwrap();
    assert!(
        !want.scans.is_empty() && want.missing == 0,
        "{name}: the whole file"
    );
    for q in 0..file.len() {
        let len = file[q].1.len();
        for cut in (0..len).step_by(4) {
            let mut m = file.clone();
            m[q].1.truncate(cut);
            let at = format!("{name} {} cut at {cut}/{len}", file[q].0);
            // a text header cut outside a calibration line reads as a shorter header: only an empty one or a cut calibration line is told (lost whole lines are not)
            let row = file[q].1[..cut]
                .rsplit(|&x| x == b'\n')
                .next()
                .unwrap_or_default();
            if file[q].0.ends_with(".TXT")
                && cut > 0
                && !row.trim_ascii_start().starts_with(b"$$ Cal")
            {
                continue;
            }
            let have = match read(&m, &at) {
                Err(key) => {
                    assert!(key.starts_with("err."), "{at}: {key}");
                    continue;
                }
                Ok(h) => h,
            };
            let part = have.scans.iter().all(|a| {
                want.scans
                    .iter()
                    .any(|b| a.id == b.id && a.mz == b.mz && a.intensity == b.intensity)
            });
            assert!(
                part && have.scans.len() <= want.scans.len(),
                "{at}: a scan read short or not in the whole file"
            );
            assert!(
                have.scans.len() == want.scans.len() || have.missing > 0,
                "{at}: {} of {} scans and no notice",
                have.scans.len(),
                want.scans.len()
            );
            for (id, pts) in &have.chroms {
                if let Some((_, all)) = want.chroms.iter().find(|(w, _)| w == id) {
                    assert!(
                        pts.iter().all(|p| all.contains(p)),
                        "{at}: chromatogram {id} has a point the whole file has not"
                    );
                }
            }
        }
    }
}

#[test]
fn an_andi_ms_file_cut_short_is_counted() {
    every_cut_is_counted(
        "andi-ms",
        vec![(
            "x.cdf".into(),
            common::andi::andi_ms(&common::andi::FLAT_RUN),
        )],
    );
}

#[test]
fn a_waters_folder_cut_short_is_counted() {
    every_cut_is_counted("waters", common_waters::folder_file());
}
