//! The files `.mzfeatf` and `.mzfeat` and the recipe: what is written is read back identical, two writes give the same bytes, arrays start at
//! multiples of 64 bytes, the fingerprint of the files does not change when only the alignment changes, a damaged file is refused.

use mzlab_feat::famalign::align_families;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::store::{read_file, read_table, write_file, write_table, Origin};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_DETECTED, CELL_FILLED, FLAG_GOOD, ROLE_M};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn features(n: usize, seed: u64) -> FileFeatures {
    let mut r = Lcg(seed);
    let mut f = FileFeatures {
        dt: 1.5 + r.next(),
        ..FileFeatures::default()
    };
    for i in 0..n {
        let rt = (1.0 + r.next() * 20.0) as f32;
        f.mz.push(100.0 + i as f64 * 0.37 + r.next() * 1e-4);
        f.rt.push(rt);
        f.left.push(rt - 0.05);
        f.right.push(rt + 0.05);
        f.apex.push(i as u32 * 3);
        f.scan_lo.push(i as u32 * 3);
        f.scan_hi.push(i as u32 * 3 + 1);
        f.area.push(1e6 * (0.5 + r.next()));
        f.height.push(1e5 * (0.5 + r.next()) as f32);
        f.npts.push(5 + (i % 9) as u16);
        f.flags.push(FLAG_GOOD);
        f.iso_group.push(0);
        f.charge.push((i % 3) as i8 - 1);
        f.shape.push(r.next() as f32);
        f.snr.push(3.0 + r.next() as f32);
        f.fwhm.push(r.next() as f32);
        f.asym.push(0.0);
        f.gaps.push(0);
        f.qreason.push(0);
        f.ms2_n.push((i % 4) as u16);
        f.purity.push(-1.0);
        f.area_win.push(9e5);
        f.family_id.push(i as u32 + 1);
        f.role.push(ROLE_M);
        f.alt.push(0);
    }
    f
}

fn origin() -> Origin {
    Origin {
        inputs: vec![
            ("a.mzML".into(), "ab".repeat(32)),
            ("b.mzML".into(), "cd".repeat(32)),
        ],
        ..Origin::default()
    }
}

fn series() -> (Vec<FileFeatures>, Aligned) {
    let files = vec![features(300, 1), features(300, 2)];
    let refs: Vec<&FileFeatures> = files.iter().collect();
    (
        files.clone(),
        align_families(&refs, &Recipe::default().align).unwrap(),
    )
}

#[test]
fn file_round_trip_and_alignment_of_arrays() {
    let f = features(257, 7);
    let r = Recipe::default();
    let bytes = write_file(&f, &r, &origin());
    assert_eq!(bytes, write_file(&f, &r, &origin()));
    let (back, meta) = read_file(&bytes).unwrap();
    assert_eq!(back, f);
    assert_eq!(
        meta.get("recipe_hash").and_then(|v| v.str()).unwrap().len(),
        32
    );
    assert_eq!(meta.get("inputs").unwrap().arr().len(), 2);
    let data =
        (16 + u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize).div_ceil(64) * 64;
    assert!(data <= bytes.len());
    for a in meta.get("arrays").unwrap().arr() {
        assert_eq!(
            a.get("offset").and_then(|v| v.f64()).unwrap() as usize % 64,
            0
        );
    }
    let names: Vec<&str> = meta
        .get("arrays")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|a| a.get("name")?.str())
        .collect();
    assert!(names.contains(&"evid"));
    let evid = meta
        .get("arrays")
        .unwrap()
        .arr()
        .iter()
        .find(|a| a.get("name").and_then(|v| v.str()) == Some("evid"))
        .unwrap();
    assert_eq!(evid.get("count").and_then(|v| v.f64()), Some(0.0));
}

#[test]
fn table_round_trip_and_states() {
    let (_, mut t) = series();
    // a filled cell is never a detected one, and survives the file
    let k = (0..t.state.len())
        .find(|&i| t.state[i] == CELL_DETECTED)
        .unwrap();
    t.state[k] = CELL_FILLED;
    t.apex_rt[k] = f32::NAN;
    let detected = t.state.iter().filter(|&&s| s == CELL_DETECTED).count();
    let r = Recipe::default();
    let bytes = write_table(&t, &r, &origin());
    assert_eq!(bytes, write_table(&t, &r, &origin()));
    let (back, meta) = read_table(&bytes).unwrap();
    assert_eq!(back.to_bytes(), t.to_bytes());
    assert_eq!(back.n_files, 2);
    assert_eq!(
        back.state.iter().filter(|&&s| s == CELL_DETECTED).count(),
        detected
    );
    assert_eq!(back.state[k], CELL_FILLED);
    assert_eq!(meta.get("rows").and_then(|v| v.f64()), Some(t.len() as f64));
}

