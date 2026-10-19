// Ported from Attributio chem/registry.py (load_registry, load_meta), reglists/schema.py (read_membership), ecotox/hazard.py (snapshot).
//! Typed readers of the versioned snapshots: the compound registry, the list membership table, the ECOTOX values and the ECOSAR endpoints. Each takes the text
//! of the file (separator «;», every cell kept as text, numbers read the way `pd.to_numeric(errors="coerce")` does); the caller decides where the text comes from.

use super::vocab::canon;
use crate::attributio::error::Result;
use crate::attributio::frame::{to_num, Frame};

/// Columns of `compound_registry.csv`, in order.
pub const REGISTRY_COLUMNS: [&str; 28] = [
    "inchikey",
    "inchikey_first_block",
    "canonical_name",
    "pubchem_title",
    "formula",
    "monoisotopic_mass",
    "smiles",
    "pubchem_cid",
    "cas",
    "dtxsid",
    "norman_ids",
    "norman_sle",
    "xlogp",
    "xlogp_source",
    "pcl_categories",
    "in_pubchemlite",
    "pcl_annotation_count",
    "roles",
    "project_names",
    "best_level",
    "project_group",
    "project_category",
    "suspect_lists",
    "ik_basis",
    "ghs_aquatic",
    "norman_pred_acute",
    "sources",
    "updated_on",
];

/// Columns of `reglist_membership.csv`, in order.
pub const MEMBERSHIP_COLUMNS: [&str; 18] = [
    "inchikey_skeleton",
    "list_id",
    "name",
    "list_attribute",
    "is_tp",
    "parent",
    "lc_esi_amenable",
    "amenability_rule",
    "formula",
    "mono_mass",
    "smiles",
    "inchikey",
    "list_group",
    "cas",
    "pubchem_cid",
    "xlogp",
    "list_version",
    "resolved_by",
];

/// Membership columns that hold vocabulary words (older files carry Italian ones).
const VOCAB_COLUMNS: [&str; 3] = ["is_tp", "lc_esi_amenable", "resolved_by"];

/// One row of the compound registry: a compound, keyed by the first block of its InChIKey.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegistryRow {
    pub inchikey: String,
    pub first_block: String,
    pub canonical_name: String,
    pub pubchem_title: String,
    pub formula: String,
    pub monoisotopic_mass: Option<f64>,
    pub smiles: String,
    pub pubchem_cid: String,
    pub cas: String,
    pub dtxsid: String,
    pub norman_ids: String,
    pub norman_sle: String,
    pub xlogp: Option<f64>,
    pub xlogp_source: String,
    pub pcl_categories: String,
    pub in_pubchemlite: String,
    pub pcl_annotation_count: Option<f64>,
    pub roles: String,
    pub project_names: String,
    pub best_level: Option<f64>,
    pub project_group: String,
    pub project_category: String,
    pub suspect_lists: String,
    pub ik_basis: String,
    pub ghs_aquatic: String,
    pub norman_pred_acute: String,
    pub sources: String,
    pub updated_on: String,
}

/// One row of the list membership table: a list entry, keyed by (list, first block).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MembershipRow {
    pub first_block: String,
    pub list_id: String,
    pub name: String,
    pub list_attribute: String,
    pub is_tp: String,
    pub parent: String,
    pub lc_esi_amenable: String,
    pub amenability_rule: String,
    pub formula: String,
    pub mono_mass: String,
    pub smiles: String,
    pub inchikey: String,
    pub list_group: String,
    pub cas: String,
    pub pubchem_cid: String,
    pub xlogp: String,
    pub list_version: String,
    pub resolved_by: String,
}

/// One aggregated measured value (`ecotox_values.csv`): geometric mean per (CAS, kind, taxon, species).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EcotoxValue {
    pub cas: String,
    pub ik1: String,
    pub kind: String,
    pub taxon: String,
    pub species: String,
    pub endpoint: String,
    pub duration_h: String,
    pub op: String,
    pub value_mg_l: Option<f64>,
    pub n_results: String,
    pub n_refs: String,
    pub media: String,
    pub measurement: String,
    pub result_ids: String,
    pub reference: String,
}

/// One ECOSAR endpoint (`ecosar_endpoints.csv`; the column `ik1` is `key`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EcosarEndpoint {
    pub key: String,
    pub qsar_class: String,
    pub alert: String,
    pub organism: String,
    pub endpoint: String,
    pub duration: String,
    pub value_mg_l: Option<f64>,
    pub kow_cutoff: Option<f64>,
    pub flags: String,
    pub logkow: Option<f64>,
    pub logkow_basis: String,
    pub mw: Option<f64>,
    pub water_solubility: Option<f64>,
    pub in_domain: bool,
    pub excluded_because: String,
}

fn text(f: &Frame, r: usize, name: &str) -> String {
    f.col(name)
        .map_or_else(String::new, |c| f.cell(r, c).to_string())
}

fn number(f: &Frame, r: usize, name: &str) -> Option<f64> {
    f.col(name).and_then(|c| to_num(f.cell(r, c)))
}

