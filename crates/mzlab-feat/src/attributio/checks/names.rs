// Ported from Attributio chem/checks.py.
//! The name checks of the registry: one structure under several names, one name over several structures, a name outside the PubChem synonyms, the name source
//! against the loader, and the library entries that contradict their own precursor.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{
    finding, norm_name, Context, Evidence, Finding, Member, MzlibCheck, Value, CHECK_NAME_INCHIKEY,
    INFO, WARNING,
};

/// (a) one structure under several names (same campaign: MERGE; several campaigns: curation).
fn names_of_one_structure(ctx: &Context, out: &mut Vec<Finding>) {
    let mut by_block: BTreeMap<&str, Vec<&Member>> = BTreeMap::new();
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        by_block
            .entry(m.first_block.as_deref().unwrap_or(""))
            .or_default()
            .push(m);
    }
    for (blk, g) in by_block {
        let names: BTreeSet<&str> = g.iter().map(|m| m.name.as_str()).collect();
        if names.len() < 2 {
            continue;
        }
        let mut by_camp: BTreeMap<&str, Vec<&Member>> = BTreeMap::new();
        for m in &g {
            by_camp.entry(m.campaign.as_str()).or_default().push(m);
        }
        for (camp, h) in &by_camp {
            let hn: BTreeSet<&str> = h.iter().map(|m| m.name.as_str()).collect();
            if hn.len() > 1 {
                let value = h
                    .iter()
                    .map(|x| format!("{} [{}, L{}]", x.name, x.ik_basis, x.best_level))
                    .collect::<Vec<_>>()
                    .join("; ");
                out.push(finding(
                    CHECK_NAME_INCHIKEY,
                    WARNING,
                    camp,
                    &hn.iter().copied().collect::<Vec<_>>().join(" | "),
                    blk,
                    "MERGE: same structure under different names in the same campaign -> counted multiple times by identity_table()".into(),
                    Value::Text(value),
                ));
            }
        }
        if by_camp.len() > 1 {
            let per: Vec<(&str, BTreeSet<&str>)> = by_camp
                .iter()
                .map(|(c, h)| (*c, h.iter().map(|m| m.name.as_str()).collect()))
                .collect();
            if per.iter().map(|p| &p.1).collect::<BTreeSet<_>>().len() > 1 {
                let value = per
                    .iter()
                    .map(|(c, v)| {
                        format!("{c}: {}", v.iter().copied().collect::<Vec<_>>().join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                out.push(finding(
                    CHECK_NAME_INCHIKEY,
                    INFO,
                    &per.iter().map(|p| p.0).collect::<Vec<_>>().join("+"),
                    &names.iter().copied().collect::<Vec<_>>().join(" | "),
                    blk,
                    "same structure with different names across campaigns (curation to align?)"
                        .into(),
                    Value::Text(value),
                ));
            }
        }
    }
}

/// (b) one name over several structures.
fn structures_of_one_name(ctx: &Context, out: &mut Vec<Finding>) {
    let mut by_name: BTreeMap<(&str, &str), Vec<&Member>> = BTreeMap::new();
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        by_name
            .entry((m.campaign.as_str(), m.name.as_str()))
            .or_default()
            .push(m);
    }
    for ((camp, name), g) in by_name {
        let mut blocks: Vec<&str> = g.iter().filter_map(|m| m.first_block.as_deref()).collect();
        if blocks.iter().collect::<BTreeSet<_>>().len() < 2 {
            continue;
        }
        let value = g
            .iter()
            .map(|x| {
                format!(
                    "{} [{}, {} features]",
                    x.first_block.as_deref().unwrap_or(""),
                    x.ik_basis,
                    x.n_features
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        blocks.sort_unstable();
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            WARNING,
            camp,
            name,
            &blocks.join("|"),
            "SPLIT: the same name covers different structures (intended homologous series, or name and structure from different sources)".into(),
            Value::Text(value),
        ));
    }
}

/// Four consecutive ASCII lowercase letters (`[a-z]{4}`).
fn has_word(s: &str) -> bool {
    let l = s.to_lowercase();
    let mut run = 0;
    for c in l.chars() {
        run = if c.is_ascii_lowercase() { run + 1 } else { 0 };
        if run >= 4 {
            return true;
        }
    }
    false
}

/// (c) a name that is not among the PubChem synonyms of its structure.
fn names_vs_synonyms(ctx: &Context, out: &mut Vec<Finding>) {
    let mut syn: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for p in &ctx.pubchem {
        let mut s: BTreeSet<String> = p.synonyms.split('|').map(norm_name).collect();
        s.insert(norm_name(&p.title));
        syn.insert(p.cid.as_str(), s);
    }
    let mut ev_names: HashMap<(&str, &str), BTreeSet<&str>> = HashMap::new();
    for e in &ctx.evidence {
        if let Some(n) = e.source_name.as_deref() {
            ev_names
                .entry((e.campaign.as_str(), e.identity.as_str()))
                .or_default()
                .insert(n);
        }
    }
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        let Some(c) = ctx.compound(m.first_block.as_deref()) else {
            continue;
        };
        if c.pubchem_cid.is_empty() || m.ik_basis.starts_with("PubChem name") {
            continue;
        }
        let Some(s) = syn.get(c.pubchem_cid.as_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let mut cands = vec![m.name.as_str()];
        cands.extend(
            ev_names
                .get(&(m.campaign.as_str(), m.name.as_str()))
                .into_iter()
                .flatten()
                .copied(),
        );
        if cands
            .iter()
            .any(|x| !norm_name(x).is_empty() && s.contains(&norm_name(x)))
        {
            continue;
        }
        if !has_word(&m.name) || m.group == "ambiguous" {
            continue;
        }
        let detail = format!(
            "name not among the PubChem synonyms of CID {} ('{}') [structure from: {}]",
            c.pubchem_cid, c.pubchem_title, m.ik_basis
        );
        let sev = if m.group == "contaminant" {
            WARNING
        } else {
            INFO
        };
        let value = cands
            .iter()
            .skip(1)
            .take(3)
            .copied()
            .collect::<Vec<_>>()
            .join("; ");
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            sev,
            &m.campaign,
            &m.name,
            m.first_block.as_deref().unwrap_or(""),
            detail,
            Value::Text(value),
        ));
    }
}

