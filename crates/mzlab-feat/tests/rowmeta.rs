//! The summaries of the rows (`rowmeta::compute`): medians over the files that detected the row, the best MS2, the charge, the M+1/M of a family, and
//! the same summaries from the stored files (in slabs) as from the files in memory; the table with them survives a write and a read.

use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::Index;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::series::align_and_fill;
use mzlab_feat::slabs::align_and_fill_stored;
use mzlab_feat::store::{read_table, write_file, write_table, Origin};
use mzlab_feat::table::{FileFeatures, FLAG_GOOD, NO_FILE, ROLE_ISOTOPE, ROLE_M};

const FILES: usize = 3;

/// A file with an ion of m/z 200.1 (RT 5, family 1) and its M+1 at 201.1034 (area 1/10), an ion at 300.2 only in files 0 and 2, and one at 400.3 with MS2 in file 1.
fn file(k: usize) -> FileFeatures {
    let mut f = FileFeatures {
        dt: 1.0,
        ..FileFeatures::default()
    };
    let mut push = |mz: f64,
                    rt: f32,
                    left: f32,
                    right: f32,
                    area: f64,
                    shape: f32,
                    snr: f32,
                    fwhm: f32,
                    fam: u32,
                    role: u8,
                    ms2: (u16, f32),
                    q: u8| {
        f.mz.push(mz);
        f.rt.push(rt);
        f.left.push(left);
        f.right.push(right);
        f.apex.push(0);
        f.scan_lo.push(0);
        f.scan_hi.push(0);
        f.area.push(area);
        f.height.push(1e6);
        f.npts.push(10);
        f.flags.push(FLAG_GOOD);
        f.iso_group.push(0);
        f.charge.push(1);
        f.shape.push(shape);
        f.snr.push(snr);
        f.fwhm.push(fwhm);
        f.asym.push(1.0);
        f.gaps.push(0);
        f.qreason.push(q);
        f.ms2_n.push(ms2.0);
        f.purity.push(ms2.1);
        f.area_win.push(area);
        f.family_id.push(fam);
        f.role.push(role);
        f.alt.push(0);
    };
    let d = k as f32 * 0.01;
    push(
        200.1,
        5.0 + d,
        4.9 + d,
        5.1 + d,
        1e7,
        0.9 + d,
        100.0 + k as f32,
        10.0 + k as f32,
        1,
        ROLE_M,
        (0, -1.0),
        0,
    );
    push(
        201.103355,
        5.0 + d,
        4.9 + d,
        5.1 + d,
        1e6,
        0.9,
        10.0,
        10.0,
        1,
        ROLE_ISOTOPE,
        (0, -1.0),
        0,
    );
    if k != 1 {
        push(
            300.2,
            7.0,
            6.9 - d,
            7.1 + 2.0 * d,
            5e6,
            0.8,
            50.0,
            8.0,
            0,
            ROLE_M,
            (0, -1.0),
            1 << (k / 2),
        );
    }
    push(
        400.3,
        9.0,
        8.9,
        9.1,
        4e6,
        0.7,
        40.0,
        9.0,
        0,
        ROLE_M,
        if k == 1 {
            (3, 0.8)
        } else {
            (2, 0.3 + k as f32 * 0.1)
        },
        0,
    );
    f
}

fn recipe() -> Recipe {
    Recipe {
        mode: Mode::Ion,
        weak_files: 0,
        fill_gaps: false,
        ..Recipe::default()
    }
}

fn open(_: usize) -> Result<Index<Vec<u8>>> {
    unreachable!("no weak tier, no filling")
}

fn row(t: &mzlab_feat::table::Aligned, mz: f64) -> usize {
    (0..t.len()).find(|&r| (t.mz[r] - mz).abs() < 1e-3).unwrap()
}

#[test]
fn medians_charge_isotope_and_ms2_come_from_the_files_that_detected_the_row() {
    let files: Vec<FileFeatures> = (0..FILES).map(file).collect();
    let (t, _) = align_and_fill(&files, &recipe(), open).unwrap();
    assert!(t.is_id_ordered() && t.meta.len() == t.len());
    let m = &t.meta;
    let a = row(&t, 200.1);
    assert!(
        (m.rt_lo[a] - 4.91).abs() < 1e-5 && (m.rt_hi[a] - 5.11).abs() < 1e-5,
        "{} {}",
        m.rt_lo[a],
        m.rt_hi[a]
    );
    assert!(
        (m.q_shape[a] - 0.91).abs() < 1e-5
            && (m.q_snr[a] - 101.0).abs() < 1e-4
            && (m.q_fwhm[a] - 11.0).abs() < 1e-4
    );
    assert_eq!((m.charge[a], m.qreason[a], m.ms2_file[a]), (1, 0, NO_FILE));
    let b = row(&t, 300.2);
    // present in files 0 and 2: the median of two is their mean, the quality bits are the OR
    assert!(
        (m.rt_lo[b] - (6.9 + 6.88) / 2.0).abs() < 1e-5,
        "{}",
        m.rt_lo[b]
    );
    assert_eq!(m.qreason[b], 1 | (1 << 1));
    let c = row(&t, 400.3);
    assert_eq!((m.ms2_file[c], m.ms2_purity[c]), (1, 0.8));
}

#[test]
fn the_row_with_no_detected_cell_has_no_summary() {
    let files: Vec<FileFeatures> = (0..FILES).map(file).collect();
    let (mut t, _) = align_and_fill(&files, &recipe(), open).unwrap();
    let r = row(&t, 300.2);
    for k in 0..FILES {
        let c = t.cell(r, k);
        t.state[c] = mzlab_feat::table::CELL_ABSENT;
    }
    mzlab_feat::rowmeta::compute(&mut t, &mut |k, _| Ok(files[k].clone())).unwrap();
    assert!(t.meta.rt_lo[r].is_nan() && t.meta.charge[r] == 0 && t.meta.ms2_file[r] == NO_FILE);
}

#[test]
fn stored_files_give_the_summaries_of_the_files_in_memory() {
    let files: Vec<FileFeatures> = (0..FILES).map(file).collect();
    let (want, _) = align_and_fill(&files, &recipe(), open).unwrap();
    let r = recipe();
    let mut stored: Vec<Vec<u8>> = files
        .iter()
        .map(|f| write_file(f, &r, &Origin::default()))
        .collect();
    let (got, _) = align_and_fill_stored(&mut stored, &MemBudget::mb(1024.0), &r, open).unwrap();
    assert_eq!(got.to_bytes(), want.to_bytes());
    let bytes = write_table(&got, &r, &Origin::default());
    let (back, meta) = read_table(&bytes).unwrap();
    assert_eq!(back.meta.len(), got.meta.len());
    assert_eq!(back.to_bytes(), got.to_bytes());
    assert_eq!(meta.get("version").and_then(|v| v.f64()), Some(2.0));
    assert!(back.is_id_ordered());
}
