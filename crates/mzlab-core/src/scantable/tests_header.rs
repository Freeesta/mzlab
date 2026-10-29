//! `read_head` against the previous header code (three source reads) under a counting source and the window cache.

use super::rows::Head;
use super::*;
use crate::window::Windowed;

struct Counting {
    data: Vec<u8>,
    calls: usize,
    bytes: u64,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.calls += 1;
        self.bytes += buf.len() as u64;
        self.data.read_at(off, buf)
    }
}

/// The header code before `read_head`: the first read is only searched, then the head and the run text are read again.
fn old_head<R: ReadAt>(src: &mut Source<'_, R>) -> Result<Head> {
    let size = src.size;
    let head_end = {
        let first = src.read(0, size.min(4 << 20))?;
        find(&first, b"<run ").map(|p| p as u64)
    };
    let head_bytes = src.read(0, head_end.unwrap_or(200000.min(size)))?;
    let run_tag_lo = head_end.unwrap_or(0);
    let run_text = src.read(run_tag_lo, (run_tag_lo + 2000).min(size))?;
    Ok(Head::parse(
        &String::from_utf8_lossy(&head_bytes),
        &String::from_utf8_lossy(&run_text),
    ))
}

fn header(pad: usize, tail: usize, with_run: bool) -> Vec<u8> {
    let mut s = String::from("<mzML><instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\"><cvParam accession=\"MS:1000768\" name=\"x\"/><cvParam accession=\"MS:1000084\" name=\"TQMS\"/></instrumentConfiguration></instrumentConfigurationList>");
    s.push_str("<!--");
    s.push_str(&"x".repeat(pad));
    s.push_str("-->");
    if with_run {
        s.push_str("<run id=\"r\" defaultInstrumentConfigurationRef=\"IC1\" startTimeStamp=\"2020-01-01T00:00:00Z\">");
    }
    s.push_str(&"y".repeat(tail));
    s.into_bytes()
}

/// (calls, bytes) the source sees for `f` under the window cache.
fn measure<F>(data: &[u8], f: F) -> (Head, usize, u64)
where
    F: FnOnce(&mut Source<'_, Windowed<'_, Counting>>) -> Result<Head>,
{
    let size = data.len() as u64;
    let mut c = Counting {
        data: data.to_vec(),
        calls: 0,
        bytes: 0,
    };
    let h = {
        let mut w = Windowed::new(&mut c, size);
        let mut src = Source { r: &mut w, size };
        f(&mut src).unwrap()
    };
    (h, c.calls, c.bytes)
}

#[test]
fn head_is_unchanged_and_the_first_read_is_reused() {
    // `<run ` near the start, after 4 MB (outside the first read), and absent
    let near = header(5_000, 3_000_000, true);
    let far = header(4_300_000, 1_000_000, true);
    let none = header(300_000, 3_000_000, false);
    let tiny = header(10, 5000, false);
    for data in [&near, &far, &none, &tiny] {
        let (old, _, _) = measure(data, |s| old_head(s));
        let (new, _, _) = measure(data, |s| read_head(s));
        assert_eq!(old, new);
    }
    let (_, oc, ob) = measure(&near, |s| old_head(s));
    let (_, nc, nb) = measure(&near, |s| read_head(s));
    eprintln!("near: {oc} reads {ob} B -> {nc} reads {nb} B");
    assert_eq!(oc - nc, 1, "one source read less");
    assert!(ob - nb >= 1 << 20, "about a window less: {ob} -> {nb}");
    // no `<run ` and a 200 000 B head that fits in the first read: one read
    let (_, nc, _) = measure(&none, |s| read_head(s));
    assert_eq!(nc, 1);
    // `<run ` beyond the first read: the old reads are kept
    let (_, oc, _) = measure(&far, |s| old_head(s));
    let (_, nc, _) = measure(&far, |s| read_head(s));
    assert!(nc <= oc);
}
