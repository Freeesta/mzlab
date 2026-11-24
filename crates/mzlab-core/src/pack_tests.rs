//! `pack/1` in memory: round trips above the part size, determinism, damaged and missing parts, unsafe manifests, files that change while read.

use std::collections::BTreeMap;
use std::io::Read;

use crate::error::{Error, Result};
use crate::pack::{self, Manifest, Options, PartSink};
use crate::pack_read::{self, FileSink, FileState, PartSource, PartState};

fn data(n: usize, seed: u32) -> Vec<u8> {
    (0..n)
        .map(|i| ((i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed) >> 13) as u8)
        .collect()
}

fn files() -> Vec<(String, Vec<u8>)> {
    vec![
        ("a/big.bin".into(), data(5000, 1)),
        ("a/empty".into(), Vec::new()),
        ("b/mid.txt".into(), data(1500, 2)),
        ("b/dir/x.raw".into(), data(3333, 3)),
        ("z".into(), data(7, 4)),
    ]
}

#[derive(Default)]
struct Mem(Vec<Vec<u8>>);
impl PartSink for Mem {
    fn put(&mut self, part: usize, b: &[u8]) -> Result<()> {
        if self.0.len() <= part {
            self.0.resize(part + 1, Vec::new());
        }
        self.0[part].extend_from_slice(b);
        Ok(())
    }
}

struct Parts(Vec<Option<Vec<u8>>>);
impl PartSource for Parts {
    fn size(&mut self, i: usize) -> Result<Option<u64>> {
        Ok(self.0[i].as_ref().map(|p| p.len() as u64))
    }
    fn open(&mut self, i: usize, from: u64) -> Result<Box<dyn Read>> {
        Ok(Box::new(std::io::Cursor::new(
            self.0[i].clone().unwrap()[from as usize..].to_vec(),
        )))
    }
}

#[derive(Default)]
struct Out {
    done: BTreeMap<String, Vec<u8>>,
    cur: Option<(String, Vec<u8>)>,
    existing: Vec<String>,
}
impl FileSink for Out {
    fn exists(&mut self, p: &str) -> Result<bool> {
        Ok(self.existing.iter().any(|e| e == p))
    }
    fn begin(&mut self, p: &str) -> Result<()> {
        self.cur = Some((p.into(), Vec::new()));
        Ok(())
    }
    fn write(&mut self, b: &[u8]) -> Result<()> {
        self.cur.as_mut().unwrap().1.extend_from_slice(b);
        Ok(())
    }
    fn end(&mut self, keep: bool) -> Result<()> {
        let (p, b) = self.cur.take().unwrap();
        if keep {
            self.done.insert(p, b);
        }
        Ok(())
    }
}

fn opts(part: u64, zip64_at: u64) -> Options {
    Options {
        name: "t".into(),
        part_size: part,
        zip64_at,
    }
}

fn make(part: u64, zip64_at: u64) -> (Manifest, Vec<Vec<u8>>) {
    let fs = files();
    let list: Vec<(String, u64)> = fs
        .iter()
        .map(|(n, b)| (n.clone(), b.len() as u64))
        .collect();
    let mut sink = Mem::default();
    let mut open =
        |i: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::Cursor::new(fs[i].1.clone()))) };
    let m = pack::pack(&list, &mut open, &mut sink, &opts(part, zip64_at)).unwrap();
    (m, sink.0)
}

#[test]
fn a_round_trip_over_many_parts_gives_back_every_file_byte_for_byte() {
    for part in [1024, 4096, 100_000] {
        let (m, parts) = make(part, zipstream_at());
        assert_eq!(m.parts.len(), parts.len());
        assert!(parts
            .iter()
            .take(parts.len() - 1)
            .all(|p| p.len() as u64 == part));
        let mut src = Parts(parts.into_iter().map(Some).collect());
        let mut out = Out::default();
        let (ps, fs) =
            pack_read::unpack(&m, &mut src, &mut out, false, true, &mut |_, _| {}).unwrap();
        assert!(
            ps.iter().all(|p| *p == PartState::Ok) && fs.iter().all(|f| *f == FileState::Written)
        );
        for (name, bytes) in files() {
            assert!(out.done[&name] == bytes, "{name} at {part}");
        }
    }
}

