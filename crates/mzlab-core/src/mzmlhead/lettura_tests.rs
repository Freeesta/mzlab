//! Chromatograms read together in spans against one read per chromatogram: the same arrays bit for bit, and a few reads for thousands of SRM traces.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_precision_loss)]
use super::super::{Chrom, Mzml};
use crate::error::Result;
use crate::idxread::ReadAt;
use base64::Engine;

/// A file in memory that counts its reads.
struct Counting {
    data: Vec<u8>,
    calls: usize,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, out: &mut [u8]) -> Result<()> {
        self.calls += 1;
        self.data.read_at(off, out)
    }
}

fn b64(v: &[f64]) -> String {
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// An mzML without scans: `n` SRM traces of `len` points. Every 4th has its times in seconds, every 5th declares one point too many
/// (the reader then falls back to the full read), every 7th is preceded by a TIC that is not an SRM trace; `big` adds a trace longer than a span.
fn mrm(n: usize, len: usize, big: bool) -> Vec<u8> {
    let mut s = String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><chromatogramList count=\"0\">\n");
    let mut index = 0;
    let mut one = |s: &mut String, id: String, len: usize, declared: usize, unit: &str, shift: f64| {
        let t: Vec<f64> = (0..len).map(|k| shift + k as f64 * 0.01).collect();
        let y: Vec<f64> = (0..len).map(|k| (k * 7 % 13) as f64).collect();
        s.push_str(&format!(
            "<chromatogram index=\"{index}\" id=\"{id}\" defaultArrayLength=\"{declared}\"><binaryDataArrayList count=\"2\">\
             <binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000595\" unitName=\"{unit}\"/><binary>{}</binary></binaryDataArray>\
             <binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000515\"/><binary>{}</binary></binaryDataArray>\
             </binaryDataArrayList></chromatogram>\n",
            b64(&t),
            b64(&y)
        ));
        index += 1;
    };
    for i in 0..n {
        if i % 7 == 3 {
            one(&mut s, "TIC".into(), len, len, "minute", 0.0);
        }
        let unit = if i % 4 == 1 { "second" } else { "minute" };
        let declared = if i % 5 == 2 { len + 1 } else { len };
        one(&mut s, format!("SRM SIC Q1=100.{i} Q3=50.{i}"), len, declared, unit, i as f64 * 0.1);
        if big && i == n / 2 {
            one(&mut s, "SRM SIC Q1=300.5 Q3=150.5".into(), 1_100_000, 1_100_000, "minute", 2.0);
        }
    }
    s.push_str("</chromatogramList></run></mzML>\n");
    s.into_bytes()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn spans_give_the_arrays_of_one_read_per_chromatogram() {
    let data = mrm(40, 50, true);
    let mut m = Mzml::new(data.clone(), data.len() as u64);
    let chroms = m.chromatograms(0).unwrap();
    let srm: Vec<&Chrom> = chroms.iter().filter(|c| c.kind == "srm").collect();
    assert_eq!(srm.len(), 41);
    let (mut pairs, mut times) = (vec![], vec![]);
    m.chromatograms_each(&srm, |c, (t, y)| pairs.push((c.index, bits(&t), bits(&y))))
        .unwrap();
    m.chromatograms_times(&srm, |t| times.push(bits(&t))).unwrap();
    let mut fell_back = 0;
    for (k, c) in srm.iter().enumerate() {
        let (t, y) = m.chromatogram(c).unwrap();
        assert_eq!(pairs[k], (c.index, bits(&t), bits(&y)), "chromatogram {}", c.index);
        let one = match m.chromatogram_times(c).unwrap() {
            Some(t) => t,
            None => {
                fell_back += 1;
                m.chromatogram(c).unwrap().0
            }
        };
        assert_eq!(times[k], bits(&one), "times of chromatogram {}", c.index);
    }
    assert!(fell_back >= 8, "the fallback path is exercised: {fell_back}");
}

#[test]
fn thousands_of_srm_traces_take_a_few_reads() {
    let data = mrm(4000, 30, false);
    let size = data.len() as u64;
    let mut m = Mzml::new(Counting { data, calls: 0 }, size);
    let chroms = m.chromatograms(0).unwrap();
    let before = m.r.calls;
    let facts = crate::iteminfo::facts_from(&mut m, 0, chroms.len() as i64, true).unwrap();
    assert!(facts.srm_rt.is_some());
    let facts_reads = m.r.calls - before;
    let before = m.r.calls;
    crate::fileroutes::mrm_transitions(&mut m, &chroms).unwrap();
    let mrm_reads = m.r.calls - before;
    // one read per trace was 4000+ for each; the chromatogram list and the header scans are a handful of windows
    assert!(facts_reads < 60, "facts_from: {facts_reads} reads");
    assert!(mrm_reads < 20, "mrm_transitions: {mrm_reads} reads");
}
