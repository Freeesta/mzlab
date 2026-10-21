//! The routes that read a file besides its index, from the mzML and from the reader (helpers of `lettori_mzidx.rs`).
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{self, Value};
use mzlab_core::lettori;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{fileroutes, itemroutes, lettori_mzml, lr};

pub fn mzml_of(l: &mut dyn lettori::Lettore, c: usize) -> Vec<u8> {
    lettori_mzml::scrivi(l, c, Vec::new(), &mut |_, _| {}).unwrap()
}

/// A vendor file as the engine reads it: the mzML without its spectra (`lettori_mzml::guscio`) and the reader for the scans.
struct Nativo<'a> {
    g: Vec<u8>,
    l: &'a mut dyn lettori::Lettore,
    c: usize,
}

impl ReadAt for Nativo<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.g.read_at(off, buf)
    }
    fn lettore(&mut self) -> Option<(&mut dyn lettori::Lettore, usize)> {
        Some((&mut *self.l, self.c))
    }
}

/// The routes that read the file besides its index give the same JSON from the mzML (index `a`) and from the reader (index `b` + `guscio`).
pub fn same_routes(
    label: &str,
    mzml: &[u8],
    a: &[u8],
    b: &[u8],
    l: &mut dyn lettori::Lettore,
    c: usize,
) {
    let g = lettori_mzml::guscio(l, c).unwrap();
    let (sa, sb) = (mzml.len() as u64, g.len() as u64);
    let mut ma = Mzml::new(mzml.to_vec(), sa);
    let mut mb = Mzml::new(Nativo { g, l, c }, sb);
    let mut ra = Routes::new(Index::open(a.to_vec(), a.len() as u64).unwrap());
    let mut rb = Routes::new(Index::open(b.to_vec(), b.len() as u64).unwrap());
    let name = format!("{label}.mzML");
    let sha = |v: String, r: &Routes<Vec<u8>>| {
        let s = r
            .idx
            .source
            .get("sha256")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string();
        v.replace(&s, "SHA")
    };
    let ea = sha(lr::file_entry(&mut ra.idx, &mut ma, &name, 1).unwrap(), &ra);
    let eb = sha(lr::file_entry(&mut rb.idx, &mut mb, &name, 1).unwrap(), &rb);
    assert_eq!(ea, eb, "{label}: lr_entry");
    let ia = lr::open_items(&mut ra.idx, &mut ma, &name, &lr::Sample::default()).unwrap();
    let ib = lr::open_items(&mut rb.idx, &mut mb, &name, &lr::Sample::default()).unwrap();
    assert_eq!(ia.len(), ib.len(), "{label}: items");
    for (x, y) in ia.iter().zip(&ib) {
        assert_eq!(
            (sha(x.info.clone(), &ra), x.hr),
            (sha(y.info.clone(), &rb), y.hr),
            "{label}: lr_items"
        );
        let file = json::parse(x.info.as_bytes())
            .unwrap()
            .get("file")
            .and_then(Value::str)
            .unwrap_or("")
            .to_string();
        assert_eq!(
            itemroutes::item_method(&ra.idx, &mut ma, &file).unwrap(),
            itemroutes::item_method(&rb.idx, &mut mb, &file).unwrap(),
            "{label}: method {file}"
        );
    }
    let (ha, hb) = (ma.info_head().unwrap(), mb.info_head().unwrap());
    let fa = sha(
        fileroutes::fileinfo(&mut ra, &mut ma, &name, &ha).unwrap(),
        &ra,
    );
    let fb = sha(
        fileroutes::fileinfo(&mut rb, &mut mb, &name, &hb).unwrap(),
        &rb,
    );
    assert_eq!(fa, fb, "{label}: fileinfo");
    let (ca, cb) = (
        ma.chromatograms(ra.idx.scans.last().map_or(0, |s| s.end))
            .unwrap(),
        mb.chromatograms(0).unwrap(),
    );
    assert_eq!(
        fileroutes::mrm_transitions(&mut ma, &ca).unwrap(),
        fileroutes::mrm_transitions(&mut mb, &cb).unwrap(),
        "{label}: mrm"
    );
    same_scans(label, &mut ra, &mut rb, &mut ma, &mut mb);
}

/// `scaninfo` and `/api/scan` of every n/10-th scan, from the mzML (`ra`, `ma`) and from the reader (`rb`, `mb`).
fn same_scans(
    label: &str,
    ra: &mut Routes<Vec<u8>>,
    rb: &mut Routes<Vec<u8>>,
    ma: &mut Mzml<Vec<u8>>,
    mb: &mut Mzml<Nativo<'_>>,
) {
    let n = ra.idx.scans.len();
    for sid in (0..n).step_by((n / 10).max(1)) {
        assert_eq!(
            ra.scaninfo(sid, &mut ma.r).unwrap(),
            rb.scaninfo(sid, &mut mb.r).unwrap(),
            "{label}: scaninfo {sid}"
        );
        let q = Query::parse(&format!("?sid={sid}"));
        assert_eq!(
            ra.mzml_route("/api/scan", &q, &mut ma.r).unwrap(),
            rb.mzml_route("/api/scan", &q, &mut mb.r).unwrap(),
            "{label}: scan {sid}"
        );
    }
}
