#![cfg(feature = "waters")]
//! A damaged part of a Waters folder among whole ones (synthetic `flags_folder`, two scanning functions): a record no acquisition writes, an index cut
//! inside a row, a `_FUNCTNS.INF` cut at a whole record, a `_HEADER.TXT` cut inside a calibration line, an analog trace that cannot be read. The part is
//! left out and counted (`warn.waters.damaged`, or the notice of the traces), everything else reads as in the whole folder.
mod common_waters;
use common_waters::*;
use mzlab_core::mzml::Spectrum;
use mzlab_core::readers::{FileSet, Limits, Losses, Member, WATERS_DAMAGED};

type Files = Vec<(String, Vec<u8>)>;

struct Read {
    scans: Vec<Spectrum>,
    losses: Losses,
    chroms: Vec<String>,
    method: String,
}

fn read_set(fs: FileSet) -> Read {
    let mut l =
        mzlab_lettori::open_bare(fs, &Limits::sito()).unwrap_or_else(|e| panic!("{}", e.key));
    let n = l.index(0).unwrap().len();
    assert_eq!(l.samples()[0].scansioni, n, "the sample counts the index");
    let scans = (0..n).map(|k| l.scan(0, k).unwrap()).collect();
    let losses = l.losses(0).unwrap();
    assert_eq!(l.missing(0), losses.lost(), "missing is what losses lost");
    let chroms = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    Read {
        scans,
        losses,
        chroms,
        method: l.method(0).unwrap(),
    }
}

fn read(f: Files) -> Read {
    read_set(file_set(f))
}

fn bytes<'a>(f: &'a mut Files, name: &str) -> &'a mut Vec<u8> {
    &mut f.iter_mut().find(|x| x.0 == name).unwrap().1
}

/// The scans of function `k` of the whole folder.
fn of_function(r: &Read, k: u32) -> Vec<&Spectrum> {
    let tag = format!("function={k} ");
    r.scans.iter().filter(|s| s.id.starts_with(&tag)).collect()
}

/// Every scan of `part` is the scan of the same id in `whole`, points and all.
fn is_part(part: &Read, whole: &Read) {
    for s in &part.scans {
        let w = whole
            .scans
            .iter()
            .find(|w| w.id == s.id)
            .expect("a scan of the whole folder");
        assert_eq!((&s.mz, &s.intensity), (&w.mz, &w.intensity), "{}", s.id);
    }
}

#[test]
fn a_damaged_record_leaves_out_its_scan_only() {
    let whole = read(flags_folder(0x4C));
    let mut f = flags_folder(0x4C);
    // scan 2 of function 1 (24-byte scans): an intensity word without its leading bit
    bytes(&mut f, "_FUNC001.DAT")[24..28].copy_from_slice(&1u32.to_le_bytes());
    let r = read(f);
    let n = whole.scans.len();
    assert_eq!(r.losses, Losses::of(n, n - 1, WATERS_DAMAGED));
    assert!(!r
        .scans
        .iter()
        .any(|s| s.id == "function=1 process=0 scan=2"));
    is_part(&r, &whole);
    assert!(
        r.method.contains("function 1: 1 damaged scans left out"),
        "{}",
        r.method
    );
}

#[test]
fn an_index_cut_inside_a_row_leaves_out_its_function() {
    let whole = read(flags_folder(0x4C));
    let mut f = flags_folder(0x4C);
    let idx = bytes(&mut f, "_FUNC002.IDX");
    let rows = idx.len() / 22;
    idx.truncate(idx.len() - 5);
    let r = read(f);
    let n1 = of_function(&whole, 1).len();
    assert_eq!(r.losses, Losses::of(n1 + rows - 1, n1, WATERS_DAMAGED));
    assert_eq!(r.scans.len(), n1);
    is_part(&r, &whole);
    assert!(
        r.method.contains("_FUNC002.IDX cut inside a row"),
        "{}",
        r.method
    );
}

