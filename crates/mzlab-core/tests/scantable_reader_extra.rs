//! The row of a reader's scan whose extras carry the maker's TIC (every `.wiff` scan) and, with `precursori`, a base peak: built straight from the
//! scan (`mzidx_reader::righe`) it equals, field by field and bit for bit, the row read back from the header text `mzmlwrite::spectrum_head` writes
//! (the same scans with a userParam that sends them to the text) and the row of the mzML `readers_mzml::scrivi` writes (finite totals: it refuses
//! the others). Totals that no short decimal holds, 0 and -0, subnormals, huge, negative, infinite and NaN with any payload (written `NaN`).
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]
// the extras are set field by field: some exist only with `precursori`
#![allow(clippy::field_reassign_with_default)]

use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::readers::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::scantable::{Row, Table};
use mzlab_core::{mzidx_reader, readers_mzml, scantable};

struct One {
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
    cards: Vec<(Spectrum, SpectrumExtra)>,
}

impl Reader for One {
    fn format(&self) -> Format {
        Format::Altro("uno")
    }
    fn samples(&self) -> &[Sample] {
        &self.campioni
    }
    fn index(&mut self, _c: usize) -> Result<&[ScanEntry]> {
        Ok(&self.indice)
    }
    fn scan(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        self.cards
            .get(k)
            .map(|s| s.0.clone())
            .ok_or_else(|| Error::new("error.badIndex"))
    }
    fn scan_header(&mut self, _c: usize, _k: usize) -> Result<Vec<Field>> {
        Ok(Vec::new())
    }
    fn chromatograms(&mut self, _c: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(Vec::new())
    }
    fn chromatogram(&mut self, _c: usize, _id: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn method(&mut self, _c: usize) -> Result<String> {
        Ok(String::new())
    }
    fn mzml_header(&mut self, _c: usize) -> Result<Option<Header>> {
        Ok(None)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.cards
            .get(s.index)
            .map(|x| x.1.clone())
            .unwrap_or_default()
    }
}

/// xorshift: the same scans on every run.
struct Case(u64);
impl Case {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// A total of scan `k`: fixed edge values first, then any bit pattern (every NaN payload and sign, subnormals, infinities) and decimals with no short text.
fn total(r: &mut Case, k: usize) -> f64 {
    const EDGES: [f64; 14] = [
        0.0,
        -0.0,
        0.1,
        1.0 / 3.0,
        5e-324,
        2.2250738585072014e-308,
        1.7976931348623157e308,
        -1234.5678901234567,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        1e21,
        123456789012345680.0,
    ];
    match k {
        k if k < EDGES.len() => EDGES[k],
        _ if k.is_multiple_of(7) => f64::from_bits(0x7ff0_0000_0000_0001 | r.next()), // NaN, any payload and sign
        _ if k.is_multiple_of(3) => f64::from_bits(r.next()),
        _ => (r.unit() - 0.25) * 10f64.powi((r.next() % 40) as i32 - 10),
    }
}

/// `n` scans; `finiti`: the totals the mzML converter accepts; `to_text`: a userParam naming an accession in every scan, so `righe` reads the row
/// from the header text (it names no accession a row reads: the row is the same).
fn reader(n: usize, seed: u64, finished: bool, to_text: bool) -> One {
    let fin = |v: f64| if finished && !v.is_finite() { 0.5 } else { v };
    let mut r = Case(seed | 1);
    let mut schede = Vec::new();
    for k in 0..n {
        let level = if k.is_multiple_of(3) { 1 } else { 2 };
        let np = if k % 5 == 4 {
            0
        } else {
            1 + (r.next() % 30) as usize
        };
        let mut mz: Vec<f64> = (0..np).map(|_| 50.0 + r.unit() * 1000.0).collect();
        mz.sort_by(f64::total_cmp);
        let intensity: Vec<f32> = (0..np).map(|_| (r.unit() * 1e5) as f32).collect();
        let sp = Spectrum {
            index: k,
            id: format!("sample=1 period=1 cycle={} experiment={}", k / 3, k % 3),
            level,
            rt: k as f64 * 0.011 + r.unit() * 1e-3,
            polarity: [1, -1, 0][k % 3],
            profile: k % 4 == 1,
            filter: String::new(),
            precursor: (level > 1).then(|| 200.0 + r.unit() * 500.0),
            mz,
            intensity,
            wide: None,
        };
        let mut extra = SpectrumExtra::default();
        // one scan in eleven without the maker's TIC: the sum of the points
        extra.tic = (k % 11 != 10).then(|| fin(total(&mut r, k)));
        if to_text {
            extra.params.push(("note".into(), "see MS:1000927".into()));
        }
        extra.window = k.is_multiple_of(2).then_some((50.0, 1050.0));
        if level > 1 {
            extra.ce = Some(10.0 + (k % 4) as f64 * 5.0);
            extra.iso_half = (k % 2 == 1).then_some(0.35);
        }
        #[cfg(feature = "precursori")]
        {
            extra.base_peak =
                (k % 4 != 3).then(|| (fin(total(&mut r, k + 3)), fin(total(&mut r, k + 5))));
        }
        schede.push((sp, extra));
    }
    let indice = schede
        .iter()
        .map(|(s, _)| ScanEntry {
            id: s.id.clone(),
            rt: s.rt,
            livello: s.level,
            polarita: s.polarity,
            tic: 0.0,
            base_peak: None,
            filtro: s.filter.clone(),
            precursore: s.precursor,
            position: None,
        })
        .collect();
    One {
        campioni: vec![Sample {
            id: 1,
            nome: "uno".into(),
            scansioni: schede.len(),
            size: None,
        }],
        indice,
        cards: schede,
    }
}

fn bits(v: Option<f64>) -> Option<u64> {
    v.map(f64::to_bits)
}

/// Every field of the two rows but the bytes of the scan in the mzML (a reader's row has none); the floats by their bits.
fn equal(k: usize, a: &Row, b: &Row) {
    assert_eq!(a.level, b.level, "row {k}: level");
    assert_eq!(a.rt.to_bits(), b.rt.to_bits(), "row {k}: rt");
    assert_eq!(a.polarity, b.polarity, "row {k}: polarity");
    assert_eq!(
        a.tic.to_bits(),
        b.tic.to_bits(),
        "row {k}: tic {} vs {}",
        a.tic,
        b.tic
    );
    assert_eq!(a.filter, b.filter, "row {k}: filter");
    assert_eq!(a.profile, b.profile, "row {k}: profile");
    assert_eq!(a.an, b.an, "row {k}: analyzer");
    assert_eq!(a.native, b.native, "row {k}: native id");
    assert_eq!(a.kind, b.kind, "row {k}: kind");
    assert_eq!(
        a.mob.map(|(v, u)| (v.to_bits(), u)),
        b.mob.map(|(v, u)| (v.to_bits(), u)),
        "row {k}: mobility"
    );
    assert_eq!(
        bits(a.resolving_power()),
        bits(b.resolving_power()),
        "row {k}: resolution"
    );
    assert_eq!(
        bits(a.precursor()),
        bits(b.precursor()),
        "row {k}: precursor"
    );
    // the private parts (parent, the precursor detail) by their text, which writes every finite float exactly
    let without = |r: &Row| {
        let mut r = r.clone();
        (r.start, r.end, r.at) = (0, 0, None);
        format!("{r:?}")
    };
    assert_eq!(without(a), without(b), "row {k}");
}

fn compare(label: &str, a: &Table, b: &Table) {
    assert_eq!(a.rows.len(), b.rows.len(), "{label}: rows");
    for (k, (x, y)) in a.rows.iter().zip(&b.rows).enumerate() {
        equal(k, x, y);
    }
    assert_eq!(a.hr1, b.hr1, "{label}: hr1");
    assert_eq!(a.run_json(), b.run_json(), "{label}: run");
}

const CASI: [(usize, u64); 4] = [(1, 1), (40, 2), (500, 3), (3000, 4)];

/// Any total, against the header text `righe` writes and reads back for a scan it cannot build straight.
#[test]
fn reader_tic_as_from_text() {
    for (n, seed) in CASI {
        let direct = mzidx_reader::righe(&mut reader(n, seed, false, false), 0).unwrap();
        let text = mzidx_reader::righe(&mut reader(n, seed, false, true), 0).unwrap();
        compare(&format!("{n} scans"), &direct, &text);
    }
}

/// Finite totals, against the mzML the converter writes, read back by `scantable`.
#[test]
fn reader_tic_as_from_mzml() {
    for (n, seed) in CASI {
        let mut l = reader(n, seed, true, false);
        let direct = mzidx_reader::righe(&mut l, 0).unwrap();
        let mut mzml = readers_mzml::scrivi(&mut l, 0, Vec::new(), &mut |_, _| {}).unwrap();
        let size = mzml.len() as u64;
        let text = scantable::read(&mut mzml, size).unwrap();
        compare(&format!("{n} scans, mzML"), &direct, &text);
    }
}

/// The edges are in the scans: a NaN with a payload or a sign (the text keeps neither), -0, the infinities, subnormals, negative totals.
#[test]
fn the_edges_are_there() {
    let l = reader(3000, 4, false, false);
    let tic: Vec<f64> = l.cards.iter().filter_map(|s| s.1.tic).collect();
    assert!(tic
        .iter()
        .any(|v| v.is_nan() && v.to_bits() != f64::NAN.to_bits()));
    assert!(tic.iter().any(|v| *v == 0.0 && v.is_sign_negative()));
    assert!(tic.iter().any(|v| v.is_infinite()));
    assert!(tic.iter().any(|v| v.is_subnormal()));
    assert!(tic.iter().any(|v| *v < 0.0 && v.is_finite()));
}
