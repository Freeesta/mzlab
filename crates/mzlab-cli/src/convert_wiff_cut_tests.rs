//! `mzlab-cli file convert` on a large synthetic `.wiff` container (more FAT sectors than the header lists, so a DIFAT sector) cut inside the FAT sectors the DIFAT
//! lists: the same key as the engine gives (`wiffTruncated`, shared by the site and the app), never «not a .wiff». Synthetic bytes, no lab data.
use super::super::convert;
use crate::convert_fixtures::folder;
use crate::flow_run_tests::p;

const NONE: u32 = 0xFFFF_FFFF;
const END: u32 = 0xFFFF_FFFE;

/// 130 FAT sectors: the DIFAT sector, the directory sector, then the FAT sectors (the last 21 listed only by the DIFAT).
fn large() -> Vec<u8> {
    let (n_fat, n_dif, ss) = (130usize, 1usize, 512usize);
    let (dir, fat0) = (n_dif, n_dif + 1);
    let mut h = vec![0u8; 512];
    h[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    for (at, v) in [
        (0x2C, n_fat as u32),
        (0x30, dir as u32),
        (0x38, 4096),
        (0x3C, END),
        (0x44, 0),
        (0x48, 1),
    ] {
        h[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    h[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
    h[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    for i in 0..109 {
        h[0x4C + 4 * i..0x50 + 4 * i].copy_from_slice(&((fat0 + i) as u32).to_le_bytes());
    }
    let mut out = h;
    for k in 0..127 {
        let j = 109 + k;
        out.extend((if j < n_fat { (fat0 + j) as u32 } else { NONE }).to_le_bytes());
    }
    out.extend(END.to_le_bytes());
    let mut root = vec![0u8; ss];
    root[0x42] = 5;
    root[0x44..0x50].fill(0xFF);
    root[0x74..0x78].copy_from_slice(&END.to_le_bytes());
    out.extend(root);
    let mut fat = vec![NONE; n_fat * ss / 4];
    fat[..n_dif].fill(0xFFFF_FFFC);
    fat[dir] = END;
    fat[fat0..fat0 + n_fat].fill(0xFFFF_FFFD);
    out.extend(fat.iter().flat_map(|v| v.to_le_bytes()));
    out
}

#[test]
fn a_large_wiff_cut_inside_its_difat_listed_fat_gives_the_engine_key() {
    let dir = folder("wiff_cut_difat");
    std::fs::create_dir_all(&dir).unwrap();
    let whole = large();
    // the engine (shared with the site and the app) on the same bytes
    let engine = |b: &[u8]| {
        mzlab_core::wiff::container(b.to_vec(), "x.wiff")
            .err()
            .map(|e| e.key)
    };
    assert_eq!(engine(&whole), None);
    for cut in [whole.len() - 1, whole.len() - 600, 512 * 123 + 7] {
        let cut_bytes = &whole[..cut];
        assert_eq!(
            engine(cut_bytes),
            Some("err.file.wiffTruncated"),
            "cut {cut}"
        );
        std::fs::write(dir.join("x.wiff"), cut_bytes).unwrap();
        std::fs::write(dir.join("x.wiff.scan"), [0u8; 16]).unwrap();
        let o = convert(&[p(&dir.join("x.wiff")), "--out", p(&dir.join("out"))]);
        assert_ne!(o.code, 0, "{}", o.stderr);
        assert!(
            o.stderr.contains("err.file.wiffTruncated"),
            "cut {cut}: {}",
            o.stderr
        );
        assert!(
            !o.stderr.contains("err.file.notWiff"),
            "cut {cut}: {}",
            o.stderr
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
