//! `normative build`: reads every list of `sources.json`, resolves the entries to structures and writes `lists/<ID>.csv`, `provenance.json` and
//! `unresolved.tsv`. Nothing is written until every input is read and every pin is checked; a list whose source fails keeps its last good CSV and is marked
//! `stale`, and a list is never emptied silently.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::build_out::write_outputs;
use super::build_rows::{build_list, Built};
use super::entry::{cas_tokens, Entry};
use super::fetch::{Cache, Fetch};
use super::readers::{self, Ctx, Input};
use super::resolve::{Plan, Resolver};
use super::sources::{ListSrc, Sources};
use super::NormError;

pub const PROV_SCHEMA: &str = "normative-prov/1";

pub struct Opts {
    pub sources: PathBuf,
    pub overrides: PathBuf,
    pub out: PathBuf,
    pub only: Vec<String>,
    pub update_pins: bool,
    pub resolve: bool,
}

pub struct Failed {
    pub id: String,
    pub why: String,
}

/// What a run did, one line per list.
pub struct Report {
    pub lines: Vec<String>,
    pub stale: usize,
}

type Read = Vec<(usize, Vec<Entry>, Vec<Input>)>;

fn read_all<F: Fetch>(
    cache: &Cache<F>,
    lists: &[&ListSrc],
) -> Result<(Read, Vec<Failed>), NormError> {
    let (mut ok, mut failed) = (Vec::new(), Vec::new());
    for (i, l) in lists.iter().enumerate() {
        let mut ctx = Ctx {
            loader: cache,
            inputs: Vec::new(),
        };
        match readers::read(&mut ctx, l) {
            Ok(e) => ok.push((i, e, ctx.inputs)),
            Err(e) if e.key == "err.normative.large" => return Err(e),
            Err(e) => failed.push(Failed {
                id: l.id.clone(),
                why: e.to_string(),
            }),
        }
    }
    Ok((ok, failed))
}

/// The resolver loaded for the CAS numbers and DTXSIDs of what was read, with the inputs it used.
fn make_resolver<F: Fetch>(
    cache: &Cache<F>,
    o: &Opts,
    sources: &Sources,
    read: &Read,
    inputs: &mut Vec<Input>,
) -> Result<Resolver, NormError> {
    let mut cas: BTreeSet<String> = BTreeSet::new();
    let mut dtx: BTreeSet<String> = BTreeSet::new();
    for (_, es, _) in read {
        for e in es {
            cas.extend(cas_tokens(&e.cas));
            if !e.dtxsid.is_empty() {
                dtx.insert(e.dtxsid.clone());
            }
        }
    }
    if !o.resolve || read.is_empty() {
        return Ok(Resolver::default());
    }
    let curated = std::fs::read_to_string(&o.overrides).unwrap_or_default();
    let plan = Plan {
        susdat: sources.resolver_url("susdat"),
        pubchem_ids: sources.resolver_url("pubchem_ids"),
        pubchem_parent: sources.resolver_url("pubchem_parent"),
        pubchem_props: sources.resolver_url("pubchem_props"),
        dsstox: sources.resolver_url("dsstox"),
        cas: &cas,
        dtxsid: &dtx,
    };
    Resolver::load(cache, &plan, &curated, inputs)
}

/// Every input whose URL is pinned must hash to its pin; `--update-pins` writes what was read instead.
fn check_pins(
    o: &Opts,
    sources: &mut Sources,
    read: &Read,
    resolver_inputs: &[Input],
) -> Result<(), NormError> {
    let pins = sources.pins();
    let mut seen: Vec<(String, String)> = Vec::new();
    for x in read
        .iter()
        .flat_map(|(_, _, i)| i.iter())
        .chain(resolver_inputs.iter())
    {
        if !seen.iter().any(|(u, _)| *u == x.url) {
            seen.push((x.url.clone(), x.sha256.clone()));
        }
    }
    let mut bad = Vec::new();
    for (url, sha) in &seen {
        if let Some((_, Some(pin))) = pins.iter().find(|(u, _)| u == url) {
            if pin != sha {
                bad.push(format!("{url}: pinned {pin}, read {sha}"));
            }
        }
    }
    if !bad.is_empty() && !o.update_pins {
        return Err(NormError::pin(&bad.join("; ")));
    }
    if o.update_pins {
        let known: Vec<(String, String)> = seen
            .iter()
            .filter(|(u, _)| pins.iter().any(|(p, _)| p == u))
            .cloned()
            .collect();
        sources.set_pins(&known);
        std::fs::write(&o.sources, sources.to_text()).map_err(|e| NormError::io(&o.sources, &e))?;
    }
    Ok(())
}

pub fn run<F: Fetch>(cache: &Cache<F>, o: &Opts) -> Result<Report, NormError> {
    let mut sources = Sources::load(&o.sources)?;
    let all = sources.lists()?;
    for id in &o.only {
        if !all.iter().any(|l| &l.id == id) {
            return Err(NormError::usage(&format!(
                "--only {id}: not in {}",
                o.sources.display()
            )));
        }
    }
    let chosen: Vec<&ListSrc> = all
        .iter()
        .filter(|l| o.only.is_empty() || o.only.contains(&l.id))
        .collect();
    let (read, failed) = read_all(cache, &chosen)?;

    let mut resolver_inputs: Vec<Input> = Vec::new();
    let resolver = make_resolver(cache, o, &sources, &read, &mut resolver_inputs)?;
    check_pins(o, &mut sources, &read, &resolver_inputs)?;

    let built: Vec<Built> = read
        .into_iter()
        .map(|(i, es, inputs)| build_list(chosen[i], &es, inputs, &resolver, o.resolve))
        .collect();
    write_outputs(o, &all, &built, &failed, &resolver_inputs)?;
    let mut lines: Vec<String> = built
        .iter()
        .map(|b| {
            let c = b.coverage;
            format!(
                "{} entries={} screenable={} groups={} uvcb={} inorganic={} unresolved={}",
                b.id, c.entries, c.screenable, c.groups, c.uvcb, c.inorganic, c.unresolved
            )
        })
        .collect();
    lines.extend(failed.iter().map(|f| format!("{} stale: {}", f.id, f.why)));
    Ok(Report {
        lines,
        stale: failed.len(),
    })
}
