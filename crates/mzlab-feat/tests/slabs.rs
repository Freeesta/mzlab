//! The alignment of a stored series gives the same `.mzfeat` bytes in memory, in slabs of m/z and with a lean read of the files, and refuses in order when
//! nothing fits. The files are synthetic tables (clusters of features a few ppm apart, jittered in RT) written as `.mzfeatf`.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::idxread::Index;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::slabs::{align_and_fill_stored, Stored};
use mzlab_feat::store::{
    read_file, read_file_from, write_file, write_table_to, Origin, Slice, Want,
};
use mzlab_feat::table::FileFeatures;

const FILES: usize = 5;
const CLUSTERS: usize = 400;
/// Budget (MB) of the long series: above it the families fit in memory, below the slabs are needed.
const LONG_MB: f64 = 4.0;

/// Deterministic noise in [0, 1).
fn unit(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as f64 / (1u64 << 31) as f64
}

fn file(k: usize) -> FileFeatures {
    file_of(k, CLUSTERS, true)
}

/// `sparse`: some clusters are missing and some have two features; otherwise every cluster is in every file with one feature (the series that align well).
fn file_of(k: usize, clusters: usize, sparse: bool) -> FileFeatures {
    let mut s = 7 + k as u64;
    let mut f = FileFeatures {
        dt: 1.5,
        ..Default::default()
    };
    for c in 0..clusters {
        if sparse && unit(&mut s) < 0.15 {
            continue;
        }
        let mz = 150.0 + c as f64 * 0.37 + (c % 7) as f64 * 0.001;
        let rt = 1.0 + (c % 23) as f64 * 0.4;
        let n = 1 + (sparse && unit(&mut s) < 0.2) as usize;
        for j in 0..n {
            f.mz.push(mz * (1.0 + (unit(&mut s) - 0.5) * 4e-6 + j as f64 * 3e-6));
            f.rt.push((rt + (unit(&mut s) - 0.5) * 0.04 + j as f64 * 0.3) as f32);
            f.left.push((rt - 0.05) as f32);
            f.right.push((rt + 0.05) as f32);
            f.apex.push(c as u32);
            f.scan_lo.push(0);
            f.scan_hi.push(1);
            f.area.push(1e6 * (1.0 + unit(&mut s)));
            f.height.push(1e5);
            f.npts.push(9);
            f.flags.push(if unit(&mut s) < 0.05 { 2 } else { 16 });
            f.iso_group.push(0);
            f.charge.push(0);
            for v in [
                &mut f.shape,
                &mut f.snr,
                &mut f.fwhm,
                &mut f.asym,
                &mut f.purity,
            ] {
                v.push(0.5);
            }
            f.gaps.push(0);
            f.qreason.push(0);
            f.ms2_n.push(0);
            f.area_win.push(1.0);
            f.family_id.push(0);
            f.role.push(1);
            f.alt.push(0);
        }
    }
    let mut order: Vec<usize> = (0..f.len()).collect();
    order.sort_by(|&a, &b| f.mz[a].total_cmp(&f.mz[b]));
    let mut sorted = FileFeatures {
        dt: f.dt,
        ..Default::default()
    };
    for i in order {
        sorted.push_row(&f, i);
        sorted.family_id.push(0);
        sorted.role.push(1);
        sorted.alt.push(0);
    }
    sorted
}

