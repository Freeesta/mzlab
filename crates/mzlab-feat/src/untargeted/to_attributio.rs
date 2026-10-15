//! The `untargeted` table as the input of the cascade of levels of Attributio, so that the cascade runs on raw data without a table from mzmine. Rewritten
//! from scratch after the layout of the merged feature table that mzmine 3.9.0 exports (MIT, (c) 2004-2022 The mzmine Development Team; the columns follow
//! `io/export_features_gnps/fbmn/` and `io/export_features_csv/`, one `feature_state` and `area` per sample); no code copied.
//! Paper: MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! `build` returns the `Campaign` and the `Sources` that `attributio::readers`/`sources` build from a campaign folder: each file of the series is a site
//! (`F1`, `F2`, ...) with the sorbents `U` (the file) and `V` (empty), so a feature is detected where its cell is detected or filled; the feature id is the 1-based row; the MS2 of a
//! row is the spectrum of its most intense scan (`most_intense_ms2`). The sources that need other programs (SIRIUS, GNPS2, mzmine libraries, standards,
//! the chemist's decisions) are empty, which is the case of a campaign without those folders.

use super::ident::Idents;
use super::isotopes::IsoGroups;
use crate::attributio::campaign::Env;
use crate::attributio::error::Result;
use crate::attributio::frame::{to_csv, Frame};
use crate::attributio::readers::feature_decisions::DecisionTable;
use crate::attributio::readers::ms2::Spectrum;
use crate::attributio::readers::sirius::SiriusTables;
use crate::attributio::sources::{PolSources, Sources};
use crate::attributio::Campaign;
use crate::ms2spec::{pick, read_peaks};
use crate::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED};
use mzlab_core::error::Result as CoreResult;
use mzlab_core::idxread::{Index, ReadAt};
use std::collections::HashMap;

/// `campaign.toml` of the table: one site per file, two sorbents (the cascade is made for dual-SPE campaigns; `V` is never detected), the layout of the sample columns of mzmine.
fn campaign_toml(files: usize) -> String {
    let mut s = String::from(
        "[campaign]\nid = \"UNTARGETED\"\nname = \"Untargeted table\"\n\n[mzmine]\nsorbents = [\"U\", \"V\"]\nsample_column = \"datafile:{prefix}_{sorbent}_{tag}.mzML:{field}\"\n\
         polarity_tag = { POS = \"POS\", NEG = \"NEG\" }\n",
    );
    let mut names = Vec::new();
    for k in 1..=files {
        s.push_str(&format!(
            "\n[sites.F{k}]\nfile_prefix = \"F{k}\"\nlabel = \"File {k}\"\nshort = \"F{k}\"\n"
        ));
        names.push(format!("\"F{k}\""));
    }
    s.push_str(&format!(
        "\n[matrices.all]\nsites = [{}]\ndir = \"All\"\nlabel = \"All files\"\n",
        names.join(", ")
    ));
    s
}

/// The merged table of the polarity: `id`, `mz`, `rt`, the signs of isotopes and identities, then `feature_state` and `area` of each file.
fn frame_of(t: &Aligned, iso: &IsoGroups, idents: &Idents, pol: &str) -> Result<Frame> {
    let mut cols: Vec<String> = [
        "id",
        "mz",
        "rt",
        "iso_group",
        "charge",
        "family",
        "neutral_mass",
    ]
    .map(String::from)
    .to_vec();
    for k in 1..=t.n_files {
        cols.push(format!("datafile:F{k}_U_{pol}.mzML:feature_state"));
        cols.push(format!("datafile:F{k}_U_{pol}.mzML:area"));
        cols.push(format!("datafile:F{k}_V_{pol}.mzML:feature_state"));
        cols.push(format!("datafile:F{k}_V_{pol}.mzML:area"));
    }
    let (group, charge) = iso.per_row(t.len());
    let rows: Vec<Vec<String>> = (0..t.len())
        .map(|r| {
            let mut row = vec![
                (r + 1).to_string(),
                t.mz[r].to_string(),
                t.rt[r].to_string(),
                group[r].to_string(),
                charge[r].to_string(),
                idents.family.get(r).copied().unwrap_or(0).to_string(),
                idents
                    .ident
                    .neutral
                    .get(r)
                    .map_or(String::new(), |x| x.to_string()),
            ];
            for k in 0..t.n_files {
                let c = t.cell(r, k);
                let state = match t.state[c] {
                    CELL_DETECTED | CELL_FILLED => "DETECTED",
                    CELL_ABSENT => "UNKNOWN",
                    _ => "UNKNOWN",
                };
                row.push(state.to_string());
                row.push(t.area[c].to_string());
                row.push("UNKNOWN".to_string());
                row.push("0".to_string());
            }
            row
        })
        .collect();
    Frame::parse(&to_csv(&cols, &rows, ','), ',')
}