fn zipstream_at() -> u64 {
    crate::zipstream::ZIP64_AT
}

#[test]
fn the_same_input_gives_the_same_parts_and_the_plan_names_them_before() {
    let (m1, p1) = make(2048, zipstream_at());
    let (m2, p2) = make(2048, zipstream_at());
    assert!(p1 == p2 && m1 == m2);
    assert_eq!(pack::manifest_json(&m1), pack::manifest_json(&m2));
    let list: Vec<(String, u64)> = files()
        .iter()
        .map(|(n, b)| (n.clone(), b.len() as u64))
        .collect();
    let planned = pack::plan(&list, &opts(2048, zipstream_at()));
    assert_eq!(planned.stream_bytes, m1.stream_bytes);
    assert_eq!(
        planned.parts.iter().map(|p| p.size).collect::<Vec<_>>(),
        m1.parts.iter().map(|p| p.size).collect::<Vec<_>>()
    );
    assert_eq!(
        planned.files.iter().map(|f| f.offset).collect::<Vec<_>>(),
        m1.files.iter().map(|f| f.offset).collect::<Vec<_>>()
    );
}

#[test]
fn the_manifest_reads_back_as_written() {
    let (m, _) = make(2048, zipstream_at());
    assert!(pack::parse_manifest(pack::manifest_json(&m).as_bytes()).unwrap() == m);
}

#[test]
fn a_flipped_byte_loses_the_files_of_that_part_and_the_others_are_written() {
    let (m, mut parts) = make(1024, zipstream_at());
    assert!(parts.len() >= 5);
    parts[1][10] ^= 1;
    let mut src = Parts(parts.into_iter().map(Some).collect());
    let mut out = Out::default();
    let (ps, fs) = pack_read::unpack(&m, &mut src, &mut out, false, true, &mut |_, _| {}).unwrap();
    assert_eq!(ps[1], PartState::Corrupt("sha256"));
    assert!(ps
        .iter()
        .enumerate()
        .all(|(i, p)| i == 1 || *p == PartState::Ok));
    assert_eq!(fs[0], FileState::Lost, "a/big.bin starts in part 1");
    assert!(fs.contains(&FileState::Written));
    assert!(!out.done.contains_key("a/big.bin"));
    for (name, bytes) in files()
        .into_iter()
        .filter(|(n, _)| out.done.contains_key(n))
    {
        assert!(out.done[&name] == bytes);
    }
    assert_eq!(pack_read::damage(&ps, &fs), (true, true));
    let v = pack_read::verify_json(&m, &ps);
    assert!(
        v.contains("\"corrupt\":[\"t.part0002\"]") && v.contains("\"resend\":[\"t.part0002\"]"),
        "{v}"
    );
}

#[test]
fn a_missing_part_and_a_short_part_are_told_apart() {
    let (m, mut parts) = make(1024, zipstream_at());
    let mut opt: Vec<Option<Vec<u8>>> = parts.iter().cloned().map(Some).collect();
    opt[2] = None;
    parts[3].pop();
    opt[3] = Some(parts[3].clone());
    let ps = pack_read::verify_parts(&m, &mut Parts(opt), &mut |_, _| {}).unwrap();
    assert_eq!(
        (ps[2], ps[3]),
        (PartState::Missing, PartState::Corrupt("size"))
    );
    let v = pack_read::verify_json(&m, &ps);
    assert!(
        v.contains("\"missing\":[\"t.part0003\"]") && v.contains("\"corrupt\":[\"t.part0004\"]"),
        "{v}"
    );
}

#[test]
fn an_existing_file_is_never_overwritten_without_force() {
    let (m, parts) = make(4096, zipstream_at());
    let mut src = Parts(parts.into_iter().map(Some).collect());
    let mut out = Out {
        existing: vec!["z".into()],
        ..Out::default()
    };
    let e = pack_read::unpack(&m, &mut src, &mut out, false, true, &mut |_, _| {})
        .err()
        .unwrap();
    assert_eq!(e.key, "err.pack.exists");
    assert!(out.done.is_empty());
    assert!(pack_read::unpack(&m, &mut src, &mut out, true, true, &mut |_, _| {}).is_ok());
    assert_eq!(out.done.len(), 5);
}

