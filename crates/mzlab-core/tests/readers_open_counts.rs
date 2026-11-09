//! Source reads and bytes of opening a multi-sample `.wiff` pair the way the page does (`crates/mzlab-wasm/src/natives.rs`): the list of samples, one index
//! builder per sample, the engine's reader. A counting `ReadAt` stands in for the browser's synchronous reads. The synthetic pair needs python3
//! (`tools/wiff_riferimento.py`) and skips without it.

use std::cell::Cell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff;

const SAMPLES: usize = 14;

#[derive(Default)]
struct Conti {
    reads: Cell<u64>,
    bytes: Cell<u64>,
}

struct Contato {
    data: Vec<u8>,
    c: Rc<Conti>,
}

impl ReadAt for Contato {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        if std::env::var("TRACE_READS").is_ok() && buf.len() > 4096 {
            eprintln!("read len={} off={} of {}", buf.len(), off, self.data.len());
        }
        self.c.reads.set(self.c.reads.get() + 1);
        self.c.bytes.set(self.c.bytes.get() + buf.len() as u64);
        let s = self
            .data
            .get(off as usize..off as usize + buf.len())
            .ok_or_else(|| mzlab_core::Error::new("error.badIndex"))?;
        buf.copy_from_slice(s);
        Ok(())
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A 14-sample pair of 200 dense profile scans each, written by the Python reference writer; `None` without python3.
fn pair() -> Option<(Vec<Vec<u8>>, u64)> {
    let dir = std::env::temp_dir().join(format!("mzlab_apri_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let code = format!(
        "import sys,random\nsys.path.insert(0,'tools')\nimport wiff_riferimento as W\nfrom pathlib import Path\nrnd=random.Random(3)\nw=W.Writer()\nfor n in range(1,{}):\n    b=[]\n    for k in range(200):\n        p=sorted(set((100+11*j+rnd.randrange(5),1+rnd.randrange(60)) for j in range(300)))\n        b.append(('scan',p,1000.0*k,float(sum(c for _,c in p))))\n    w.sample(n,b,data_name='S%d'%n)\nw.save(Path(sys.argv[1]),'multi')\n",
        SAMPLES + 1
    );
    let ok = Command::new("python3")
        .current_dir(root())
        .args(["-I", "-c", &code])
        .arg(&dir)
        .status()
        .ok()?
        .success();
    if !ok {
        return None;
    }
    let v = vec![
        std::fs::read(dir.join("multi.wiff")).ok()?,
        std::fs::read(dir.join("multi.wiff.scan")).ok()?,
    ];
    let total = v.iter().map(|m| m.len() as u64).sum();
    let _ = std::fs::remove_dir_all(&dir);
    Some((v, total))
}

fn set(dati: &[Vec<u8>], c: &Rc<Conti>) -> FileSet {
    let names = ["multi.wiff", "multi.wiff.scan"];
    FileSet {
        membri: dati
            .iter()
            .zip(names)
            .map(|(d, n)| Member {
                nome: n.into(),
                dim: d.len() as u64,
                dati: Box::new(Contato {
                    data: d.clone(),
                    c: c.clone(),
                }),
                sha1: None,
            })
            .collect(),
    }
}

#[test]
fn opening_every_sample_of_a_wiff_pair_reads_it_about_once() {
    let Some((dati, total)) = pair() else {
        eprintln!("SKIP read_open_counts.rs: python3 not available");
        return;
    };
    let c = Rc::new(Conti::default());
    // the list of samples (`native_samples`)
    let l = readers::apri(set(&dati, &c), &Limits::sito()).unwrap();
    assert_eq!(l.samples().len(), SAMPLES);
    drop(l);
    let list = (c.reads.get(), c.bytes.get());
    // one index builder per sample, each in its own worker (`build_mzidx_native`)
    for s in 0..SAMPLES {
        let mut l = wiff::reader::open_only(set(&dati, &c), &Limits::sito(), Some(s)).unwrap();
        assert!(!l.index(s).unwrap().is_empty());
    }
    let all = (c.reads.get(), c.bytes.get());
    eprintln!(
        "dataset {total} B: list {} reads {} B; + {SAMPLES} builders: {} reads {} B ({:.2}x)",
        list.0,
        list.1,
        all.0,
        all.1,
        all.1 as f64 / total as f64
    );
    // the list: the container and the heads of every block, each byte about once (a window used to run 1 MB past the end of each injection)
    assert!(
        list.1 <= total + total / 10,
        "the list read {} B of {total}",
        list.1
    );
    // a builder reads the container and the heads of its own injection only
    let (contenitore, scan) = (dati[0].len() as u64, dati[1].len() as u64);
    let builders = all.1 - list.1;
    assert!(
        builders <= SAMPLES as u64 * contenitore + scan + scan / 10,
        "the builders read {builders} B of {total}"
    );
}