/// The `Campaign` and the `Sources` of the table `t` of polarity `polarity` (`spectra[r]` = MS2 of row `r`, empty for none).
pub fn build(
    t: &Aligned,
    iso: &IsoGroups,
    idents: &Idents,
    spectra: &[Option<Spectrum>],
    polarity: i8,
) -> Result<(Campaign, Sources)> {
    let campaign = Campaign::from_toml(
        &campaign_toml(t.n_files),
        "untargeted.toml",
        "untargeted",
        Env::default(),
    )?;
    let pol = if polarity >= 0 { "POS" } else { "NEG" };
    let table = frame_of(t, iso, idents, pol)?;
    let feature_spectra: HashMap<i64, Spectrum> = spectra
        .iter()
        .enumerate()
        .filter_map(|(r, s)| s.clone().map(|s| (r as i64 + 1, s)))
        .collect();
    let counts = vec![
        (format!("features {pol}"), table.len()),
        (format!("feature spectra {pol}"), feature_spectra.len()),
    ];
    let p = PolSources {
        pol: pol.to_string(),
        table,
        twins: Default::default(),
        mzmine: Vec::new(),
        gnps_library: Vec::new(),
        gnps_network: Vec::new(),
        feature_spectra,
        library_spectra: Default::default(),
    };
    let sources = Sources {
        pols: vec![p],
        sirius: SiriusTables::default(),
        standards_library: Frame::with_columns(&[]),
        standards: None,
        level1_decisions: Vec::new(),
        decisions: DecisionTable::default(),
        warnings: Vec::new(),
        counts,
    };
    Ok((campaign, sources))
}

/// The most intense MS2 scan of each row, read from the `.mzidx` of the file where the ion is highest: among the MS2 scans whose isolation window holds the
/// m/z of the row and whose time lies in the RT range of the peak there, the one with the highest TIC (`ms2spec::pick`). `open(k)` gives the index of file `k`.
pub fn most_intense_ms2<R: ReadAt>(
    t: &Aligned,
    polarity: i8,
    mut open: impl FnMut(usize) -> CoreResult<Index<R>>,
) -> CoreResult<Vec<Option<Spectrum>>> {
    let mut out: Vec<Option<Spectrum>> = vec![None; t.len()];
    for k in 0..t.n_files {
        let mine: Vec<usize> = (0..t.len())
            .filter(|&r| {
                let c = t.cell(r, k);
                t.state[c] == CELL_DETECTED
                    && (0..t.n_files).all(|j| t.height[t.cell(r, j)] <= t.height[c])
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        let mut ix = open(k)?;
        let mut by_rt: Vec<u32> = (0..ix.scans.len() as u32)
            .filter(|&g| ix.scans[g as usize].level == 2)
            .collect();
        by_rt.sort_by(|&a, &b| ix.scans[a as usize].rt.total_cmp(&ix.scans[b as usize].rt));
        let chosen: Vec<(usize, u32)> = mine
            .iter()
            .filter_map(|&r| {
                let c = t.cell(r, k);
                let (rt, w) = (f64::from(t.apex_rt[c]), f64::from(t.width[c]) / 2.0);
                pick(&ix, &by_rt, t.mz[r], polarity, rt - w, rt + w).map(|g| (r, g))
            })
            .collect();
        let wanted: Vec<u32> = chosen.iter().map(|c| c.1).collect();
        let peaks = read_peaks(&mut ix, polarity, &wanted)?;
        for (r, g) in chosen {
            if let Some((mz, it)) = peaks.get(&g) {
                out[r] = Some(Spectrum {
                    mz: mz.clone(),
                    intensity: it.clone(),
                    precursor: t.mz[r],
                });
            }
        }
    }
    Ok(out)
}