#[test]
fn damaged_files_are_refused() {
    let (_, t) = series();
    let bytes = write_table(&t, &Recipe::default(), &origin());
    assert!(read_table(&bytes[..10]).is_err());
    assert!(read_table(&bytes[..bytes.len() - 100]).is_err());
    let mut wrong = bytes.clone();
    wrong[0] = b'X';
    assert!(read_table(&wrong).is_err());
    assert!(
        read_file(&bytes).is_err(),
        "a table is not a file of features"
    );
}

#[test]
fn fingerprints_follow_the_parameters() {
    let base = Recipe::default();
    let mut other = base.clone();
    other.align.link_rt = 0.3;
    other.fill.floor = 1e4;
    other.mode = Mode::Ion;
    assert_eq!(
        base.file_hash(),
        other.file_hash(),
        "the alignment does not change the files"
    );
    assert_ne!(base.table_hash(), other.table_hash());
    let mut detect = base.clone();
    detect.detect.floor = 1e4;
    assert_ne!(base.file_hash(), detect.file_hash());
    let mut slabs = base.clone();
    slabs.detect.budget = 1;
    assert_eq!(
        base.file_hash(),
        slabs.file_hash(),
        "the size of the slabs does not change the result"
    );
    assert_eq!(base.table_hash(), Recipe::default().table_hash());
}

#[test]
fn ids_are_positions_and_counts_come_from_the_states() {
    let (_, mut t) = series();
    assert!(!t.is_empty());
    assert_eq!((t.id(0), t.id(t.len() - 1)), (1, t.len() as u32));
    assert_eq!(t.row_of_id(0), None);
    assert_eq!(t.row_of_id(t.len() as u32 + 1), None);
    assert!((0..t.len()).all(|r| t.row_of_id(t.id(r)) == Some(r)));
    let k = (0..t.state.len())
        .find(|&i| t.state[i] == CELL_DETECTED)
        .unwrap();
    let before = t.state_counts(k / t.n_files);
    t.state[k] = CELL_FILLED;
    let after = t.state_counts(k / t.n_files);
    assert_eq!((after.0 + 1, after.2), (before.0, before.2 + 1));
    let total: usize = (0..t.len())
        .map(|r| t.state_counts(r))
        .map(|c| (c.0 + c.1 + c.2) as usize)
        .sum();
    assert!(total <= t.state.len());
}

#[test]
fn sorting_the_rows_keeps_every_row_whole() {
    let (_, t) = series();
    assert!(
        !t.is_id_ordered(),
        "the test series must start out of order"
    );
    let mut s = t.clone();
    s.sort_rows();
    assert!(s.is_id_ordered());
    assert_eq!(
        (s.len(), s.state.len(), s.member_mz.len()),
        (t.len(), t.state.len(), t.member_mz.len())
    );
    // every row of the sorted table is a row of the original: its reference ion, its cells and its members
    let key = |a: &Aligned, r: usize| {
        let ms: Vec<(u64, u32)> = a
            .members(r)
            .map(|m| (a.member_mz[m].to_bits(), a.member_rt[m].to_bits()))
            .collect();
        let cells: Vec<(u64, u8)> = (0..a.n_files)
            .map(|k| (a.area[a.cell(r, k)].to_bits(), a.state[a.cell(r, k)]))
            .collect();
        (a.mz[r].to_bits(), a.rt[r].to_bits(), cells, ms)
    };
    let mut before: Vec<_> = (0..t.len()).map(|r| key(&t, r)).collect();
    let mut after: Vec<_> = (0..s.len()).map(|r| key(&s, r)).collect();
    before.sort();
    after.sort();
    assert_eq!(before, after);
    let again = s.clone();
    s.sort_rows();
    assert_eq!(s.to_bytes(), again.to_bytes());
}
