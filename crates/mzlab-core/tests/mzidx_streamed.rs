//! The one-pass build (`mzidx::build_mzml_streamed`: pass 1 on the bytes as they arrive, the scan table at the end) writes the bytes of today's
//! build (`upload::prepare` + `mzidx::build_mzml_corriere`) for low-resolution and Orbitrap headers, with the peaks in memory and in the scratch area,
//! from a stream that gives a few bytes at a time; a table that disagrees with pass 1 falls back to today's build from the copy, same bytes.
mod fake_reader;
use std::io::{Cursor, Read};

use fake_reader::Finto;
use mzlab_core::mzml::Corriere;
use mzlab_core::mzmlwrite::Header;
use mzlab_core::{mzidx, readers_mzml, upload};

/// No helper instance: the parser decodes every spectrum itself.
struct Alone;
impl Corriere for Alone {
    fn window(&self) -> usize {
        0
    }
    fn send(&mut self, _: u64, _: &[u8], _: usize) -> bool {
        false
    }
    fn take(&mut self, _: u64) -> Option<Vec<u8>> {
        None
    }
}

/// A stream that answers at most `step` bytes per read, as the pieces of a copy arrive.
struct Pieces<'a> {
    b: &'a [u8],
    at: usize,
    step: usize,
}
impl Read for Pieces<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = out.len().min(self.step).min(self.b.len() - self.at);
        out[..n].copy_from_slice(&self.b[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

fn orbitrap() -> Header {
    Header {
        run: "r".into(),
        sources: vec!["r.raw".into()],
        software: ("mzLab".into(), "1".into()),
        components: vec![
            (
                "source".into(),
                "MS:1000073".into(),
                "electrospray ionization".into(),
            ),
            ("analyzer".into(), "MS:1000484".into(), "orbitrap".into()),
        ],
        ..Header::default()
    }
}

fn mzml(mut l: Finto) -> Vec<u8> {
    let mut mz = Vec::new();
    readers_mzml::write(&mut l, 0, &mut mz, &mut |_, _| {}).unwrap();
    mz
}

/// Today's index and the streamed one (with the table `twist` changes), and whether the one pass held.
fn both(
    mz: &[u8],
    budget: usize,
    step: usize,
    twist: fn(&mut mzidx::ScanTable),
) -> (Vec<u8>, Vec<u8>, bool) {
    let p = upload::prepare(&mut mz.to_vec(), mz.len() as u64, "s.mzML").unwrap();
    let (mut a, mut sa) = (Cursor::new(Vec::new()), Cursor::new(Vec::new()));
    mzidx::build_mzml_corriere(
        &p.table,
        Cursor::new(mz),
        &mut a,
        budget,
        Some(&mut sa),
        &mut Alone,
    )
    .unwrap();
    let (mut b, mut sb) = (Cursor::new(Vec::new()), Cursor::new(Vec::new()));
    let mut table = || {
        let mut t = p.table.clone();
        twist(&mut t);
        Ok(t)
    };
    let (_, held) = mzidx::build_mzml_streamed(
        Pieces { b: mz, at: 0, step },
        Cursor::new(mz),
        &mut table,
        &mut b,
        budget,
        Some(&mut sb),
        &mut Alone,
    )
    .unwrap();
    (a.into_inner(), b.into_inner(), held)
}

#[test]
fn the_streamed_index_is_todays() {
    for (name, l) in [
        ("low", Finto::vario(600, 120, 3)),
        (
            "orbitrap",
            Finto::vario(600, 120, 5).with_header(orbitrap()),
        ),
    ] {
        let mz = mzml(l);
        for (budget, step) in [
            (mzidx::DEFAULT_BUDGET, 1 << 20),
            (1 << 16, 4093),
            (1 << 16, 1 << 22),
        ] {
            let (today, one, held) = both(&mz, budget, step, |_| {});
            assert!(
                held,
                "{name}: the one pass fell back (budget {budget}, step {step})"
            );
            assert!(
                today == one,
                "{name}: the streamed index differs (budget {budget}, step {step})"
            );
        }
    }
}

#[test]
fn a_table_that_disagrees_falls_back_to_the_same_bytes() {
    let mz = mzml(Finto::vario(300, 60, 7));
    let (today, one, held) = both(&mz, 1 << 16, 8191, |t| t.hr1 = !t.hr1);
    assert!(!held, "a wrong hr1 must not keep pass 1");
    // the twisted table writes another index; what matters is that the streamed build is the build of that table
    let p = upload::prepare(&mut mz.clone(), mz.len() as u64, "s.mzML").unwrap();
    let mut t = p.table.clone();
    t.hr1 = !t.hr1;
    let (mut a, mut sa) = (Cursor::new(Vec::new()), Cursor::new(Vec::new()));
    mzidx::build_mzml_corriere(
        &t,
        Cursor::new(&mz),
        &mut a,
        1 << 16,
        Some(&mut sa),
        &mut Alone,
    )
    .unwrap();
    assert!(
        a.into_inner() == one,
        "the fallback is not today's build of its table"
    );
    assert!(!today.is_empty());
}
