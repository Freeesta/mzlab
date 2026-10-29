//! Tests of the sibling module (loaded with `#[path]`, so they see its private items).

#![allow(clippy::unwrap_used)]
use super::*;
fn linear(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    let tail = h.get(from..).filter(|_| !n.is_empty())?;
    tail.windows(n.len()).position(|w| w == n).map(|p| p + from)
}

#[test]
fn find_equals_window_scan() {
    let h = b"<a><b></b><run></run><run>x</run>";
    for n in [
        &b"</run>"[..],
        b"<",
        b"",
        b"x</run>",
        b"<run>x",
        b"zz",
        h,
        b"<a><b></b><run></run><run>x</run>>",
    ] {
        for from in 0..h.len() + 3 {
            assert_eq!(find(h, n, from), linear(h, n, from), "{n:?} {from}");
        }
    }
}

#[test]
fn accession_needle_matches_format() {
    let head = r#"<cvParam accession="MS:1000521" x="MS:1000522"/> accession="MS:10005230""#;
    for x in [
        "MS:1000521",
        "MS:1000522",
        "MS:1000523",
        "MS:100052",
        "MS:10005230",
    ] {
        assert_eq!(
            has_accession(head, x),
            head.contains(&format!("accession=\"{x}\"")),
            "{x}"
        );
    }
}

fn old_polarity(all: &[u8]) -> &'static str {
    let end = find(all, b"</run>", 0).unwrap_or_else(|| all.len().saturating_sub(1));
    let h = part(all, 0, end);
    match (
        find(h, b"accession=\"MS:1000130\"", 0).is_some(),
        find(h, b"accession=\"MS:1000129\"", 0).is_some(),
    ) {
        (true, false) => "positive",
        (false, true) => "negative",
        (true, true) => "mixed",
        _ => "unknown",
    }
}

#[test]
fn streamed_polarity_equals_whole_read() {
    const POS: &[u8] = b"accession=\"MS:1000130\"";
    const NEG: &[u8] = b"accession=\"MS:1000129\"";
    let block = 4usize << 20;
    let mut cases: Vec<Vec<u8>> = vec![];
    // markers placed around the block edge, after and before `</run>`, at the very end, and absent
    for pad in [
        0,
        100,
        block - 30,
        block - 22,
        block - 21,
        block - 1,
        block,
        block + 5,
        2 * block - 10,
    ] {
        for (a, b, run) in [
            (Some(POS), None, true),
            (Some(NEG), Some(POS), true),
            (None, None, true),
            (Some(POS), None, false),
            (Some(NEG), None, false),
        ] {
            let mut v = vec![b' '; pad];
            if let Some(a) = a {
                v.extend_from_slice(a);
            }
            v.extend_from_slice(&[b' '; 7]);
            if let Some(b) = b {
                v.extend_from_slice(b);
            }
            if run {
                v.extend_from_slice(b"</run>");
                v.extend_from_slice(POS);
            }
            cases.push(v);
        }
    }
    // a marker ending exactly on the last byte, and one straddling `</run>`
    let mut v = vec![b' '; 50];
    v.extend_from_slice(POS);
    cases.push(v);
    let mut v = POS.to_vec();
    v.extend_from_slice(b"</ru");
    cases.push(v);
    for c in cases {
        let size = c.len() as u64;
        let mut m = Mzml::new(c.clone(), size);
        assert_eq!(
            m.header_polarity().unwrap(),
            old_polarity(&c),
            "len {}",
            c.len()
        );
    }
}

#[test]
fn times_only_matches_the_full_read() {
    let doc = br#"<chromatogram index="0" id="SRM SIC 1 -> 2" defaultArrayLength="3"><binaryDataArrayList><binaryDataArray><cvParam accession="MS:1000523"/><cvParam accession="MS:1000595"/><cvParam unitName="second"/><binary>AAAAAAAAAAAAAAAAAADwPwAAAAAAAABA</binary></binaryDataArray><binaryDataArray><cvParam accession="MS:1000523"/><cvParam accession="MS:1000515"/><binary>AAAAAAAAAAAAAAAAAADwPwAAAAAAAABA</binary></binaryDataArray></binaryDataArrayList></chromatogram>"#;
    let mut m = Mzml::new(doc.to_vec(), doc.len() as u64);
    let c = m.chromatograms(0).unwrap().remove(0);
    let (t, _) = m.chromatogram(&c).unwrap();
    let got = m.chromatogram_times(&c).unwrap().unwrap();
    assert_eq!(got, t);
    assert_eq!(got.len(), 3);
}