/// `file(k)` with a dependent row (an isotopologue 1.00335 above, same RT) for every third row (every tenth if not sparse): the families that "per family" mode joins.
fn family_file(k: usize, clusters: usize, sparse: bool) -> FileFeatures {
    let f = file_of(k, clusters, sparse);
    let mut rows: Vec<(f64, usize, Option<usize>)> =
        (0..f.len()).map(|i| (f.mz[i], i, None)).collect();
    for i in (0..f.len()).step_by(if sparse { 3 } else { 10 }) {
        rows.push((f.mz[i] + 1.00335, i, Some(i)));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut at = vec![0usize; f.len()];
    for (pos, r) in rows.iter().enumerate() {
        if r.2.is_none() {
            at[r.1] = pos;
        }
    }
    let mut out = FileFeatures {
        dt: f.dt,
        ..Default::default()
    };
    for &(mz, i, root) in &rows {
        out.push_row(&f, i);
        let last = out.len() - 1;
        out.mz[last] = mz;
        out.family_id.push(root.map_or(0, |r| at[r] as u32 + 1));
        out.role.push(if root.is_some() { 2 } else { 1 });
        out.alt.push(0);
    }
    out
}

/// The files of the series, counting how many times a file is asked for.
struct Counting(Vec<Vec<u8>>, usize);

impl Stored for Counting {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        self.1 += 1;
        Ok(self.0[k].clone())
    }
}

fn series() -> Counting {
    let r = Recipe::default();
    Counting(
        (0..FILES)
            .map(|k| write_file(&file(k), &r, &Origin::default()))
            .collect(),
        0,
    )
}

fn family_series() -> Counting {
    family_series_of(FILES, CLUSTERS, true)
}

fn family_series_of(files: usize, clusters: usize, sparse: bool) -> Counting {
    let r = Recipe::default();
    Counting(
        (0..files)
            .map(|k| write_file(&family_file(k, clusters, sparse), &r, &Origin::default()))
            .collect(),
        0,
    )
}

fn recipe(mode: Mode) -> Recipe {
    Recipe {
        mode,
        weak_files: 0,
        fill_gaps: false,
        ..Recipe::default()
    }
}

fn aligned_bytes(src: &mut Counting, mb: f64, r: &Recipe) -> Result<Vec<u8>> {
    let open = |_: usize| -> Result<Index<Vec<u8>>> { unreachable!("no weak tier, no filling") };
    let (t, _) = align_and_fill_stored(src, &MemBudget::mb(mb), r, open)?;
    let mut out = Vec::new();
    write_table_to(&mut out, &t, r, &Origin::default(), &mut (&[][..])).unwrap();
    Ok(out)
}

#[test]
fn slabs_give_the_bytes_of_the_alignment_in_memory() {
    let r = recipe(Mode::Ion);
    let mut whole = series();
    let reference = aligned_bytes(&mut whole, 1024.0, &r).unwrap();
    assert_eq!(whole.1, 3 * FILES, "in memory: the survey, one read of each file to align and one for the summaries of the rows");
    let table = mzlab_feat::store::read_table(&reference).unwrap().0;
    let rows = table.len();
    assert!(rows > 300, "{rows} rows");
    assert!(
        table.is_id_ordered(),
        "the rows come out in the order that defines the ids"
    );
    for mb in [0.3, 0.4] {
        let mut s = series();
        let got = aligned_bytes(&mut s, mb, &r).unwrap();
        assert!(got == reference, "{mb} MB: different bytes");
        assert!(
            s.1 >= 3 * FILES,
            "{mb} MB: {} reads of files, expected several slabs",
            s.1
        );
    }
}

/// The series of `file_of(k, CLUSTERS, false)` with every RT moved by `0.15 * k` minutes (more than the RT gap).
fn drifted_series() -> Counting {
    let r = Recipe::default();
    Counting(
        (0..FILES)
            .map(|k| {
                let mut f = file_of(k, CLUSTERS, false);
                for v in [&mut f.rt, &mut f.left, &mut f.right] {
                    v.iter_mut().for_each(|x| *x += 0.15 * k as f32);
                }
                write_file(&f, &r, &Origin::default())
            })
            .collect(),
        0,
    )
}

