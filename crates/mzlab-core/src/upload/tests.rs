use super::*;
use sha1::{Digest as _, Sha1};

/// A file in memory that, like a JS callback might, refuses a read of length 0.
struct Strict(Vec<u8>);

impl ReadAt for Strict {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        assert!(!buf.is_empty(), "a read of length 0");
        self.0.read_at(off, buf)
    }
}

fn bytes(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| (i.wrapping_mul(31) ^ (i >> 7)) as u8)
        .collect()
}

/// The probe from its definition: SHA-1 of the size, the first and the last 64 KB.
fn by_definition(b: &[u8]) -> String {
    let n = b.len().min(PROBE as usize);
    let mut p = Sha1::new();
    p.update(b.len().to_string().as_bytes());
    p.update(&b[..n]);
    p.update(&b[b.len() - n..]);
    hex(&p.finalize())
}

#[test]
fn hex_is_lowercase_two_digits_per_byte() {
    assert_eq!(hex(&[]), "");
    assert_eq!(hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
}

#[test]
fn passata_and_probe_agree_for_empty_small_and_large_files_in_one_two_or_three_pieces() {
    for size in [
        0usize,
        1,
        20 * 1024,
        PROBE as usize - 1,
        PROBE as usize,
        PROBE as usize + 1,
        3 * PROBE as usize + 5,
        200_000,
    ] {
        let b = bytes(size);
        let want = by_definition(&b);
        assert_eq!(
            probe(&mut Strict(b.clone()), size as u64).unwrap(),
            want,
            "probe {size}"
        );
        let cuts = [
            vec![],
            vec![size / 2],
            vec![size / 3, 2 * size / 3],
            vec![1.min(size), size.saturating_sub(1)],
            vec![PROBE as usize / 2],
        ];
        for cut in cuts {
            let mut p = Passata::new(size as u64);
            let mut from = 0;
            for c in cut.into_iter().chain([size]) {
                let c = c.clamp(from, size);
                p.aggiorna(&b[from..c]);
                from = c;
            }
            let (_, _, probe_p) = p.fine().unwrap();
            assert_eq!(probe_p, want, "passata {size}");
        }
    }
}
