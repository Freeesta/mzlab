//! The builder output is byte-for-byte stable: a deterministic synthetic library hashes to the value recorded from the implementation that
//! allocated its buffers on every call (the buffer reuse must not change a single byte of the `.mzlib`).

use mzlab_lib::build::{BuildOpts, Builder};
use mzlab_lib::sha256::sha256;
use mzlab_lib::spec::{Meta, Spectrum};

fn library_sha() -> String {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut b = Builder::new(Vec::<u8>::new(), BuildOpts::default());
    for i in 0..3000usize {
        let prec = 100.0 + next() * 800.0;
        let n = 3 + (next() * 40.0) as usize;
        let mut mz: Vec<f64> = (0..n)
            .map(|_| (50.0 + next() * 800.0).min(prec - 1.0))
            .collect();
        mz.sort_by(|a, b| a.total_cmp(b));
        mz.dedup_by(|a, b| (*a - *b).abs() < 0.02);
        let it = mz.iter().map(|_| 1.0 + next() * 999.0).collect();
        let s = Spectrum {
            prec,
            pol: if i % 7 == 0 { -1 } else { 1 },
            mz,
            it,
            meta: Meta {
                name: format!("Compound {i}"),
                adduct: "[M+H]+".into(),
                inchikey: format!("{:014X}-ABCDEFGHIJ-N", i % 900),
                accession: format!("ACC{i:07}"),
                ..Meta::default()
            },
            rt: if i % 3 == 0 {
                Some(next() * 20.0)
            } else {
                None
            },
            ri: None,
            ei: false,
        };
        b.add(&s).unwrap();
    }
    let mut out = Vec::new();
    b.finish(&mut out).unwrap();
    sha256(&[&out[..]])
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[test]
fn built_library_bytes_are_stable() {
    assert_eq!(
        library_sha(),
        "5ccd306e9194f7e4a50e3e7b2584542c665def97af895eb127a5749ac69cab15"
    );
}
