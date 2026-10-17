//! `upload::Passata` (SHA-256, SHA-1 and probe in one pass, fed in pieces) gives what the three separate reads gave: the same values whatever the pieces.
use mzlab_core::lettori::{FileSet, Membro};
use mzlab_core::upload::{self, Passata};
use sha1::{Digest as _, Sha1};
use sha2::Sha256;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn bytes(n: usize) -> Vec<u8> {
    let mut x = 0x9e37_79b9u32;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

/// The three values as the file was read by the old `fingerprint` and `sha1_hex`: each from the whole bytes.
fn reference(b: &[u8]) -> (String, String, String) {
    let probe = 65536.min(b.len());
    let mut p = Sha1::new();
    p.update(b.len().to_string().as_bytes());
    p.update(&b[..probe]);
    p.update(&b[b.len() - probe..]);
    (
        hex(&Sha256::digest(b)),
        hex(&Sha1::digest(b)),
        hex(&p.finalize()),
    )
}

#[test]
fn passata_equals_the_whole_file_hashes_whatever_the_pieces() {
    for n in [0, 1, 100, 65535, 65536, 65537, 131_072, 200_000, 3_145_733] {
        let b = bytes(n);
        let want = reference(&b);
        for piece in [1usize << 20, 4096, 7, 65536, 100_000] {
            let mut p = Passata::new(n as u64);
            for c in b.chunks(piece.max(1)) {
                p.aggiorna(c);
            }
            assert_eq!(p.fine(), Some(want.clone()), "size {n}, pieces {piece}");
        }
        assert_eq!(
            upload::fingerprint3(&mut b.clone(), n as u64).unwrap(),
            want
        );
    }
}

#[test]
fn a_short_or_long_pass_gives_nothing() {
    let b = bytes(1000);
    let mut p = Passata::new(1001);
    p.aggiorna(&b);
    assert_eq!(p.fine(), None);
    let mut p = Passata::new(999);
    p.aggiorna(&b);
    assert_eq!(p.fine(), None);
}

fn set(b: &[u8]) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b.to_vec()),
            sha1: None,
        }],
    }
}

#[test]
fn impronte_gives_the_sha1_to_the_member_and_the_known_ones_replace_the_read() {
    let b = bytes(300_000);
    let (sha256, sha1, probe) = reference(&b);
    let mut fs = set(&b);
    let imp = upload::impronte(&mut fs).unwrap();
    assert_eq!(imp.len(), 1);
    assert_eq!(
        (
            imp[0].sha256.clone(),
            imp[0].sha1.clone(),
            imp[0].probe.clone()
        ),
        (sha256.clone(), sha1.clone(), probe.clone())
    );
    assert_eq!(fs.membri[0].sha1.as_deref(), Some(sha1.as_str()));
    // taken before: the same, without reading the member
    let mut fs = set(&[]);
    fs.membri[0].dim = b.len() as u64;
    let known =
        upload::impronte_note(&mut fs, &[(sha256.clone(), sha1.clone(), probe.clone())]).unwrap();
    assert_eq!(known, imp);
    assert_eq!(fs.membri[0].sha1.as_deref(), Some(sha1.as_str()));
    // not as many as the members: the caller reads them
    assert!(upload::impronte_note(&mut set(&b), &[]).is_none());
}

/// The fingerprints taken while the file is copied (`Passata` fed in pieces) given to `prepare_con` give the same `Prepared` as the full read, and the file is read once less.
#[test]
fn prepare_with_known_fingerprints_is_identical_and_skips_the_reread() {
    use mzlab_core::idxread::ReadAt;
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};

    struct Counting(Vec<u8>, u64);
    impl ReadAt for Counting {
        fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
            self.1 += buf.len() as u64;
            self.0.read_at(off, buf)
        }
    }

    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200usize {
        w.spectrum(&Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt: 0.05 * i as f64,
            polarity: 1,
            profile: false,
            filter: String::new(),
            precursor: None,
            mz: (0..40).map(|b| 100.0 + b as f64 * 1.5).collect(),
            intensity: (0..40).map(|b| 10.0 + ((i * 7 + b) % 50) as f32).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;

    let mut full = Counting(mzml.clone(), 0);
    let a = upload::prepare(&mut full, size, "x.mzML").unwrap();

    let mut pass = Passata::new(size);
    for c in mzml.chunks(8 << 20) {
        pass.aggiorna(c);
    }
    let (sha256, _, probe) = pass.fine().unwrap();
    let mut quick = Counting(mzml, 0);
    let b = upload::prepare_con(
        &mut quick,
        size,
        "x.mzML",
        Some((sha256.clone(), probe.clone())),
    )
    .unwrap();

    assert_eq!(
        (a.sha256.as_str(), a.probe.as_str()),
        (sha256.as_str(), probe.as_str())
    );
    assert_eq!(a.table.source_json, b.table.source_json);
    assert_eq!(a.table.scans_json, b.table.scans_json);
    assert_eq!(a.table.run_json, b.table.run_json);
    assert_eq!((a.table.rt, a.table.tic), (b.table.rt, b.table.tic));
    assert_eq!((a.sha256, a.probe), (b.sha256, b.probe));
    assert!(quick.1 < full.1, "{} vs {}", quick.1, full.1);
}