#[test]
fn a_function_list_cut_at_a_whole_record_leaves_out_the_rest() {
    let whole = read(flags_folder(0x4C));
    let mut f = flags_folder(0x4C);
    bytes(&mut f, "_FUNCTNS.INF").truncate(416);
    let r = read(f);
    let (n1, n2) = (of_function(&whole, 1).len(), of_function(&whole, 2).len());
    assert_eq!(r.losses, Losses::of(n1 + n2, n1, WATERS_DAMAGED));
    is_part(&r, &whole);
    assert!(
        r.method
            .contains("1 functions past the end of _FUNCTNS.INF"),
        "{}",
        r.method
    );
}

#[test]
fn a_calibration_line_cut_leaves_out_its_function_never_uncalibrated() {
    let whole = read(flags_folder(0x4C));
    let mut f = flags_folder(0x4C);
    let (a, b) = FLAGS_CAL;
    let cut = format!("$$ Acquired Name: FLAGS_001\r\n$$ Cal Function 1: {a},{b},T1\r\n$$ Cal Function 2: {a},1.00");
    *bytes(&mut f, "_HEADER.TXT") = cut.into_bytes();
    let r = read(f);
    let (n1, n2) = (of_function(&whole, 1).len(), of_function(&whole, 2).len());
    assert_eq!(r.losses, Losses::of(n1 + n2, n1, WATERS_DAMAGED));
    is_part(&r, &whole);
    assert!(
        r.method.contains("no whole Cal Function line"),
        "{}",
        r.method
    );
}

#[test]
fn a_folder_with_nothing_left_is_still_refused() {
    let mut f = flags_folder(0x4C);
    for k in ["_FUNC001.IDX", "_FUNC002.IDX"] {
        let idx = bytes(&mut f, k);
        idx.truncate(idx.len() - 5);
    }
    let e = mzlab_lettori::open_bare(file_set(f), &Limits::sito())
        .err()
        .expect("refused");
    assert_eq!(e.key, "err.file.indexCut");
}

/// `_CHROMS.INF` with traces `names` and their `_CHROnnn.DAT` (8-byte points after a 128-byte head).
fn with_traces(f: &mut Files, names: &[&str]) {
    let mut inf = vec![0u8; 128];
    for n in names {
        let mut r = vec![0u8; 85];
        let t = format!("{n}\0$CC$,1.0,0,0,100,psi");
        r[4..4 + t.len()].copy_from_slice(t.as_bytes());
        inf.extend(r);
    }
    f.push(("_CHROMS.INF".into(), inf));
    for k in 1..=names.len() {
        let mut d = vec![0u8; 128];
        for i in 0..5 {
            d.extend((i as f32 * 0.5).to_le_bytes());
            d.extend((100.0f32 + i as f32).to_le_bytes());
        }
        f.push((format!("_CHRO{k:03}.DAT"), d));
    }
}

#[test]
fn an_unreadable_analog_trace_leaves_out_that_trace_only() {
    let mut f = flags_folder(0x4C);
    with_traces(&mut f, &["Pressure A", "Pressure B"]);
    let whole = read(f.clone());
    assert!(
        whole.chroms.iter().any(|c| c == "Pressure B"),
        "{:?}",
        whole.chroms
    );
    // the second trace's member says it holds more bytes than the small members may: it cannot be read
    let mut fs = file_set(f);
    let m: &mut Member = fs
        .membri
        .iter_mut()
        .find(|m| m.nome.ends_with("_CHRO002.DAT"))
        .unwrap();
    m.dim = 9 << 20;
    let r = read_set(fs);
    assert_eq!(r.losses, whole.losses);
    assert_eq!(r.scans.len(), whole.scans.len());
    let want: Vec<_> = whole
        .chroms
        .iter()
        .filter(|c| *c != "Pressure B")
        .cloned()
        .collect();
    assert_eq!(r.chroms, want);
    assert!(
        r.method
            .contains("1 analog traces unreadable, left out: 2 (Pressure B)"),
        "{}",
        r.method
    );
}