#[test]
fn a_dry_run_writes_nothing() {
    let (m, parts) = make(4096, zipstream_at());
    let mut out = Out::default();
    let (_, fs) = pack_read::unpack(
        &m,
        &mut Parts(parts.into_iter().map(Some).collect()),
        &mut out,
        false,
        false,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(out.done.is_empty() && fs.iter().all(|f| *f == FileState::Ready));
}

#[test]
fn a_path_that_leaves_the_folder_is_refused_in_the_manifest_and_when_packing() {
    let (m, _) = make(4096, zipstream_at());
    for evil in [
        "../x",
        "/abs",
        "a/../../x",
        "a\\b",
        "C:/x",
        "a//b",
        "./a",
        "C:x",
        "C:x/y",
        "D:evil.exe",
        "a/b:stream",
    ] {
        let text = pack::manifest_json(&m).replace("\"z\"", &format!("{evil:?}"));
        assert_eq!(
            pack::parse_manifest(text.as_bytes()).err().unwrap().key,
            "err.pack.path",
            "{evil}"
        );
        let mut sink = Mem::default();
        let mut open = |_: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::empty())) };
        let e = pack::pack(
            &[(evil.to_string(), 0)],
            &mut open,
            &mut sink,
            &opts(1024, zipstream_at()),
        )
        .err()
        .unwrap();
        assert_eq!(e.key, "err.pack.path", "{evil}");
    }
    let part = pack::manifest_json(&m).replace("t.part0001", "../t.part0001");
    assert_eq!(
        pack::parse_manifest(part.as_bytes()).err().unwrap().key,
        "err.pack.path"
    );
}

#[test]
fn an_inconsistent_manifest_is_refused() {
    let (m, _) = make(4096, zipstream_at());
    let text = pack::manifest_json(&m);
    let sizes = text.replace(
        &format!("\"stream_bytes\":{}", m.stream_bytes),
        "\"stream_bytes\":5",
    );
    for t in [sizes, text.replace("pack/1", "pack/2"), "{".into()] {
        assert_eq!(
            pack::parse_manifest(t.as_bytes()).err().unwrap().key,
            "err.pack.manifest"
        );
    }
}

#[test]
fn a_file_that_changes_size_while_it_is_read_stops_the_pack() {
    for (listed, real) in [(100u64, 120usize), (100, 80)] {
        let mut sink = Mem::default();
        let mut open = |_: usize| -> Result<Box<dyn Read>> {
            Ok(Box::new(std::io::Cursor::new(vec![7u8; real])))
        };
        let e = pack::pack(
            &[("f".into(), listed)],
            &mut open,
            &mut sink,
            &opts(1024, zipstream_at()),
        )
        .err()
        .unwrap();
        assert_eq!(e.key, "err.pack.growing");
    }
}

#[test]
fn an_entry_over_the_zip64_limit_round_trips_too() {
    let (m, parts) = make(2048, 1000);
    let mut out = Out::default();
    pack_read::unpack(
        &m,
        &mut Parts(parts.into_iter().map(Some).collect()),
        &mut out,
        false,
        true,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(out.done.len(), 5);
}

#[test]
fn paths_that_are_one_file_on_a_case_insensitive_disk_are_refused() {
    let (m, _) = make(4096, zipstream_at());
    let mut sink = Mem::default();
    let mut open = |_: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::empty())) };
    let two = [("Sample.raw".to_string(), 0), ("sample.raw".to_string(), 0)];
    let e = pack::pack(&two, &mut open, &mut sink, &opts(1024, zipstream_at()))
        .err()
        .unwrap();
    assert_eq!(e.key, "err.pack.duplicate");
    let text = pack::manifest_json(&m).replace("\"z\"", "\"A/BIG.bin\"");
    assert_eq!(
        pack::parse_manifest(text.as_bytes()).err().unwrap().key,
        "err.pack.duplicate"
    );
}

