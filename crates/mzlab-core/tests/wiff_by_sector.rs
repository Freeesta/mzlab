//! The `.wiff` of a pair is read by sector: opening the pair does not read the DAD stream of the samples (most of a real `.wiff`), it is read when it is asked for,
//! and a sample opened alone (the index worker of the page) gives what the whole pair gives. Synthetic pair of `tools/wiff_dad_mrm.py`, skipped without python3.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{FileSet, Limits, Member, Reader};
use mzlab_core::wiff::reader::open_only;

/// A file in memory that counts the bytes read from it.
struct Counting(Vec<u8>, Rc<Cell<u64>>);

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
        self.1.set(self.1.get() + buf.len() as u64);
        Ok(())
    }
}

fn set(dir: &Path, name: &str, sha1: Option<String>) -> (FileSet, Rc<Cell<u64>>, u64) {
    let read = Rc::new(Cell::new(0));
    let (mut size, mut members) = (0, vec![]);
    for (k, file) in [format!("{name}.wiff"), format!("{name}.wiff.scan")]
        .into_iter()
        .enumerate()
    {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        if k == 0 {
            size = bytes.len() as u64;
        }
        let dati: Box<dyn ReadAt> = if k == 0 {
            Box::new(Counting(bytes.clone(), read.clone()))
        } else {
            Box::new(bytes.clone())
        };
        members.push(Member {
            nome: file,
            dim: bytes.len() as u64,
            dati,
            sha1: sha1.clone(),
        });
    }
    (FileSet { membri: members }, read, size)
}

fn digest(r: &mut Box<dyn Reader>, c: usize) -> String {
    let mut s = format!("scans={}\n", r.index(c).unwrap().len());
    for e in r.chromatograms(c).unwrap() {
        let h = r.chromatogram(c, &e.id).unwrap();
        s += &format!("{} {:?} {:?}\n", e.id, h.time, h.intensity);
    }
    s + &format!(
        "{:?}\n",
        r.dad_data(c)
            .unwrap()
            .map(|d| (d.points(), d.channels(), d.twc_mau.clone()))
    )
}

#[test]
fn the_dad_stream_is_read_when_asked_for_and_a_sample_alone_equals_the_pair() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_sector_{}", std::process::id()));
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/wiff_dad_mrm.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_by_sector: python3 not available");
        return;
    }
    let limits = Limits::sito();
    let (fs, read, size) = set(&out, "dadscan", Some("0".repeat(40)));
    let mut whole = open_only(fs, &limits, None).unwrap();
    let opened = read.get();
    assert!(
        opened < size,
        "opening read {opened} of {size} bytes: the DAD stream must wait"
    );
    let first = digest(&mut whole, 0);
    assert!(read.get() > opened, "the DAD stream was never read");
    let (fs, _, _) = set(&out, "dadscan", Some("0".repeat(40)));
    let mut alone = open_only(fs, &limits, Some(0)).unwrap();
    assert_eq!(first, digest(&mut alone, 0));
}

/// Every sample of a pair of several injections (`tools/wiff_isolation.py`) opened alone gives what the whole pair gives for it, with no SHA-1 given.
#[test]
fn each_sample_of_a_multi_sample_pair_opened_alone_equals_the_whole_pair() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_multi_{}", std::process::id()));
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/wiff_isolation.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_by_sector: python3 not available");
        return;
    }
    let limits = Limits::sito();
    let (fs, _, _) = set(&out, "sano", None);
    let mut whole = open_only(fs, &limits, None).unwrap();
    let n = whole.samples().len();
    assert!(n > 1, "the fixture must hold several samples");
    for c in 0..n {
        let (fs, _, _) = set(&out, "sano", None);
        let mut alone = open_only(fs, &limits, Some(c)).unwrap();
        assert_eq!(digest(&mut whole, c), digest(&mut alone, c), "sample {c}");
        let (fs, _, _) = set(&out, "sano", None);
        let mut bare = mzlab_core::wiff::reader::open_bare(fs, &limits, c).unwrap();
        assert_eq!(
            digest(&mut whole, c),
            digest(&mut bare, c),
            "bare sample {c}"
        );
    }
}

/// A pair opened with the `PARTIAL` mark tells its source by the ranges of the sample: the same sample gives the same identity, another sample another one,
/// and the scans are the ones the whole pair gives.
#[test]
fn a_partial_identity_follows_the_sample_and_leaves_the_scans_alone() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_partial_{}", std::process::id()));
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/wiff_isolation.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_by_sector: python3 not available");
        return;
    }
    let limits = Limits::sito();
    let id = |c: usize| {
        let (fs, _, _) = set(&out, "sano", Some(mzlab_core::wiff::reader::PARTIAL.into()));
        let mut r = open_only(fs, &limits, Some(c)).unwrap();
        (r.source_identity(c).unwrap().unwrap(), digest(&mut r, c))
    };
    let (a0, d0) = id(0);
    let (a1, _) = id(1);
    assert!(a0.starts_with("partial(sample="), "{a0}");
    assert_eq!(a0, id(0).0, "same file and sample, same identity");
    assert_ne!(a0, a1, "another sample, another identity");
    let (fs, _, _) = set(&out, "sano", None);
    let mut whole = open_only(fs, &limits, None).unwrap();
    assert_eq!(whole.source_identity(0).unwrap(), None);
    assert_eq!(d0, digest(&mut whole, 0));
}
