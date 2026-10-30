//! `resolve`: from the identifiers printed in a legal text to the structure a feature can be compared with. The order is the study's §3: `law`, `susdat`,
//! `pubchem`, `dsstox`, `curated`, else `unresolved`. The match key is the first block of the MS-ready InChIKey (`inchikey_skeleton`). A CAS number that
//! leads to two skeletons is `unresolved` with both in the log: a number is an input, never an identity.

use std::collections::{BTreeMap, BTreeSet};

use mzlab_core::elements;
use mzlab_feat::chem::grafo::Mol;

use super::readers::{Input, Loader};
use super::NormError;

impl Resolver {
    pub fn load(
        loader: &dyn Loader,
        plan: &Plan,
        curated_tsv: &str,
        inputs: &mut Vec<Input>,
    ) -> Result<Resolver, NormError> {
        let mut r = Resolver::default();
        r.read_curated(curated_tsv);
        let grab = |url: &str, inputs: &mut Vec<Input>| -> Result<std::path::PathBuf, NormError> {
            let c = loader.ensure(url)?;
            inputs.push(Input {
                url: url.into(),
                sha256: c.sha256.clone(),
                retrieved: c.retrieved.clone(),
                pages: 1,
            });
            Ok(c.path)
        };
        if let Some(u) = &plan.susdat {
            let p = grab(u, inputs)?;
            r.read_susdat(&p, plan)?;
        }
        // PubChem only for the numbers SusDat did not give
        let missing: BTreeSet<String> = plan
            .cas
            .iter()
            .filter(|c| !r.susdat.contains_key(*c))
            .cloned()
            .collect();
        if let (Some(ids), Some(par), Some(props)) =
            (&plan.pubchem_ids, &plan.pubchem_parent, &plan.pubchem_props)
        {
            if !missing.is_empty() {
                let p = grab(ids, inputs)?;
                r.read_ids(&p, &missing)?;
                let want: BTreeSet<String> = r.ids.values().flatten().cloned().collect();
                let p = grab(par, inputs)?;
                r.read_parents(&p, &want)?;
                let mut all: BTreeSet<String> = want.clone();
                all.extend(r.parent.values().cloned());
                r.read_props(loader, props, &all, inputs)?;
            }
        }
        if let Some(u) = &plan.dsstox {
            if !plan.dtxsid.is_empty() {
                let p = grab(u, inputs)?;
                r.read_dsstox(&p, plan.dtxsid)?;
            }
        }
        Ok(r)
    }

    fn from_susdat(&self, recs: &[Sus]) -> Res {
        let mut skel: BTreeMap<String, &Sus> = BTreeMap::new();
        for r in recs {
            let k = if r.ms_key.is_empty() {
                &r.key
            } else {
                &r.ms_key
            };
            let s = skeleton_of(k);
            if !s.is_empty() {
                skel.entry(s).or_insert(r);
            }
        }
        match skel.len() {
            0 => Res::Unresolved("SusDat has the number but no InChIKey".into()),
            1 => {
                let (s, r) = skel.into_iter().next().expect("one");
                let ms_smiles = if r.ms_smiles.is_empty() {
                    &r.smiles
                } else {
                    &r.ms_smiles
                };
                let same = skeleton_of(&r.key) == s;
                let (formula, mass) = match same.then(|| {
                    (
                        hill(&r.formula),
                        r.mass
                            .parse::<f64>()
                            .map(|x| format!("{x:.6}"))
                            .unwrap_or_default(),
                    )
                }) {
                    Some((f, m)) if !f.is_empty() && !m.is_empty() => (f, m),
                    _ => formula_mass(ms_smiles).unwrap_or_default(),
                };
                let form = match form_of(&r.smiles, 0) {
                    "parent" if !same => "salt",
                    f => f,
                };
                Res::Found(Found {
                    inchikey: r.key.clone(),
                    skeleton: s,
                    smiles: ms_smiles.clone(),
                    formula,
                    mono_mass: mass,
                    cid: r.cid.clone(),
                    form,
                    by: "susdat",
                })
            }
            _ => Res::Unresolved(format!(
                "SusDat: two skeletons {}",
                skel.keys().cloned().collect::<Vec<_>>().join(" and ")
            )),
        }
    }

    fn from_pubchem(&self, cas: &str) -> Option<Res> {
        let cids = self.ids.get(cas)?;
        let mut skel: BTreeMap<String, (&String, &String)> = BTreeMap::new();
        for cid in cids {
            let par = self.parent.get(cid).unwrap_or(cid);
            let Some(p) = self.props.get(par) else {
                continue;
            };
            let s = skeleton_of(&p.key);
            if !s.is_empty() {
                skel.entry(s).or_insert((cid, par));
            }
        }
        Some(match skel.len() {
            0 => Res::Unresolved("PubChem has the number but no InChIKey".into()),
            1 => {
                let (s, (cid, par)) = skel.into_iter().next().expect("one");
                let p = &self.props[par];
                let listed = self.props.get(cid).unwrap_or(p);
                let form = if cid == par {
                    "parent"
                } else {
                    form_of(&listed.smiles, listed.charge)
                };
                let form = if form == "parent" && cid != par {
                    "salt"
                } else {
                    form
                };
                Res::Found(Found {
                    inchikey: listed.key.clone(),
                    skeleton: s,
                    smiles: p.smiles.clone(),
                    formula: p.formula.clone(),
                    mono_mass: p.mass.clone(),
                    cid: par.clone(),
                    form,
                    by: "pubchem",
                })
            }
            _ => Res::Unresolved(format!(
                "PubChem: two skeletons {}",
                skel.keys().cloned().collect::<Vec<_>>().join(" and ")
            )),
        })
    }