#[test]
fn a_part_size_of_zero_is_refused_instead_of_looping() {
    let mut sink = Mem::default();
    let mut open = |_: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::empty())) };
    let e = pack::pack(
        &[("f".into(), 0)],
        &mut open,
        &mut sink,
        &opts(0, zipstream_at()),
    )
    .err()
    .unwrap();
    assert_eq!(e.key, "err.run.input");
}

/// Parts that are the right size but cannot be opened for reading once `shut` is set.
struct Locked(Parts, std::rc::Rc<std::cell::Cell<bool>>);
impl PartSource for Locked {
    fn size(&mut self, i: usize) -> Result<Option<u64>> {
        self.0.size(i)
    }
    fn open(&mut self, i: usize, from: u64) -> Result<Box<dyn Read>> {
        if i == 0 && self.1.get() {
            return Err(Error::new("err.run.input").with("detail", "locked"));
        }
        self.0.open(i, from)
    }
}

#[test]
fn a_part_that_cannot_be_read_is_an_input_error_not_a_mismatch() {
    let (m, parts) = make(4096, zipstream_at());
    let shut = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut src = Locked(Parts(parts.into_iter().map(Some).collect()), shut.clone());
    let mut out = Out::default();
    // every part is verified first; the lock closes once part 0 has been, before the files are copied
    let r = pack_read::unpack(&m, &mut src, &mut out, false, true, &mut |i, _| {
        if i == 0 {
            shut.set(true);
        }
    });
    assert_eq!(r.err().unwrap().key, "err.run.input");
}

#[test]
fn a_dry_run_plan_is_refused_for_the_names_a_real_run_refuses() {
    let o = opts(1024, zipstream_at());
    let bad: [&[&str]; 6] = [
        &["A.txt", "a.txt"],
        &["same", "same"],
        &["c:x.txt"],
        &["a", "a/b"],
        &["A", "a/b"],
        &["x/../y"],
    ];
    for names in bad {
        let list: Vec<(String, u64)> = names.iter().map(|n| (n.to_string(), 0)).collect();
        let mut open = |_: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::empty())) };
        let real = pack::pack(&list, &mut open, &mut Mem::default(), &o)
            .err()
            .unwrap();
        let plan = pack::plan_checked(&list, &o).err().unwrap();
        assert_eq!(
            (&plan.key, &plan.params),
            (&real.key, &real.params),
            "{names:?}"
        );
    }
    assert!(pack::plan_checked(&[("a/b".into(), 1), ("a/c".into(), 1)], &o).is_ok());
}

/// A disk that folds `e` + a combining accent onto the one-character form, as macOS does.
struct Folding(Out);
impl Folding {
    fn fold(p: &str) -> String {
        p.replace("e\u{301}", "\u{e9}")
    }
}
impl FileSink for Folding {
    fn exists(&mut self, p: &str) -> Result<bool> {
        let want = Self::fold(p);
        Ok(self.0.done.keys().any(|k| Self::fold(k) == want))
    }
    fn begin(&mut self, p: &str) -> Result<()> {
        self.0.begin(p)
    }
    fn write(&mut self, b: &[u8]) -> Result<()> {
        self.0.write(b)
    }
    fn end(&mut self, keep: bool) -> Result<()> {
        self.0.end(keep)
    }
}

#[test]
fn unpack_stops_at_a_name_the_disk_folds_onto_a_file_it_just_wrote() {
    let list = [("caf\u{e9}".to_string(), 3), ("cafe\u{301}".to_string(), 3)];
    let mut parts = Mem::default();
    let mut open =
        |_: usize| -> Result<Box<dyn Read>> { Ok(Box::new(std::io::Cursor::new(vec![7u8; 3]))) };
    let m = pack::pack(&list, &mut open, &mut parts, &opts(1024, zipstream_at())).unwrap();
    let mut sink = Folding(Out::default());
    let e = pack_read::unpack(
        &m,
        &mut Parts(parts.0.into_iter().map(Some).collect()),
        &mut sink,
        false,
        true,
        &mut |_, _| {},
    )
    .err()
    .unwrap();
    assert_eq!(e.key, "err.pack.exists");
    assert_eq!(
        sink.0.done.len(),
        1,
        "the first file stays, the second is not written"
    );
}
