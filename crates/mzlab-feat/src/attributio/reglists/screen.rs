// Ported from Attributio reglists/screen.py.
//! B2: MS1 suspect screening of a campaign against the approved lists. The suspects are the union of the chosen lists (one per InChIKey first block, LC-ESI
//! amenable «true» or «doubtful», [M+H]+ or [M-H]- inside the MS1 window); every mass hit of a feature within 5 ppm is written with a class, nothing is dropped
//! silently. Evidence classes: A = MS2 + matching SIRIUS formula + coherent RT, B = MS2 + one of the two, C = MS2 only, D = no MS2 (not reportable), X = the
//! feature is already annotated at Levels 1-3 as another compound. An [M+NH4]+ or [M+Na]+ hit with no [M+H]+ hit of the same suspect within 0.15 min is an
//! isolated adduct and goes one class down. The inputs are plain rows (suspects, features, evidence): where they come from is the caller's business.
//! Not ported: the Crippen logP that the reference computes with RDKit for a suspect that has a SMILES but no XLogP in its list or in the registry; such a
//! suspect gets no logP, so its RT is «n.a.».

use super::b1::{in_window, StructRow};
use super::catalog;
use crate::attributio::annotate::text::fmt_g;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::frame::to_num;
use crate::attributio::tox::snapshot::MembershipRow;
use crate::attributio::tox::vocab::{AMENABLE, FALSE, TRUE};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub use super::screen_types::*;

/// `round(x, n)` of Python: the correctly rounded decimal, ties on the exact value to even.
pub fn round_py(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap_or(x)
}

fn unique<'a>(vals: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    for v in vals {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    seen
}

/// The union of the entries of `lists` that a structural screening can use and whose [M+H]+ or [M-H]- falls inside the window, one row per first block
/// (the values of the first list of the catalogue order win).
pub fn suspects(
    mem: &[MembershipRow],
    lists: &[String],
    window: Option<(f64, f64)>,
) -> Vec<Suspect> {
    let ids: Vec<&str> = lists.iter().map(String::as_str).collect();
    let order = catalog::ordered(&ids);
    let rank = |l: &str| order.iter().position(|o| o == l).unwrap_or(usize::MAX);
    let mut d: Vec<&MembershipRow> = mem
        .iter()
        .filter(|m| lists.contains(&m.list_id) && AMENABLE.contains(&m.lc_esi_amenable.as_str()))
        .filter(|m| in_window(&m.mono_mass, window))
        .collect();
    d.sort_by(|a, b| (&a.first_block, rank(&a.list_id)).cmp(&(&b.first_block, rank(&b.list_id))));
    let mut out: Vec<Suspect> = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let j = d[i..]
            .iter()
            .position(|m| m.first_block != d[i].first_block)
            .map_or(d.len(), |p| i + p);
        out.push(union_row(&d[i..j]));
        i = j;
    }
    out
}

fn union_row(g: &[&MembershipRow]) -> Suspect {
    let f = g[0];
    let ids = unique(g.iter().map(|m| m.list_id.as_str()));
    let ordered = catalog::ordered(&ids);
    Suspect {
        skeleton: f.first_block.clone(),
        name: f.name.clone(),
        formula: f.formula.clone(),
        mono_mass: f.mono_mass.trim().parse().unwrap_or(f64::NAN),
        smiles: f.smiles.clone(),
        inchikey: f.inchikey.clone(),
        lc_esi: f.lc_esi_amenable.clone(),
        xlogp: f.xlogp.clone(),
        lists: ordered.join(", "),
        list_labels: ordered
            .iter()
            .map(|l| catalog::label(l))
            .collect::<Vec<_>>()
            .join(", "),
        attributes: unique(g.iter().map(|m| m.list_attribute.as_str())).join(" || "),
        is_tp: if g.iter().any(|m| m.is_tp == TRUE) {
            TRUE
        } else {
            FALSE
        }
        .to_string(),
        parent: unique(
            g.iter()
                .map(|m| m.parent.as_str())
                .filter(|p| !p.is_empty()),
        )
        .join(", "),
        logp: None,
        logp_source: String::new(),
    }
}