    /// One CAS number (or none) of an entry through the steps of §3.
    pub fn resolve(
        &self,
        list_id: &str,
        entry_ref: &str,
        cas: &str,
        dtxsid: &str,
        law_key: &str,
    ) -> Res {
        if let Some(f) = law(law_key) {
            return Res::Found(f);
        }
        let mut why: Vec<String> = Vec::new();
        if !cas.is_empty() {
            match self.susdat.get(cas) {
                Some(recs) => match self.from_susdat(recs) {
                    Res::Found(f) => return Res::Found(f),
                    Res::Unresolved(w) => why.push(w),
                },
                None => why.push("not in SusDat".into()),
            }
            match self.from_pubchem(cas) {
                Some(Res::Found(f)) => return Res::Found(f),
                Some(Res::Unresolved(w)) => why.push(w),
                None => why.push("not in PubChem".into()),
            }
        } else if let Some(recs) = self.sus_dtx.get(dtxsid) {
            if let Res::Found(f) = self.from_susdat(recs) {
                return Res::Found(f);
            }
        }
        if let Some(f) = self
            .dss
            .get(dtxsid)
            .filter(|f| f.form != "salt" && !f.skeleton.is_empty())
        {
            return Res::Found(f.clone());
        }
        if let Some(k) =
            self.curated
                .get(&(list_id.to_string(), entry_ref.to_string(), cas.to_string()))
        {
            return Res::Found(Found {
                inchikey: k.clone(),
                skeleton: skeleton_of(k),
                form: "as_listed",
                by: "curated",
                ..Found::default()
            });
        }
        if cas.is_empty() {
            why.insert(0, "no CAS number in the text".into());
        } else if !super::entry::cas_check_ok(cas) {
            why.push("the check digit of the CAS number is wrong (typo in the text?)".into());
        }
        Res::Unresolved(why.join("; "))
    }
}

/// A structure found for one CAS number or DTXSID.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    /// InChIKey of the form the text names.
    pub inchikey: String,
    pub skeleton: String,
    pub smiles: String,
    pub formula: String,
    pub mono_mass: String,
    pub cid: String,
    /// `salt`, `anion`, `parent` or `as_listed`.
    pub form: &'static str,
    pub by: &'static str,
}

pub enum Res {
    Found(Found),
    /// Why not.
    Unresolved(String),
}

#[derive(Clone, Debug, Default)]
pub(super) struct Sus {
    pub(super) key: String,
    pub(super) ms_key: String,
    pub(super) ms_smiles: String,
    pub(super) smiles: String,
    pub(super) formula: String,
    pub(super) mass: String,
    pub(super) cid: String,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Props {
    pub(super) key: String,
    pub(super) formula: String,
    pub(super) mass: String,
    pub(super) smiles: String,
    pub(super) charge: i32,
}

#[derive(Default)]
pub struct Resolver {
    pub(super) susdat: BTreeMap<String, Vec<Sus>>,
    pub(super) sus_dtx: BTreeMap<String, Vec<Sus>>,
    pub(super) ids: BTreeMap<String, BTreeSet<String>>,
    pub(super) parent: BTreeMap<String, String>,
    pub(super) props: BTreeMap<String, Props>,
    pub(super) dss: BTreeMap<String, Found>,
    /// (list_id, entry_ref, cas) -> inchikey and reason, from `overrides.tsv`.
    pub(super) curated: BTreeMap<(String, String, String), String>,
}

/// The sources of the resolvers (URLs from `sources.json`; any may be absent) and what to look for.
pub struct Plan<'a> {
    pub susdat: Option<String>,
    pub pubchem_ids: Option<String>,
    pub pubchem_parent: Option<String>,
    /// URL template with `{cids}`.
    pub pubchem_props: Option<String>,
    pub dsstox: Option<String>,
    pub cas: &'a BTreeSet<String>,
    pub dtxsid: &'a BTreeSet<String>,
}

pub(super) fn skeleton_of(key: &str) -> String {
    if key.len() == 27 && key.as_bytes()[14] == b'-' {
        key[..14].to_string()
    } else {
        String::new()
    }
}

/// `form` from a SMILES: a dot is a salt, a negative charge alone an anion.
pub(super) fn form_of(smiles: &str, charge: i32) -> &'static str {
    if smiles.contains('.') {
        "salt"
    } else if charge < 0 || (smiles.contains("-]") && !smiles.contains("+]")) {
        "anion"
    } else if charge > 0 || smiles.contains("+]") {
        "salt"
    } else {
        "parent"
    }
}

/// Formula in Hill order and mass of a neutral SMILES, by the structure reader of the app.
pub(super) fn formula_mass(smiles: &str) -> Option<(String, String)> {
    let f = Mol::parse(smiles).ok()?.formula();
    Some((elements::fmt(&f), format!("{:.6}", elements::mass(&f))))
}

pub(super) fn hill(raw: &str) -> String {
    elements::parse_formula(raw)
        .map(|f| elements::fmt(&f))
        .unwrap_or_else(|_| raw.to_string())
}

/// A structure given by the law itself: a full InChIKey in the entry.
fn law(key: &str) -> Option<Found> {
    let s = skeleton_of(key);
    (!s.is_empty()).then(|| Found {
        inchikey: key.into(),
        skeleton: s,
        form: "as_listed",
        by: "law",
        ..Found::default()
    })
}
