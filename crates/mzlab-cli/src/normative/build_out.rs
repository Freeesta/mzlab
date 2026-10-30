//! `normative build`, the files it writes: `lists/<ID>.csv`, `provenance.json` (`normative-prov/1`) and `unresolved.tsv`. Used by `build.rs`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Out, Value};
use mzlab_feat::attributio::reglists::catalog;

use super::build::{Failed, Opts, PROV_SCHEMA};
use super::build_rows::Built;
use super::readers::Input;
use super::sources::ListSrc;
use super::NormError;

fn input_json(o: &mut Out, x: &Input) {
    o.raw("{\"url\":")
        .str(&x.url)
        .raw(",\"sha256\":")
        .str(&x.sha256)
        .raw(",\"retrieved\":")
        .str(&x.retrieved);
    if x.pages > 1 {
        o.raw(",\"pages\":").int(x.pages as i64);
    }
    o.raw("}");
}

fn list_json(o: &mut Out, src: &ListSrc, b: &Built) {
    o.raw("{\"id\":")
        .str(&src.id)
        .raw(",\"reader\":")
        .str(&src.reader)
        .raw(",\"tags\":{");
    for (i, (k, v)) in src.tags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("},\"inputs\":[");
    for (i, x) in b.inputs.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        input_json(o, x);
    }
    let c = b.coverage;
    o.raw("],\"coverage\":{");
    for (i, (k, v)) in [
        ("entries", c.entries),
        ("screenable", c.screenable),
        ("groups", c.groups),
        ("uvcb", c.uvcb),
        ("inorganic", c.inorganic),
        ("unresolved", c.unresolved),
    ]
    .into_iter()
    .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" })
            .str(k)
            .raw(":")
            .int(v as i64);
    }
    o.raw("},\"csv_sha256\":")
        .str(&sha256_hex(b.csv.as_bytes()))
        .raw(",\"stale\":false}");
}

/// The previous provenance object of a list, read back with `stale` set to `true` (and the error that made it so).
fn stale_json(o: &mut Out, prev: Option<&Value>, id: &str, why: &str) {
    let mut v = prev
        .cloned()
        .unwrap_or_else(|| Value::Obj(vec![("id".into(), Value::Str(id.into()))]));
    if let Value::Obj(m) = &mut v {
        m.retain(|(k, _)| k != "stale" && k != "error");
        m.push(("stale".into(), Value::Bool(true)));
        m.push(("error".into(), Value::Str(why.into())));
    }
    json::write_value(o, &v);
}

fn write_file(p: &PathBuf, t: &str) -> Result<(), NormError> {
    std::fs::write(p, t).map_err(|e| NormError::io(p, &e))
}

/// `provenance.json`: this run's lists, the stale ones from their previous object, the untouched ones as they were.
fn provenance_text(
    all: &[ListSrc],
    built: &[Built],
    failed: &[Failed],
    resolver_inputs: &[Input],
    prev: Option<&Value>,
) -> String {
    let prev_list = |id: &str| {
        prev.and_then(|p| p.get("lists")).and_then(|l| {
            l.arr()
                .iter()
                .find(|x| x.get("id").and_then(Value::str) == Some(id))
        })
    };
    let ids: Vec<&str> = all.iter().map(|l| l.id.as_str()).collect();
    let mut o_json = Out::default();
    o_json
        .raw("{\"schema\":")
        .str(PROV_SCHEMA)
        .raw(",\"lists\":[");
    let mut first = true;
    for id in catalog::ordered(&ids) {
        let src = all.iter().find(|l| l.id == id).expect("listed");
        let mut piece = Out::default();
        if let Some(b) = built.iter().find(|b| b.id == id) {
            list_json(&mut piece, src, b);
        } else if let Some(f) = failed.iter().find(|f| f.id == id) {
            stale_json(&mut piece, prev_list(&id), &id, &f.why);
        } else if let Some(p) = prev_list(&id) {
            json::write_value(&mut piece, p);
        } else {
            continue;
        }
        o_json.raw(if first { "" } else { "," }).raw(&piece.0);
        first = false;
    }
    o_json.raw("],\"resolvers\":[");
    let mut ri: Vec<&Input> = resolver_inputs
        .iter()
        .filter(|x| !x.url.contains("/property/"))
        .collect();
    ri.dedup_by(|a, b| a.url == b.url);
    if resolver_inputs.is_empty() {
        if let Some(Value::Arr(a)) = prev.and_then(|p| p.get("resolvers")).cloned() {
            for (i, v) in a.iter().enumerate() {
                o_json.raw(if i > 0 { "," } else { "" });
                json::write_value(&mut o_json, v);
            }
        }
    }
    for (i, x) in ri.iter().enumerate() {
        o_json.raw(if i > 0 { "," } else { "" });
        input_json(&mut o_json, x);
    }
    let props = resolver_inputs
        .iter()
        .filter(|x| x.url.contains("/property/"))
        .count();
    o_json
        .raw("],\"pubchem_property_requests\":")
        .int(props as i64)
        .raw("}\n");
    o_json.0
}

/// `unresolved.tsv`: this run's lists replace their old rows, the other lists keep theirs.
fn unresolved_text(o: &Opts, built: &[Built]) -> String {
    let rebuilt: BTreeSet<&str> = built.iter().map(|b| b.id.as_str()).collect();
    let mut rows: Vec<[String; 5]> = Vec::new();
    if let Ok(old) = std::fs::read_to_string(o.out.join("unresolved.tsv")) {
        for l in old.lines().skip(1) {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() == 5 && !rebuilt.contains(f[0]) {
                rows.push([
                    f[0].into(),
                    f[1].into(),
                    f[2].into(),
                    f[3].into(),
                    f[4].into(),
                ]);
            }
        }
    }
    rows.extend(built.iter().flat_map(|b| b.unresolved.iter().cloned()));
    let order = |id: &str| catalog::order_of(id).unwrap_or(usize::MAX);
    rows.sort_by(|a, b| {
        (order(&a[0]), &a[0], &a[1], &a[3]).cmp(&(order(&b[0]), &b[0], &b[1], &b[3]))
    });
    std::iter::once("list_id\tentry_ref\tname\tcas\treason\n".to_string())
        .chain(rows.iter().map(|r| {
            format!(
                "{}\n",
                r.iter()
                    .map(|c| c.replace(['\t', '\n'], " "))
                    .collect::<Vec<_>>()
                    .join("\t")
            )
        }))
        .collect()
}

pub fn write_outputs(
    o: &Opts,
    all: &[ListSrc],
    built: &[Built],
    failed: &[Failed],
    resolver_inputs: &[Input],
) -> Result<(), NormError> {
    let lists_dir = o.out.join("lists");
    std::fs::create_dir_all(&lists_dir).map_err(|e| NormError::io(&lists_dir, &e))?;
    for b in built {
        write_file(&lists_dir.join(format!("{}.csv", b.id)), &b.csv)?;
    }
    let prov_path = o.out.join("provenance.json");
    let prev = std::fs::read(&prov_path)
        .ok()
        .and_then(|b| json::parse(&b).ok());
    write_file(
        &prov_path,
        &provenance_text(all, built, failed, resolver_inputs, prev.as_ref()),
    )?;
    write_file(&o.out.join("unresolved.tsv"), &unresolved_text(o, built))
}