/// XLogP3 of each suspect: the list's own column, else the chemical registry (`registry`: first block → XLogP3). No Crippen fallback.
pub fn fill_logp(sus: &mut [Suspect], registry: &HashMap<String, Option<f64>>) {
    for s in sus {
        let own = to_num(&s.xlogp);
        let reg = registry
            .get(&s.skeleton)
            .copied()
            .flatten()
            .filter(|v| !v.is_nan());
        (s.logp, s.logp_source) = match (own, reg) {
            (Some(v), _) => (Some(v), "XLogP3".into()),
            (None, Some(v)) => (Some(v), "XLogP3 (registry)".into()),
            _ => (None, String::new()),
        };
    }
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

fn polyfit1(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxx: f64 = x.iter().map(|a| (a - mx) * (a - mx)).sum();
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let k = sxy / sxx;
    (k, my - k * mx)
}

fn robust_sd(res: &[f64]) -> f64 {
    let m = median(res);
    1.4826 * median(&res.iter().map(|r| (r - m).abs()).collect::<Vec<_>>())
}

/// The robust linear fit RT ~ logP on the Level 2-3 features that have a logP: a least-squares line refitted three times without the points farther than
/// three robust SD.
pub fn rt_model(feat: &[FeatureRow]) -> Result<RtModel> {
    let d: Vec<&FeatureRow> = feat
        .iter()
        .filter(|f| (f.level == 2 || f.level == 3) && f.xlogp.is_some())
        .collect();
    if d.len() < 2 {
        return Err(CampaignError::value(
            "RT ~ logP needs at least two Level 2-3 features with a logP",
        ));
    }
    let x: Vec<f64> = d.iter().map(|f| f.xlogp.unwrap_or(f64::NAN)).collect();
    let y: Vec<f64> = d.iter().map(|f| f.rt).collect();
    let (mut k, mut q) = polyfit1(&x, &y);
    for _ in 0..3 {
        let res: Vec<f64> = x.iter().zip(&y).map(|(a, b)| b - (k * a + q)).collect();
        let mad = robust_sd(&res);
        let keep: Vec<usize> = (0..x.len())
            .filter(|i| res[*i].abs() <= 3.0 * mad)
            .collect();
        if keep.len() < 2 {
            return Err(CampaignError::value(
                "RT ~ logP: fewer than two features left after the robust refit",
            ));
        }
        (k, q) = polyfit1(
            &keep.iter().map(|i| x[*i]).collect::<Vec<_>>(),
            &keep.iter().map(|i| y[*i]).collect::<Vec<_>>(),
        );
    }
    let res: Vec<f64> = x.iter().zip(&y).map(|(a, b)| b - (k * a + q)).collect();
    Ok(RtModel {
        slope: k,
        intercept: q,
        sd: robust_sd(&res),
        n: d.len(),
    })
}

fn digits_after(s: &[char], mut i: usize) -> (usize, usize) {
    let start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let n = if i > start {
        s[start..i].iter().collect::<String>().parse().unwrap_or(1)
    } else {
        1
    };
    (n, i)
}

/// Theoretical M+2 / M of a formula from its Cl, Br and S (as a fraction): 31.96 % per Cl, 97.28 % per Br, 4.52 % per S.
pub fn m2_ratio(formula: &str) -> f64 {
    let s: Vec<char> = formula.chars().collect();
    let (mut cl, mut br, mut sulfur) = (0usize, 0usize, 0usize);
    let mut i = 0;
    while i < s.len() {
        let next = s.get(i + 1).copied();
        match (s[i], next) {
            ('C', Some('l')) => cl += digits_after(&s, i + 2).0,
            ('B', Some('r')) => br += digits_after(&s, i + 2).0,
            ('S', n) if !n.is_some_and(|c| c.is_ascii_lowercase()) => {
                sulfur += digits_after(&s, i + 1).0
            }
            _ => {}
        }
        i += 1;
    }
    (cl as f64 * 31.96 + br as f64 * 97.28 + sulfur as f64 * 4.52) / 100.0
}

fn sorbent_of(r: &FeatureRow, cfg: &ScreenConfig) -> String {
    if cfg.sorbents.len() != 2 {
        return String::new();
    }
    let lab = |s: &String| cfg.sorbent_labels.get(s).unwrap_or(s).clone();
    let (a, b) = (
        r.any_sorbent.first().copied().unwrap_or(false),
        r.any_sorbent.get(1).copied().unwrap_or(false),
    );
    match (a, b) {
        (true, true) => "both".into(),
        (true, false) => format!("{} only", lab(&cfg.sorbents[0])),
        _ => format!("{} only", lab(&cfg.sorbents[1])),
    }
}

/// Everything a hit needs besides the suspect and the feature.
struct Ctx<'a> {
    model: RtModel,
    ev: &'a Evidence,
    cfg: &'a ScreenConfig,
}

