//! An index belongs to the file it was built from (`Index::built_from`: size and probe): the same bytes do, a file replaced by another
//! of the same size does not (a change in its first or last 64 KB), a file of another size does not. Synthetic data only.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod fake_reader;

use std::io::Cursor;

use fake_reader::Finto;
use mzlab_core::idxread::Index;
use mzlab_core::{idxsource, upload};

/// An index whose `source` describes `file`, as `prepare` writes it.
fn index_of(file: &[u8]) -> Index<Vec<u8>> {
    let mut f = file.to_vec();
    let probe = upload::probe(&mut f, file.len() as u64).unwrap();
    let mut l = Finto::vario(40, 20, 7);
    let p = upload::prepare_reader(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzlab_core::mzidx::build_reader(
        &p.table,
        &mut l,
        0,
        &mut out,
        mzlab_core::mzidx::DEFAULT_BUDGET,
    )
    .unwrap();
    let b = out.into_inner();
    let size = b.len() as u64;
    let mut ix = Index::open(b, size).unwrap();
    ix.source = mzlab_core::json::parse(
        format!(
            "{{\"name\":\"x.mzML\",\"size\":{},\"probe\":\"{probe}\"}}",
            file.len()
        )
        .as_bytes(),
    )
    .unwrap();
    ix
}

#[test]
fn an_index_belongs_only_to_the_bytes_it_was_built_from() {
    let file: Vec<u8> = (0..300_000u32).map(|i| (i * 7 % 251) as u8).collect();
    let ix = index_of(&file);
    let n = file.len() as u64;
    assert!(ix.built_from(&mut file.clone(), n).unwrap());
    for at in [10, file.len() - 10] {
        let mut other = file.clone();
        other[at] ^= 1;
        assert!(
            !ix.built_from(&mut other, n).unwrap(),
            "a byte changed at {at}"
        );
    }
    let mut longer = file.clone();
    longer.push(0);
    assert!(!ix.built_from(&mut longer, n + 1).unwrap());
    assert!(
        !idxsource::same_print(&mzlab_core::json::Value::Null, n, "x"),
        "a source without size and probe"
    );
}
