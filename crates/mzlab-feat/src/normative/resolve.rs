//! `resolve`: from the identifiers printed in a legal text to the structure a feature can be compared with. The order is the study's §3: `law`, `susdat`,
//! `pubchem`, `dsstox`, `curated`, else `unresolved`. The match key is the first block of the MS-ready InChIKey (`inchikey_skeleton`). A CAS number that
//! leads to two skeletons is `unresolved` with both in the log: a number is an input, never an identity. The loading of the sources stays in `mzlab-cli`
//! (`normative build`); the browser and the app fill `susdat` from the CAS sidecar (`sidecar.rs`).

use std::collections::{BTreeMap, BTreeSet};

use crate::chem::grafo::Mol;
use mzlab_core::elements;

impl Resolver {
    fn pick_susdat(&self, recs: &[Sus]) -> Res {
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

    fn pick_pubchem(&self, cas: &str) -> Option<Res> {
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
                Some(recs) => match self.pick_susdat(recs) {
                    Res::Found(f) => return Res::Found(f),
                    Res::Unresolved(w) => why.push(w),
                },
                None => why.push("not in SusDat".into()),
            }
            match self.pick_pubchem(cas) {
                Some(Res::Found(f)) => return Res::Found(f),
                Some(Res::Unresolved(w)) => why.push(w),
                None => why.push("not in PubChem".into()),
            }
        } else if let Some(recs) = self.sus_dtx.get(dtxsid)
            && let Res::Found(f) = self.pick_susdat(recs)
        {
            return Res::Found(f);
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
pub struct Sus {
    pub key: String,
    pub ms_key: String,
    pub ms_smiles: String,
    pub smiles: String,
    pub formula: String,
    pub mass: String,
    pub cid: String,
}

#[derive(Clone, Debug, Default)]
pub struct Props {
    pub key: String,
    pub formula: String,
    pub mass: String,
    pub smiles: String,
    pub charge: i32,
}

#[derive(Default)]
pub struct Resolver {
    pub susdat: BTreeMap<String, Vec<Sus>>,
    pub sus_dtx: BTreeMap<String, Vec<Sus>>,
    pub ids: BTreeMap<String, BTreeSet<String>>,
    pub parent: BTreeMap<String, String>,
    pub props: BTreeMap<String, Props>,
    pub dss: BTreeMap<String, Found>,
    /// (list_id, entry_ref, cas) -> inchikey and reason, from `overrides.tsv`.
    pub curated: BTreeMap<(String, String, String), String>,
}

pub fn skeleton_of(key: &str) -> String {
    if key.len() == 27 && key.as_bytes()[14] == b'-' {
        key[..14].to_string()
    } else {
        String::new()
    }
}

/// `form` from a SMILES: a dot is a salt, a negative charge alone an anion.
pub fn form_of(smiles: &str, charge: i32) -> &'static str {
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
pub fn formula_mass(smiles: &str) -> Option<(String, String)> {
    let f = Mol::parse(smiles).ok()?.formula();
    Some((elements::fmt(&f), format!("{:.6}", elements::mass(&f))))
}

pub fn hill(raw: &str) -> String {
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