/// (d) the structure from the name source against the loader's skeleton.
fn name_vs_loader(ctx: &Context, out: &mut Vec<Finding>) {
    let mut groups: BTreeMap<(&str, &str), Vec<&Evidence>> = BTreeMap::new();
    for e in &ctx.evidence {
        if let (Some(n), Some(l)) = (&e.name_block, &e.loader_block) {
            if n != l {
                groups
                    .entry((e.campaign.as_str(), e.identity.as_str()))
                    .or_default()
                    .push(e);
            }
        }
    }
    for ((camp, name), g) in groups {
        let first = g[0];
        let loaders: BTreeSet<&str> = g.iter().filter_map(|e| e.loader_block.as_deref()).collect();
        let detail = format!(
            "the structure from the name source ({}) differs from the loader's inchikey_skeleton ({}) in {} features ('problem A'): the registry uses the one from the name source",
            first.name_source,
            first.loader_source,
            g.len()
        );
        let value = loaders.into_iter().collect::<Vec<_>>().join("|");
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            INFO,
            camp,
            name,
            first.name_block.as_deref().unwrap_or(""),
            detail,
            Value::Text(value),
        ));
    }
}

/// (e) mzMine library entries whose structure contradicts their own precursor.
fn mzlib_bugs(ctx: &Context, out: &mut Vec<Finding>) {
    for (camp, rows) in &ctx.mzlib {
        let mut by_key: BTreeMap<&str, Vec<&MzlibCheck>> = BTreeMap::new();
        for r in rows.iter().filter(|r| r.structure_ok == Some(false)) {
            if let Some(k) = r.inchi_key.as_deref().filter(|k| k.chars().count() >= 14) {
                by_key.entry(k).or_default().push(r);
            }
        }
        for (k, g) in by_key {
            let names: BTreeSet<&str> = g.iter().map(|r| r.name.as_str()).collect();
            let identity = names.into_iter().take(6).collect::<Vec<_>>().join(" | ");
            let abs: Vec<f64> = g.iter().map(|r| r.structure_ppm.abs()).collect();
            let med = crate::attributio::rt_logp::median(&abs);
            let detail = format!(
                "mzMine library bug: {} entries ({}) carry InChIKey {k} ({}) inconsistent with their precursor (median {med:.0} ppm): structure ignored",
                g.len(),
                g[0].pol,
                g[0].mol_formula
            );
            let block: String = k.chars().take(14).collect();
            out.push(finding(
                CHECK_NAME_INCHIKEY,
                INFO,
                camp,
                &identity,
                &block,
                detail,
                Value::Int(g.len() as i64),
            ));
        }
    }
}

/// One structure under several names, one name over several structures, a name outside the PubChem synonyms, the name source against the loader, and the
/// library entries that contradict their own precursor.
pub fn check_names(ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    names_of_one_structure(ctx, &mut out);
    structures_of_one_name(ctx, &mut out);
    names_vs_synonyms(ctx, &mut out);
    name_vs_loader(ctx, &mut out);
    mzlib_bugs(ctx, &mut out);
    out
}
