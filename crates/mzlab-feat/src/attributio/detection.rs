// Ported from Attributio campaign.py, annotate.py.
//! The DETECTED rule per site: a feature is present at a site and sorbent when its `feature_state` there is `DETECTED` and, if the campaign asks for a blank
//! fold (`[presence] blank_fold`), its area is at least that many times the blank abundance. The detection table has one row per feature of a polarity.

use super::campaign::{Campaign, PROTON};
use super::error::{CampaignError, Result};
use super::frame::Frame;
use super::sorbents::blank_abundance;

/// A table that lacks the column Python would index (`KeyError`).
fn key_error(col: &str) -> CampaignError {
    CampaignError::value(format!("KeyError: '{col}'"))
}

/// One feature in the detection table: where it is detected (`in_<site>`, union of the sorbents) and in which sorbent (`any_<sorbent>`).
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub pol: String,
    pub id: i64,
    pub mz: Option<f64>,
    pub rt: Option<f64>,
    pub in_site: Vec<(String, bool)>,
    pub any_sorbent: Vec<(String, bool)>,
}

impl Detection {
    pub fn site(&self, site: &str) -> Option<bool> {
        self.in_site
            .iter()
            .find(|(k, _)| k == site)
            .map(|(_, v)| *v)
    }
    pub fn sorbent(&self, sorbent: &str) -> Option<bool> {
        self.any_sorbent
            .iter()
            .find(|(k, _)| k == sorbent)
            .map(|(_, v)| *v)
    }
}

/// `has_site`: the table has the `feature_state` column of the site for the first sorbent.
pub fn has_site(c: &Campaign, f: &Frame, site: &str, pol: &str) -> bool {
    let sorbent = c.sorbents.first().map_or("", |s| s.as_str());
    f.col(&c.col(site, sorbent, pol, "feature_state")).is_some()
}

/// The blank abundance of every row for one sorbent: the column added from the per-sorbent list, else the text of `blank_subtraction`.
fn blank_abundances(f: &Frame, sorbent: &str) -> Result<Vec<f64>> {
    if let Some(c) = f.col(&format!("blank_abundance:{sorbent}")) {
        return Ok((0..f.len()).map(|r| f.num(r, c).unwrap_or(0.0)).collect());
    }
    let Some(c) = f.col("blank_subtraction") else {
        return Ok(vec![0.0; f.len()]);
    };
    (0..f.len())
        .map(|r| f.str_na(r, c).map_or(Ok(0.0), blank_abundance))
        .collect()
}

/// `detected_mask`: one flag per row for a site and a sorbent.
pub fn detected_mask(
    c: &Campaign,
    f: &Frame,
    site: &str,
    sorbent: &str,
    pol: &str,
) -> Result<Vec<bool>> {
    let state_name = c.col(site, sorbent, pol, "feature_state");
    let state = f.col(&state_name).ok_or_else(|| key_error(&state_name))?;
    let mut mask: Vec<bool> = (0..f.len())
        .map(|r| f.cell(r, state) == "DETECTED")
        .collect();
    let Some(fold) = c.presence_blank_fold else {
        return Ok(mask);
    };
    let area_name = c.col(site, sorbent, pol, "area");
    let area = f.col(&area_name).ok_or_else(|| key_error(&area_name))?;
    let blank = blank_abundances(f, sorbent)?;
    for (r, m) in mask.iter_mut().enumerate() {
        let a = f.num(r, area).unwrap_or(0.0);
        *m = *m && (blank[r] == 0.0 || a >= fold * blank[r]);
    }
    Ok(mask)
}

fn or_into(into: &mut [bool], with: &[bool]) {
    for (a, b) in into.iter_mut().zip(with) {
        *a = *a || *b;
    }
}

/// `site_detected_any`: DETECTED in at least one sorbent of the site (`sorbents` = all of the campaign when `None`).
pub fn site_detected_any(
    c: &Campaign,
    f: &Frame,
    site: &str,
    pol: &str,
    sorbents: Option<&[String]>,
) -> Result<Vec<bool>> {
    let mut m = vec![false; f.len()];
    for s in sorbents.unwrap_or(&c.sorbents) {
        or_into(&mut m, &detected_mask(c, f, site, s, pol)?);
    }
    Ok(m)
}

/// `matrix_detected` for a list of sites: DETECTED in at least one site that the table has.
pub fn sites_detected(
    c: &Campaign,
    f: &Frame,
    sites: &[String],
    pol: &str,
    sorbents: Option<&[String]>,
) -> Result<Vec<bool>> {
    let mut m = vec![false; f.len()];
    for s in sites {
        if has_site(c, f, s, pol) {
            or_into(&mut m, &site_detected_any(c, f, s, pol, sorbents)?);
        }
    }
    Ok(m)
}

/// `matrix_detected` for a matrix key of the campaign.
pub fn matrix_detected(
    c: &Campaign,
    f: &Frame,
    matrix: &str,
    pol: &str,
    sorbents: Option<&[String]>,
) -> Result<Vec<bool>> {
    let sites = c
        .matrix_sites(matrix)
        .ok_or_else(|| CampaignError::value(format!("KeyError: '{matrix}'")))?;
    sites_detected(c, f, sites, pol, sorbents)
}

