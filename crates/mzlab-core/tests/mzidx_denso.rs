//! A file whose peaks crowd one 1-Da slice of m/z: the builder refuses it once pass 1 has counted enough of them to know the slice cannot fit
//! (`budget * 2`), without reading the rest of the file, and names the bucket and that limit (phase `indexPeaks`). With room for the slice every budget, with or
//! without a scratch file, writes the same bytes. Synthetic data.

use std::io::{Cursor, Read, Seek, SeekFrom};

use mzlab_core::mzidx::{build_mzml, build_mzml_spill};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;

const SCANS: usize = 1500;
const PEAKS: usize = 1000;
const MB: usize = 1 << 20;

/// Every peak of every scan between m/z 500.0 and 500.1: one bucket holds the whole file.
fn dense() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "dense".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(SCANS).unwrap();
    for i in 0..SCANS {
        let mz: Vec<f64> = (0..PEAKS)
            .map(|j| 500.0 + j as f64 * 1e-4 + i as f64 * 1e-8)
            .collect();
        let intensity: Vec<f32> = (0..PEAKS)
            .map(|j| 100.0 + ((i * 31 + j * 17) % 997) as f32)
            .collect();
        let sp = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.01,
            polarity: 1,
            mz,
            intensity,
            ..Default::default()
        };
        w.spectrum(&sp).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// A source that counts the bytes the builder reads.
struct Counting {
    inner: Cursor<Vec<u8>>,
    read: u64,
}

impl Read for Counting {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n as u64;
        Ok(n)
    }
}

impl Seek for Counting {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[test]
fn a_slice_that_cannot_fit_is_refused_early_and_the_others_build_the_same_bytes() {
    let mut data = dense();
    let size = data.len() as u64;
    let table = prepare(&mut data, size, "dense.mzML").unwrap().table;

    // 1.5 M peaks in one bucket (at least 12 B each): far more than twice 1 MB
    for spill in [false, true] {
        let mut src = Counting {
            inner: Cursor::new(data.clone()),
            read: 0,
        };
        let mut out = Cursor::new(Vec::new());
        let mut scratch = Cursor::new(Vec::new());
        let e = build_mzml_spill(
            &table,
            &mut src,
            &mut out,
            MB,
            spill.then_some(&mut scratch as _),
        )
        .unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
        let p = |k: &str| {
            e.params
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert_eq!(
            (p("fase").as_str(), p("budget_mb").as_str()),
            ("indexPeaks", "2"),
            "spill {spill}: {e:?}"
        );
        // what pass 1 counted before it stopped: already more than the limit
        assert!(
            p("stima_mb").parse::<usize>().unwrap() > 2,
            "spill {spill}: {e:?}"
        );
        assert!(
            src.read * 2 < size,
            "spill {spill}: {} of {size} bytes read before the refusal",
            src.read
        );
        // spilling the whole file would take at least 12 B a peak
        assert!(
            scratch.get_ref().len() * 2 < SCANS * PEAKS * 12,
            "spill {spill}: {} bytes spilled",
            scratch.get_ref().len()
        );
    }

    let mut reference = Cursor::new(Vec::new());
    build_mzml(&table, Cursor::new(&data), &mut reference, 256 * MB).unwrap();
    for (budget, spill) in [
        (24 * MB, false),
        (24 * MB, true),
        (64 * MB, true),
        (512 * MB, true),
    ] {
        let mut out = Cursor::new(Vec::new());
        let mut scratch = Cursor::new(Vec::new());
        build_mzml_spill(
            &table,
            Cursor::new(&data),
            &mut out,
            budget,
            spill.then_some(&mut scratch as _),
        )
        .unwrap();
        assert!(
            out.get_ref() == reference.get_ref(),
            "budget {} MB, spill {spill}: different bytes",
            budget / MB
        );
    }
}
