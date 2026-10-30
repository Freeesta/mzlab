//! The loaders of `resolve`: they read the SusDat CSV, the PubChem identifier, parent and property files, the DSSTox dump and `overrides.tsv` into the
//! maps of `Resolver`, keeping only what the lists ask for.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};

use super::csvrd::{at, col, Records};
use super::entry::cas_tokens;
use super::fetch::open_file;
use super::readers::{Input, Loader};
use super::resolve::{form_of, skeleton_of, Found, Plan, Props, Resolver, Sus};
use super::NormError;

impl Resolver {
    pub(super) fn read_curated(&mut self, tsv: &str) {
        for l in tsv.lines().skip(1) {
            let f: Vec<&str> = l.split(';').collect();
            if f.len() >= 4 && !f[0].is_empty() && f[3].len() == 27 {
                self.curated
                    .insert((f[0].into(), f[1].into(), f[2].into()), f[3].into());
            }
        }
    }

    pub(super) fn read_susdat(
        &mut self,
        path: &std::path::Path,
        plan: &Plan,
    ) -> Result<(), NormError> {
        let mut it = Records::new(open_file(path)?, b',');
        let h = it.next().unwrap_or_default();
        let c = |n: &str| col(&h, &[n]);
        let cas_cols = [c("CAS_RN"), c("CAS_RN_Dashboard"), c("CAS_RN_PubChem")];
        let (key, msk, mss, smi, frm, mass, cid, dtx) = (
            c("StdInChIKey"),
            c("MS_Ready_StdInChIKey"),
            c("MS_Ready_SMILES"),
            c("SMILES"),
            c("Molecular_Formula"),
            c("Monoiso_Mass"),
            c("PubChem_CID"),
            c("DTXSID"),
        );
        if key.is_none() || cas_cols.iter().all(Option::is_none) {
            return Err(NormError::parse(
                "susdat",
                "the SusDat file lacks CAS_RN or StdInChIKey",
            ));
        }
        for rec in it {
            let mut tokens: Vec<String> = Vec::new();
            for cc in cas_cols {
                tokens.extend(cas_tokens(at(&rec, cc)));
            }
            let tokens: Vec<String> = tokens
                .into_iter()
                .filter(|t| plan.cas.contains(t))
                .collect();
            let d = at(&rec, dtx).to_string();
            let by_dtx = !d.is_empty() && plan.dtxsid.contains(&d);
            if tokens.is_empty() && !by_dtx {
                continue;
            }
            let s = Sus {
                key: at(&rec, key).into(),
                ms_key: at(&rec, msk).into(),
                ms_smiles: at(&rec, mss).into(),
                smiles: at(&rec, smi).into(),
                formula: at(&rec, frm).into(),
                mass: at(&rec, mass).into(),
                cid: at(&rec, cid).split(';').next().unwrap_or("").into(),
            };
            for t in tokens {
                self.susdat.entry(t).or_default().push(s.clone());
            }
            if by_dtx {
                self.sus_dtx.entry(d).or_default().push(s);
            }
        }
        Ok(())
    }

    /// `CID-Identifiers.tsv.gz`: `CID <tab> identifier <tab> type`, streamed; only the wanted CAS numbers are kept.
    pub(super) fn read_ids(
        &mut self,
        path: &std::path::Path,
        cas: &BTreeSet<String>,
    ) -> Result<(), NormError> {
        for line in BufReader::new(open_file(path)?)
            .lines()
            .map_while(Result::ok)
        {
            let mut f = line.split('\t');
            let (Some(cid), Some(id), Some(ty)) = (f.next(), f.next(), f.next()) else {
                continue;
            };
            if ty == "CAS" && cas.contains(id) {
                self.ids
                    .entry(id.to_string())
                    .or_default()
                    .insert(cid.to_string());
            }
        }
        Ok(())
    }

    /// `CID-Parent.gz`: `CID <tab> parent CID`.
    pub(super) fn read_parents(
        &mut self,
        path: &std::path::Path,
        want: &BTreeSet<String>,
    ) -> Result<(), NormError> {
        for line in BufReader::new(open_file(path)?)
            .lines()
            .map_while(Result::ok)
        {
            if let Some((cid, par)) = line.split_once('\t') {
                if want.contains(cid) {
                    self.parent.insert(cid.to_string(), par.trim().to_string());
                }
            }
        }
        Ok(())
    }

    /// PUG REST properties of the CIDs, in sorted batches of 100, through the cache.
    pub(super) fn read_props(
        &mut self,
        loader: &dyn Loader,
        tpl: &str,
        cids: &BTreeSet<String>,
        inputs: &mut Vec<Input>,
    ) -> Result<(), NormError> {
        let all: Vec<&String> = cids.iter().collect();
        for chunk in all.chunks(100) {
            let url = tpl.replace(
                "{cids}",
                &chunk
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
            let c = loader.ensure(&url)?;
            let b = loader.bytes(&c)?;
            inputs.push(Input {
                url,
                sha256: c.sha256,
                retrieved: c.retrieved,
                pages: 1,
            });
            let mut it = Records::new(b.as_slice(), b',');
            let h = it.next().unwrap_or_default();
            let (ci, k, f, m, s, ch) = (
                col(&h, &["CID"]),
                col(&h, &["InChIKey"]),
                col(&h, &["MolecularFormula"]),
                col(&h, &["MonoisotopicMass", "ExactMass"]),
                col(&h, &["SMILES", "IsomericSMILES", "CanonicalSMILES"]),
                col(&h, &["Charge"]),
            );
            for rec in it {
                let mass = at(&rec, m)
                    .parse::<f64>()
                    .map(|x| format!("{x:.6}"))
                    .unwrap_or_default();
                self.props.insert(
                    at(&rec, ci).to_string(),
                    Props {
                        key: at(&rec, k).into(),
                        formula: at(&rec, f).into(),
                        mass,
                        smiles: at(&rec, s).into(),
                        charge: at(&rec, ch).parse().unwrap_or(0),
                    },
                );
            }
        }
        Ok(())
    }

    /// The DSSTox dump: `DTXSID`, `INCHIKEY`, `SMILES`, `MOLECULAR_FORMULA`, `MONOISOTOPIC_MASS`.
    pub(super) fn read_dsstox(
        &mut self,
        path: &std::path::Path,
        want: &BTreeSet<String>,
    ) -> Result<(), NormError> {
        let mut it = Records::new(open_file(path)?, b',');
        let h = it.next().unwrap_or_default();
        let c = |n: &[&str]| col(&h, n);
        let (d, k, s, f, m) = (
            c(&["DTXSID"]),
            c(&["INCHIKEY", "InChIKey"]),
            c(&["SMILES"]),
            c(&["MOLECULAR_FORMULA"]),
            c(&["MONOISOTOPIC_MASS"]),
        );
        for rec in it {
            let id = at(&rec, d);
            if want.contains(id) {
                let smiles = at(&rec, s).to_string();
                let mass = at(&rec, m)
                    .parse::<f64>()
                    .map(|x| format!("{x:.6}"))
                    .unwrap_or_default();
                let key = at(&rec, k).to_string();
                self.dss.insert(
                    id.into(),
                    Found {
                        skeleton: skeleton_of(&key),
                        inchikey: key,
                        form: form_of(&smiles, 0),
                        smiles,
                        formula: at(&rec, f).into(),
                        mono_mass: mass,
                        cid: String::new(),
                        by: "dsstox",
                    },
                );
            }
        }
        Ok(())
    }
}