/// `neutral_mass`: per feature the first valid (> 0) value among the campaign's columns, otherwise m/z minus (POS) or plus (NEG) the proton.
pub fn neutral_mass(c: &Campaign, f: &Frame, pol: &str) -> Vec<Option<f64>> {
    let cols: Vec<usize> = c
        .neutral_mass_columns
        .iter()
        .filter_map(|n| f.col(n))
        .collect();
    let delta = if pol == "POS" { PROTON } else { -PROTON };
    let mz = f.col("mz");
    (0..f.len())
        .map(|r| {
            let first = cols.iter().find_map(|&k| f.num(r, k).filter(|v| *v > 0.0));
            first.or_else(|| mz.and_then(|k| f.num(r, k)).map(|v| v - delta))
        })
        .collect()
}

/// `site_id_set`: the ids of the features detected at a site.
pub fn site_id_set(
    c: &Campaign,
    f: &Frame,
    site: &str,
    pol: &str,
    sorbents: Option<&[String]>,
) -> Result<Vec<i64>> {
    let m = site_detected_any(c, f, site, pol, sorbents)?;
    let id = f.col("id").ok_or_else(|| key_error("id"))?;
    Ok((0..f.len())
        .filter(|&r| m[r])
        .filter_map(|r| f.int(r, id))
        .collect())
}

/// `spe_overlap`: dual-SPE complementarity at one site, `(only first, both, only second)`.
pub fn spe_overlap(
    c: &Campaign,
    f: &Frame,
    site: &str,
    pol: &str,
    sorbents: Option<&[String]>,
) -> Result<(usize, usize, usize)> {
    let sb = sorbents.unwrap_or(&c.sorbents);
    if sb.len() < 2 {
        return Err(CampaignError::value("IndexError: tuple index out of range"));
    }
    let a = detected_mask(c, f, site, &sb[0], pol)?;
    let b = detected_mask(c, f, site, &sb[1], pol)?;
    let count = |p: fn(bool, bool) -> bool| a.iter().zip(&b).filter(|(x, y)| p(**x, **y)).count();
    Ok((
        count(|x, y| x && !y),
        count(|x, y| x && y),
        count(|x, y| y && !x),
    ))
}

/// How `site_area` combines the sorbents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Agg {
    Sum,
    Max,
}

/// `site_area`: the area at a site over the sorbents, counted only where DETECTED (gap-filled areas are 0).
pub fn site_area(
    c: &Campaign,
    f: &Frame,
    site: &str,
    pol: &str,
    sorbents: Option<&[String]>,
    agg: Agg,
) -> Result<Vec<f64>> {
    let mut out = vec![0.0f64; f.len()];
    for s in sorbents.unwrap_or(&c.sorbents) {
        let name = c.col(site, s, pol, "area");
        let k = f.col(&name).ok_or_else(|| key_error(&name))?;
        let det = detected_mask(c, f, site, s, pol)?;
        for (r, o) in out.iter_mut().enumerate() {
            let a = if det[r] {
                f.num(r, k).unwrap_or(0.0)
            } else {
                0.0
            };
            *o = if agg == Agg::Sum { *o + a } else { o.max(a) };
        }
    }
    Ok(out)
}

/// `detection_table`: `_id`, m/z, RT, `in_<site>` (dual-SPE union) and `any_<sorbent>` for a polarity.
pub fn detection_table(
    c: &Campaign,
    f: &Frame,
    pol: &str,
    sites: &[String],
) -> Result<Vec<Detection>> {
    let id = f.col("id").ok_or_else(|| key_error("id"))?;
    let mut rows: Vec<Detection> = (0..f.len())
        .map(|r| Detection {
            pol: pol.to_string(),
            id: f.int(r, id).unwrap_or(0),
            mz: f.col("mz").and_then(|k| f.num(r, k)),
            rt: f.col("rt").and_then(|k| f.num(r, k)),
            in_site: Vec::new(),
            any_sorbent: Vec::new(),
        })
        .collect();
    let mut anys: Vec<Vec<bool>> = c.sorbents.iter().map(|_| vec![false; f.len()]).collect();
    for site in sites {
        let mut m = vec![false; f.len()];
        if has_site(c, f, site, pol) {
            for (k, s) in c.sorbents.iter().enumerate() {
                let mk = detected_mask(c, f, site, s, pol)?;
                or_into(&mut m, &mk);
                or_into(&mut anys[k], &mk);
            }
        }
        for (r, row) in rows.iter_mut().enumerate() {
            row.in_site.push((site.clone(), m[r]));
        }
    }
    for (k, s) in c.sorbents.iter().enumerate() {
        for (r, row) in rows.iter_mut().enumerate() {
            row.any_sorbent.push((s.clone(), anys[k][r]));
        }
    }
    Ok(rows)
}

/// `DETECTED` per site and sorbent as the text of `feature_state` (what the table shows next to the flags), `None` when the column is missing.
pub fn feature_state(
    c: &Campaign,
    f: &Frame,
    site: &str,
    sorbent: &str,
    pol: &str,
) -> Option<Vec<String>> {
    let k = f.col(&c.col(site, sorbent, pol, "feature_state"))?;
    Some((0..f.len()).map(|r| f.cell(r, k).to_string()).collect())
}