/// `compound_registry.csv`. Missing columns read as empty text or missing numbers.
pub fn read_registry(csv: &str) -> Result<Vec<RegistryRow>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| {
            let t = |n: &str| text(&f, r, n);
            let n = |c: &str| number(&f, r, c);
            RegistryRow {
                inchikey: t("inchikey"),
                first_block: t("inchikey_first_block"),
                canonical_name: t("canonical_name"),
                pubchem_title: t("pubchem_title"),
                formula: t("formula"),
                monoisotopic_mass: n("monoisotopic_mass"),
                smiles: t("smiles"),
                pubchem_cid: t("pubchem_cid"),
                cas: t("cas"),
                dtxsid: t("dtxsid"),
                norman_ids: t("norman_ids"),
                norman_sle: t("norman_sle"),
                xlogp: n("xlogp"),
                xlogp_source: t("xlogp_source"),
                pcl_categories: t("pcl_categories"),
                in_pubchemlite: t("in_pubchemlite"),
                pcl_annotation_count: n("pcl_annotation_count"),
                roles: t("roles"),
                project_names: t("project_names"),
                best_level: n("best_level"),
                project_group: t("project_group"),
                project_category: t("project_category"),
                suspect_lists: t("suspect_lists"),
                ik_basis: t("ik_basis"),
                ghs_aquatic: t("ghs_aquatic"),
                norman_pred_acute: t("norman_pred_acute"),
                sources: t("sources"),
                updated_on: t("updated_on"),
            }
        })
        .collect())
}

/// `reglist_membership.csv`: a table older than the current schema lacks the last columns (read as empty); the vocabulary columns go through `canon`.
pub fn read_membership(csv: &str) -> Result<Vec<MembershipRow>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| {
            let t = |n: &str| {
                let v = text(&f, r, n);
                if VOCAB_COLUMNS.contains(&n) {
                    canon(&v)
                } else {
                    v
                }
            };
            MembershipRow {
                first_block: t("inchikey_skeleton"),
                list_id: t("list_id"),
                name: t("name"),
                list_attribute: t("list_attribute"),
                is_tp: t("is_tp"),
                parent: t("parent"),
                lc_esi_amenable: t("lc_esi_amenable"),
                amenability_rule: t("amenability_rule"),
                formula: t("formula"),
                mono_mass: t("mono_mass"),
                smiles: t("smiles"),
                inchikey: t("inchikey"),
                list_group: t("list_group"),
                cas: t("cas"),
                pubchem_cid: t("pubchem_cid"),
                xlogp: t("xlogp"),
                list_version: t("list_version"),
                resolved_by: t("resolved_by"),
            }
        })
        .collect())
}

/// `ecotox_values.csv`.
pub fn read_ecotox_values(csv: &str) -> Result<Vec<EcotoxValue>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| {
            let t = |n: &str| text(&f, r, n);
            EcotoxValue {
                cas: t("cas"),
                ik1: t("ik1"),
                kind: t("kind"),
                taxon: t("taxon"),
                species: t("species"),
                endpoint: t("endpoint"),
                duration_h: t("duration_h"),
                op: t("op"),
                value_mg_l: number(&f, r, "value_mg_L"),
                n_results: t("n_results"),
                n_refs: t("n_refs"),
                media: t("media"),
                measurement: t("measurement"),
                result_ids: t("result_ids"),
                reference: t("reference"),
            }
        })
        .collect())
}

/// `ecosar_endpoints.csv`: `in_domain` is true for the text «true» in any case.
pub fn read_ecosar(csv: &str) -> Result<Vec<EcosarEndpoint>> {
    let f = Frame::parse(csv, ';')?;
    let key = if f.col("ik1").is_some() { "ik1" } else { "key" };
    Ok((0..f.len())
        .map(|r| {
            let t = |n: &str| text(&f, r, n);
            let n = |c: &str| number(&f, r, c);
            EcosarEndpoint {
                key: t(key),
                qsar_class: t("qsar_class"),
                alert: t("alert"),
                organism: t("organism"),
                endpoint: t("endpoint"),
                duration: t("duration"),
                value_mg_l: n("value_mg_L"),
                kow_cutoff: n("kow_cutoff"),
                flags: t("flags"),
                logkow: n("logkow"),
                logkow_basis: t("logkow_basis"),
                mw: n("mw"),
                water_solubility: n("water_solubility"),
                in_domain: t("in_domain").eq_ignore_ascii_case("true"),
                excluded_because: t("excluded_because"),
            }
        })
        .collect())
}

/// `ecotox_curated.csv`: the curated hazard decisions.
pub fn read_curated(csv: &str) -> Result<Vec<super::hazard::Curated>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| {
            let t = |n: &str| text(&f, r, n);
            super::hazard::Curated {
                ik1: t("ik1"),
                label: t("label"),
                acute: t("acute"),
                chronic: t("chronic"),
                basis: t("basis"),
                tier: t("tier"),
                note: t("note"),
            }
        })
        .collect())
}

/// `ecotox_structures.csv`: structures given to identity names by a decision.
pub fn read_overrides(csv: &str) -> Result<Vec<super::hazard::StructureOverride>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| super::hazard::StructureOverride {
            pattern: text(&f, r, "pattern"),
            compound: text(&f, r, "compound"),
            inchikey: text(&f, r, "inchikey"),
        })
        .collect())
}

/// `registry_members.csv`, identity rows only.
pub fn read_identity_members(csv: &str) -> Result<Vec<super::hazard::IdentityMember>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .filter(|r| text(&f, *r, "member_type") == "identity")
        .map(|r| super::hazard::IdentityMember {
            campaign: text(&f, r, "campaign"),
            name: text(&f, r, "name"),
            formula: text(&f, r, "formula"),
            first_block: text(&f, r, "first_block"),
        })
        .collect())
}
