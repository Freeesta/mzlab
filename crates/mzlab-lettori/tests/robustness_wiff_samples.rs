//! A synthetic `.wiff` whose sample table lists thousands of samples and no methods: opening it costs a pass over the container per sample, never a
//! probe of every method number below each sample (1 000 samples were 500 000 searches over 1 000 paths, 18 s in a debug build).
#![cfg(feature = "sciex")]

mod common;

use mzlab_core::readers::{FileSet, Limits, Member};

fn pair(samples: u32) -> FileSet {
    let mut table = vec![0u8; 32];
    for s in 1..=samples {
        table.extend(s.to_le_bytes());
        table.extend((20 + 100 * s).to_le_bytes());
        table.extend([0u8; 8]);
    }
    // each sample has an index of one empty row, so the container holds as many streams as samples
    let names: Vec<String> = (1..=samples)
        .map(|s| format!("SampleSubtree/Sample{s}/Idx"))
        .collect();
    let mut streams = vec![("SampleSubtree/SampleIdxTable", table)];
    streams.extend(names.iter().map(|n| (n.as_str(), vec![0u8; 32 + 54])));
    let wiff = common::ole::ole(&streams);
    let member = |nome: &str, b: Vec<u8>| Member {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    FileSet {
        membri: vec![member("x.wiff", wiff), member("x.wiff.scan", vec![0u8; 64])],
    }
}

#[test]
fn a_sample_table_of_1000_samples_opens_in_linear_time() {
    let t = std::time::Instant::now();
    let r = mzlab_core::wiff::reader::open_only(pair(1000), &Limits::desktop(), None);
    let elapsed = t.elapsed();
    eprintln!("1000 samples: {elapsed:?} ok={}", r.is_ok());
    assert!(elapsed.as_secs() < 5, "{elapsed:?}");
}