#[test]
fn the_rt_correction_in_slabs_gives_the_bytes_of_the_correction_in_memory() {
    let mut r = recipe(Mode::Ion);
    r.rt.min_height = 1e4;
    let plain = aligned_bytes(&mut drifted_series(), 1024.0, &r).unwrap();
    r.align.warp = mzlab_feat::rtwarp::WarpParams {
        method: mzlab_feat::rtwarp::Method::Loess,
        coarse_gap: 1.0,
        ..Default::default()
    };
    let reference = aligned_bytes(&mut drifted_series(), 1024.0, &r).unwrap();
    let rows = |b: &[u8]| mzlab_feat::store::read_table(b).unwrap().0.len();
    assert!(
        rows(&reference) < rows(&plain),
        "{} rows corrected, {} not",
        rows(&reference),
        rows(&plain)
    );
    for mb in [0.3, 0.4] {
        let got = aligned_bytes(&mut drifted_series(), mb, &r).unwrap();
        assert!(got == reference, "{mb} MB: different bytes");
    }
}

#[test]
fn the_families_in_slabs_give_the_bytes_of_the_families_in_memory() {
    let r = recipe(Mode::Family);
    let mut whole = family_series();
    let reference = aligned_bytes(&mut whole, 1024.0, &r).unwrap();
    let table = mzlab_feat::store::read_table(&reference).unwrap().0;
    assert!(
        table.len() > 300 && table.member_mz.len() > table.len(),
        "families with members: {} rows",
        table.len()
    );
    for mb in [0.4, 0.5, 0.6] {
        let mut s = family_series();
        let got = aligned_bytes(&mut s, mb, &r).unwrap();
        assert!(got == reference, "{mb} MB: different bytes");
        assert!(
            s.1 > 4 * FILES,
            "{mb} MB: {} reads of files, expected several slabs",
            s.1
        );
    }
}

/// A long series (12 files, many features each): the families do not fit in memory in the budget, which is what the page refused before, and in slabs
/// they give the bytes of the alignment in memory.
#[test]
fn a_long_series_of_families_completes_in_a_budget_that_refused_it() {
    let r = recipe(Mode::Family);
    let reference = aligned_bytes(&mut family_series_of(12, 3000, false), 1024.0, &r).unwrap();
    let mut s = family_series_of(12, 3000, false);
    let got = aligned_bytes(&mut s, LONG_MB, &r).unwrap();
    assert!(got == reference, "different bytes");
    assert!(
        s.1 > 4 * 12,
        "{} reads of files, expected several slabs",
        s.1
    );
}

#[test]
fn a_budget_that_fits_nothing_is_refused_with_the_key() {
    let mut s = series();
    let e = aligned_bytes(&mut s, 0.01, &recipe(Mode::Ion)).unwrap_err();
    assert_eq!(e.key, "error.memoryBudget");
    assert!(e.params.iter().any(|(k, v)| *k == "fase" && v == "align"));
    // per family too: not even the group of every row fits
    let mut f = series();
    let e = aligned_bytes(&mut f, 0.01, &recipe(Mode::Family)).unwrap_err();
    assert_eq!(e.key, "error.memoryBudget");
    assert!(aligned_bytes(&mut series(), 64.0, &recipe(Mode::Family)).is_ok());
}

#[test]
fn a_lean_read_of_a_range_is_the_rows_and_columns_asked_for() {
    let bytes = write_file(&file(0), &Recipe::default(), &Origin::default());
    let (full, _) = read_file(&bytes).unwrap();
    let (lo, hi) = (full.mz[full.len() / 3], full.mz[2 * full.len() / 3]);
    let want = Want {
        lean: true,
        mz: Some((lo, hi)),
    };
    let (part, _) = read_file_from(&mut Slice(&bytes), bytes.len() as u64, want).unwrap();
    let (a, b) = (
        full.mz.partition_point(|&m| m < lo),
        full.mz.partition_point(|&m| m < hi),
    );
    assert_eq!(part.mz, full.mz[a..b]);
    assert_eq!(part.area, full.area[a..b]);
    assert_eq!(part.flags, full.flags[a..b]);
    assert!(
        part.snr.is_empty() && part.apex.is_empty(),
        "the other columns stay empty"
    );
    let mut sink: Vec<u8> = Vec::new();
    sink.write_at(3, b"x").unwrap();
    assert_eq!(sink, [0, 0, 0, b'x']);
}
