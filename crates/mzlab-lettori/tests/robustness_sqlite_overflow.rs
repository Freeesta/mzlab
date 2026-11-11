//! An overflow chain of the Bruker SQLite reader that points back at a page, or a cell that claims a payload larger than the file: `err.file.brukerDamaged`,
//! never a payload copied again and again from the same page (a wrong value read with no error, or a buffer that grows to the claimed size).
#![cfg(feature = "bruker")]

mod common_sqlite;

use common_sqlite::*;

/// Table `T` at page 2: a leaf with one cell of `tot` bytes whose first overflow page is 3, which links to `link`; `extra` pages of `a` after it.
fn looped(tot: u64, link: u32, extra: usize) -> Vec<u8> {
    let (u, x) = (PAG, PAG - 35);
    let m = (u - 12) * 32 / 255 - 23;
    let k = m + (tot as usize - m) % (u - 4);
    let local = if k <= x { k } else { m };
    // a record of one text value filling the payload
    let mut kind = varint(13 + 2 * (tot - 6));
    let mut rec = vec![(kind.len() + 1) as u8];
    rec.append(&mut kind);
    rec.resize(local, b'a');
    let mut cell = varint(tot);
    cell.extend(varint(1));
    cell.extend(&rec);
    cell.extend(3u32.to_be_bytes());
    let mut pages = page(13, 0, &[cell], 0);
    let mut ov = vec![b'a'; PAG];
    ov[..4].copy_from_slice(&link.to_be_bytes());
    pages.extend(ov);
    pages.extend(vec![b'a'; PAG * extra]);
    db(&[("T", "CREATE TABLE T (Testo TEXT)", pages)])
}

fn read(b: Vec<u8>) -> Result<usize, String> {
    let mut s = open_db(b, None).map_err(|e| e.key.to_string())?;
    let t = s.table("T").expect("table T").clone();
    let mut n = 0;
    let r = s.rows(&t, |_| {
        n += 1;
        Ok(true)
    });
    r.map(|_| n).map_err(|e| e.key.to_string())
}

#[test]
fn an_overflow_page_that_links_to_itself_is_damaged() {
    // the file holds the payload, but only by reading page 3 again and again
    let tot = 4 * (PAG as u64 - 4);
    assert_eq!(
        read(looped(tot, 3, 4)),
        Err("err.file.brukerDamaged".into())
    );
}

#[test]
fn a_payload_larger_than_the_file_is_damaged_before_any_copy() {
    let t = std::time::Instant::now();
    assert_eq!(
        read(looped(100 << 20, 3, 0)),
        Err("err.file.brukerDamaged".into())
    );
    assert!(t.elapsed().as_secs() < 2);
}

#[test]
fn a_chain_of_distinct_pages_still_reads() {
    // page 3 ends the chain (link 0) and holds the rest of a payload that fits it
    let tot = PAG as u64;
    assert_eq!(read(looped(tot, 0, 0)), Ok(1));
}