fn make_hit(s: &Suspect, r: &FeatureRow, add: &str, theo: f64, ppm: f64, c: &Ctx) -> Hit {
    let key = (r.pol.clone(), r.id);
    let (ms2, blank) =
        c.ev.ms2_blank
            .get(&key)
            .cloned()
            .unwrap_or((false, "?".into()));
    let fmatch = if r.formula.is_empty() {
        "n.a."
    } else if r.formula == s.formula {
        YES
    } else {
        NO
    };
    let pred = s.logp.map(|lp| c.model.slope * lp + c.model.intercept);
    let rtok = match pred {
        None => "n.a.",
        Some(p) if (r.rt - p).abs() <= RT_SD_K * c.model.sd => YES,
        Some(_) => NO,
    };
    let good = usize::from(fmatch == YES) + usize::from(rtok == YES);
    let class = if !ms2 { "D" } else { ["C", "B", "A"][good] };
    let ratio = m2_ratio(&s.formula);
    Hit {
        class: class.into(),
        suspect: s.name.clone(),
        lists: s.lists.clone(),
        list_labels: s.list_labels.clone(),
        is_tp: s.is_tp.clone(),
        parent: s.parent.clone(),
        formula: s.formula.clone(),
        polarity: r.pol.clone(),
        feature_id: r.id,
        adduct: add.into(),
        mz_theor: round_py(theo, 5),
        mz_obs: round_py(r.mz, 5),
        ppm: round_py(ppm, 2),
        rt: round_py(r.rt, 2),
        rt_pred: pred.map(|p| round_py(p, 2)),
        rt_logp: rtok.into(),
        logp: s.logp.map(|v| round_py(v, 2)),
        logp_source: s.logp_source.clone(),
        ms2: if ms2 { YES } else { NO }.into(),
        sirius_formula: r.formula.clone(),
        formula_match: fmatch.into(),
        feature_level: r.level,
        already_annotated_as: c.ev.annotated.get(&key).cloned().unwrap_or_default(),
        in_blanks: blank,
        sites: c
            .cfg
            .sites
            .iter()
            .zip(&r.in_site)
            .filter(|(_, p)| **p)
            .map(|(s, _)| s.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        sorbent: sorbent_of(r, c.cfg),
        m2_theor: if ratio >= 0.05 {
            format!("{:.0}%", ratio * 100.0)
        } else {
            String::new()
        },
        suspect_skeleton: s.skeleton.clone(),
        suspect_inchikey: s.inchikey.clone(),
        list_attributes: s.attributes.clone(),
        isolated_adduct: false,
    }
}

/// Every hit of the suspects in one polarity, features sorted by m/z (stable) and searched within ±5 ppm.
fn hits_in_polarity(
    pol: &str,
    adducts: &[(&str, f64)],
    todo: &[Suspect],
    feat: &[FeatureRow],
    c: &Ctx,
) -> Vec<Hit> {
    let f: Vec<&FeatureRow> = feat.iter().filter(|r| r.pol == pol).collect();
    let mut order: Vec<usize> = (0..f.len()).collect();
    order.sort_by(|a, b| f[*a].mz.total_cmp(&f[*b].mz));
    let smz: Vec<f64> = order.iter().map(|i| f[*i].mz).collect();
    let mut hits = Vec::new();
    for s in todo {
        for (add, dm) in adducts {
            let theo = s.mono_mass + dm;
            let tol = theo * PPM * 1e-6;
            let lo = smz.partition_point(|v| *v < theo - tol * 1.0000001);
            let hi = smz.partition_point(|v| *v <= theo + tol * 1.0000001);
            let mut js: Vec<usize> = order[lo..hi].to_vec();
            js.sort_unstable();
            for j in js {
                let ppm = (f[j].mz - theo) / theo * 1e6;
                if ppm.abs() <= PPM {
                    hits.push(make_hit(s, f[j], add, theo, ppm, c));
                }
            }
        }
    }
    hits
}

fn class_rank(c: &str) -> usize {
    CLASSES.iter().position(|x| *x == c).unwrap_or(usize::MAX)
}

/// Isolated adducts go one class down, hits on a feature already annotated as another compound become X, then the hits are sorted.
fn finish_hits(hits: &mut [Hit]) {
    let mut mh: HashMap<String, Vec<f64>> = HashMap::new();
    for h in hits.iter().filter(|h| h.adduct == "[M+H]+") {
        mh.entry(h.suspect.clone()).or_default().push(h.rt);
    }
    for h in hits.iter_mut() {
        let near = mh
            .get(&h.suspect)
            .is_some_and(|v| v.iter().any(|x| (h.rt - x).abs() <= ISOLATED_RT_MIN));
        h.isolated_adduct = (h.adduct == "[M+NH4]+" || h.adduct == "[M+Na]+") && !near;
        if h.isolated_adduct {
            h.class = match h.class.as_str() {
                "A" => "B",
                "B" | "C" => "C",
                other => other,
            }
            .into();
        }
        if !h.already_annotated_as.is_empty() {
            h.class = "X".into();
        }
    }
    hits.sort_by(|a, b| {
        class_rank(&a.class)
            .cmp(&class_rank(&b.class))
            .then_with(|| a.lists.cmp(&b.lists))
            .then_with(|| a.suspect.cmp(&b.suspect))
            .then_with(|| a.ppm.total_cmp(&b.ppm))
    });
}

fn summary_row(lid: &str, todo: &[Suspect], hits: &[Hit]) -> SummaryRow {
    let union = lid == "UNION";
    let has = |lists: &str| union || lists.split(", ").any(|l| l == lid);
    let h: Vec<&Hit> = hits.iter().filter(|h| has(&h.lists)).collect();
    let names = |f: &dyn Fn(&&Hit) -> bool| {
        h.iter()
            .filter(|x| f(x))
            .map(|x| x.suspect.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    };
    let mut row = SummaryRow {
        list_id: lid.into(),
        list: if union {
            "UNION (dedup)".into()
        } else {
            catalog::label(lid)
        },
        suspects_screened: todo.iter().filter(|s| has(&s.lists)).count(),
        suspects_with_hits: names(&|_| true),
        hits: h.len(),
        suspects_with_ms2_hit: names(&|x| x.ms2 == YES && x.class != "X"),
        ..SummaryRow::default()
    };
    for (i, c) in CLASSES.iter().enumerate() {
        row.class_hits[i] = h.iter().filter(|x| x.class == *c).count();
        row.class_suspects[i] = names(&|x| x.class == *c);
    }
    row
}

/// B2 of a campaign. `structures` are the structures of the B1 identities (a suspect with the first block of one is listed apart, not screened);
/// `registry_xlogp` maps a first block to the registry's XLogP3.
pub fn screen_campaign(
    feat: &[FeatureRow],
    ev: &Evidence,
    structures: &[StructRow],
    mem: &[MembershipRow],
    registry_xlogp: &HashMap<String, Option<f64>>,
    cfg: &ScreenConfig,
) -> Result<B2> {
    let ids: Vec<&str> = cfg.lists.iter().map(String::as_str).collect();
    let lists = catalog::ordered(&ids);
    let mut sus = suspects(mem, &lists, cfg.window);
    fill_logp(&mut sus, registry_xlogp);
    let mut id_sk: HashMap<&str, &str> = HashMap::new();
    let mut by_name: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for s in structures.iter().filter(|s| s.first_block.is_some()) {
        by_name
            .entry(&s.nome_comune)
            .or_default()
            .insert(s.first_block.as_deref().unwrap_or(""));
    }
    for (nc, blocks) in &by_name {
        for b in blocks {
            id_sk.insert(b, nc);
        }
    }
    let (already, todo): (Vec<Suspect>, Vec<Suspect>) = sus
        .iter()
        .cloned()
        .partition(|s| id_sk.contains_key(s.skeleton.as_str()));
    let already = already
        .into_iter()
        .map(|s| {
            let a = id_sk[s.skeleton.as_str()].to_string();
            (s, a)
        })
        .collect();
    let model = rt_model(feat)?;
    let c = Ctx { model, ev, cfg };
    let mut hits = hits_in_polarity("POS", &POS_ADDUCTS, &todo, feat, &c);
    hits.extend(hits_in_polarity("NEG", &NEG_ADDUCTS, &todo, feat, &c));
    finish_hits(&mut hits);
    let summary = lists
        .iter()
        .map(String::as_str)
        .chain(["UNION"])
        .map(|l| summary_row(l, &todo, &hits))
        .collect();
    Ok(B2 {
        suspects: sus,
        already,
        hits,
        summary,
        rt_model: model,
        lists,
    })
}

/// The method paragraph of the report.
pub fn method_lines(b2: &B2, window: Option<(f64, f64)>) -> Vec<String> {
    let m = b2.rt_model;
    let win = window.map_or("none".to_string(), |(lo, hi)| {
        format!("m/z {}-{}", fmt_g(lo), fmt_g(hi))
    });
    vec![
        format!("Suspects: union of lists {}; dedup by first InChIKey block; LC-ESI 'true' or 'doubtful'; [M+H]+ or [M-H]- within MS1 window ({win}).", b2.lists.join(", ")),
        format!("Suspects already identified in B1 (same first block as a Lvl 1-3 identity): {}, not re-evaluated ('Gia identificati' sheet).", b2.already.len()),
        "Features: all features from matrix sites brought to annotation (DETECTED, mzMine blank subtraction already applied).".into(),
        "Adducts: [M+H]+, [M+NH4]+, [M+Na]+ (POS); [M-H]-, [M+Cl]- (NEG); tolerance 5 ppm.".into(),
        format!("RT ~ logP model: RT = {:.2} * logP + {:.2} min, robust SD {:.2} min, on {} Lvl 2-3 features; coherent if |residual| <= {} SD. Broad criterion: excludes only gross inconsistencies.", m.slope, m.intercept, m.sd, m.n, fmt_g(RT_SD_K)),
        "Classes: A = MS2 + matching SIRIUS formula + coherent RT; B = MS2 + one of the two; C = MS2 without the other two; D = no MS2 (not reportable); X = feature already annotated at Lvl 1-3 as another compound.".into(),
        format!("An [M+NH4]+ or [M+Na]+ without [M+H]+ of the same suspect within {} min drops one class.", fmt_g(ISOLATED_RT_MIN)),
        "Flags, not filters: blanks (n/N), theoretical M+2 for Cl/Br/S (to be verified in Xcalibur).".into(),
        "No candidate is an identification: it is the working list for B3. No counts affected.".into(),
    ]
}
