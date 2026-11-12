//! The closing tag at the end of an mzML tells a whole file from one cut short (whose TIC is rebuilt from the index).
use super::*;

#[test]
fn a_file_is_closed_when_it_ends_with_its_closing_tag() {
    let check = |s: &str| {
        let mut b = s.as_bytes().to_vec();
        let n = b.len() as u64;
        closed(&mut b, n)
    };
    assert!(check("<indexedmzML><mzML></mzML></indexedmzML>"));
    assert!(check(&format!("{}<mzML></mzML>", " ".repeat(200))));
    assert!(!check("<indexedmzML><mzML><run>"));
    assert!(!check(""));
}
