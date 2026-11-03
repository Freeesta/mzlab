//! The sources of the resolvers of `normative build` (URLs from `sources.json`) and their loading into the `Resolver` of `mzlab_feat::normative::resolve`.

use std::collections::BTreeSet;

pub use mzlab_feat::normative::resolve::{Found, Res, Resolver};

use super::readers::{Input, Loader};
use super::resolve_src::{
    read_curated, read_dsstox, read_ids, read_parents, read_props, read_susdat,
};
use super::NormError;

pub fn load(
    loader: &dyn Loader,
    plan: &Plan,
    curated_tsv: &str,
    inputs: &mut Vec<Input>,
) -> Result<Resolver, NormError> {
    let mut r = Resolver::default();
    read_curated(&mut r, curated_tsv);
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
        read_susdat(&mut r, &p, plan)?;
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
        && !missing.is_empty()
    {
        let p = grab(ids, inputs)?;
        read_ids(&mut r, &p, &missing)?;
        let want: BTreeSet<String> = r.ids.values().flatten().cloned().collect();
        let p = grab(par, inputs)?;
        read_parents(&mut r, &p, &want)?;
        let mut all: BTreeSet<String> = want.clone();
        all.extend(r.parent.values().cloned());
        read_props(&mut r, loader, props, &all, inputs)?;
    }
    if let Some(u) = &plan.dsstox
        && !plan.dtxsid.is_empty()
    {
        let p = grab(u, inputs)?;
        read_dsstox(&mut r, &p, plan.dtxsid)?;
    }
    Ok(r)
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
