//! `stat/1` as text (keys in English, fixed order) and the JSON input of the three steps (matrices as lists of rows). Numbers only.

use super::{analyse, compare, Ion, KmdOpts, KmdOut, Layout, StatsOpts, StatsOut, SCHEMA};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

fn bad() -> Error {
    Error::new("error.badStatInput")
}

fn matrix_of(v: &Value) -> Result<Vec<Vec<f64>>> {
    v.get("matrix")
        .map(|m| {
            m.arr()
                .iter()
                .map(|r| r.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect())
                .collect()
        })
        .ok_or_else(bad)
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.map(|a| {
        a.arr()
            .iter()
            .map(|x| x.str().unwrap_or("").to_string())
            .collect()
    })
    .unwrap_or_default()
}

fn num_or(v: &Value, key: &str, d: f64) -> f64 {
    v.get(key).and_then(Value::f64).unwrap_or(d)
}

/// The conditions of `{"groups": [name | null per file], "blank": [bool per file], "reference": name | null}`.
fn layout_of(v: &Value) -> Result<Layout> {
    let raw = v.get("groups").ok_or_else(bad)?.arr();
    let mut names: Vec<String> = Vec::new();
    let group_of = raw
        .iter()
        .map(|g| {
            g.str().map(|s| {
                names.iter().position(|n| n == s).unwrap_or_else(|| {
                    names.push(s.to_string());
                    names.len() - 1
                })
            })
        })
        .collect();
    let blank = (0..raw.len())
        .map(|i| {
            v.get("blank")
                .and_then(|b| b.at(i))
                .and_then(Value::bool)
                .unwrap_or(false)
        })
        .collect();
    let reference = v
        .get("reference")
        .and_then(Value::str)
        .and_then(|r| names.iter().position(|n| n == r));
    Ok(Layout {
        names,
        group_of,
        blank,
        reference,
    })
}

/// `{"matrix", "groups", "blank", "reference", "comparisons": [[a, b]] | "vsReference", "pseudo"}` -> `stat/1` of the comparisons.
pub fn stats_json(input: &[u8]) -> Result<String> {
    let v = parse(input)?;
    let (m, lay) = (matrix_of(&v)?, layout_of(&v)?);
    let pos = |n: &str| lay.names.iter().position(|x| x == n);
    let pairs: Vec<(usize, usize)> = match v.get("comparisons") {
        Some(c) if c.str().is_none() => c
            .arr()
            .iter()
            .map(|p| {
                Ok((
                    pos(p.at(0).and_then(Value::str).unwrap_or("")).ok_or_else(bad)?,
                    pos(p.at(1).and_then(Value::str).unwrap_or("")).ok_or_else(bad)?,
                ))
            })
            .collect::<Result<_>>()?,
        _ => {
            let r = lay
                .reference
                .ok_or_else(|| Error::new("error.stats.noReference"))?;
            (0..lay.names.len())
                .filter(|&g| g != r)
                .map(|g| (r, g))
                .collect()
        }
    };
    let opts = StatsOpts {
        pseudo: v.get("pseudo").and_then(Value::f64),
        ..StatsOpts::default()
    };
    Ok(write_stats(&compare(&m, &lay, &pairs, &opts)?, &lay))
}

fn write_rows(o: &mut Out, rows: &[super::Row]) {
    o.raw("[");
    for (i, r) in rows.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"feature\":").int(r.feature as i64);
        for (k, x) in [
            ("meanA", r.mean_a),
            ("meanB", r.mean_b),
            ("log2fc", r.log2fc),
            ("t", r.t),
            ("df", r.df),
            ("p", r.p),
            ("q", r.q),
        ] {
            o.raw(",\"").raw(k).raw("\":").num(x);
        }
        o.raw("}");
    }
    o.raw("]");
}

/// The `stat/1` text of a comparison run: groups, warnings, comparisons with their rows, the ANOVA and the heat map.
pub fn write_stats(s: &StatsOut, lay: &Layout) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"pseudo\":")
        .num(s.pseudo)
        .raw(",\"pseudoSource\":")
        .str(s.pseudo_source);
    o.raw(",\"nFeatures\":")
        .int(s.n_features as i64)
        .raw(",\"groups\":[");
    for (g, name) in lay.names.iter().enumerate() {
        o.raw(if g > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(name)
            .raw(",\"n\":")
            .int(lay.files_of(g).len() as i64)
            .raw("}");
    }
    o.raw("],\"warnings\":[");
    for (i, w) in s.warnings.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"code\":")
            .str(w.code)
            .raw(",\"group\":")
            .int(w.group as i64)
            .raw(",\"n\":")
            .int(w.n as i64)
            .raw("}");
    }
    o.raw("],\"comparisons\":[");
    for (i, c) in s.comparisons.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"a\":")
            .int(c.a as i64)
            .raw(",\"b\":")
            .int(c.b as i64)
            .raw(",\"refused\":");
        match c.refused {
            Some(r) => o.str(r),
            None => o.raw("null"),
        };
        o.raw(",\"rows\":");
        write_rows(&mut o, &c.rows);
        o.raw("}");
    }
    o.raw("],\"anova\":{\"refused\":");
    match s.anova.refused {
        Some(r) => o.str(r),
        None => o.raw("null"),
    };
    o.raw(",\"rows\":[");
    for (i, r) in s.anova.rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"feature\":")
            .int(r.feature as i64);
        for (k, x) in [
            ("f", r.f),
            ("df1", r.df1),
            ("df2", r.df2),
            ("p", r.p),
            ("q", r.q),
        ] {
            o.raw(",\"").raw(k).raw("\":").num(x);
        }
        o.raw("}");
    }
    o.raw("]},\"heat\":{\"groups\":[");
    o.raw(
        &s.heat_groups
            .iter()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );
    o.raw("],\"z\":[");
    for (i, row) in s.heat.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).floats(row);
    }
    o.raw("]}}");
    o.0
}

/// `{"matrix", "roles", "groups", + options}` -> the answer of `api/stat/pca` of `mzlab-core` (`pca_analysis`); an error is the JSON `{error, error_key, params}`.
pub fn pca_json(input: &[u8]) -> std::result::Result<String, String> {
    let body = std::str::from_utf8(input)
        .map_err(|_| "{\"error\":\"the request must be a JSON object\"}".to_string())?;
    mzlab_core::statroute::stat_route("api/stat/pca", body)
        .unwrap_or_else(|| Err("{\"error\":\"no route\"}".to_string()))
}

/// `{"features": [{"mz", "rt"}], "unit", "tolMDa", "minMembers"}` -> the rows and series of the Kendrick analysis.
pub fn kmd_json(input: &[u8]) -> Result<String> {
    let v = parse(input)?;
    let ions: Vec<Ion> = v
        .get("features")
        .ok_or_else(bad)?
        .arr()
        .iter()
        .map(|f| Ion {
            mz: num_or(f, "mz", f64::NAN),
            rt: f.get("rt").and_then(Value::f64),
        })
        .collect();
    let d = KmdOpts::default();
    let id = v.get("unit").and_then(Value::str).unwrap_or("CF2");
    let o = KmdOpts {
        unit: super::units()
            .iter()
            .find(|u| u.id == id)
            .map_or(d.unit, |u| u.id),
        tol_mda: num_or(&v, "tolMDa", d.tol_mda),
        min_members: num_or(&v, "minMembers", d.min_members as f64) as usize,
    };
    Ok(write_kmd(&analyse(&ions, &o)))
}

pub fn write_kmd(r: &KmdOut) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"unit\":{\"id\":")
        .str(r.unit.id)
        .raw(",\"mass\":")
        .num(r.unit.mass)
        .raw(",\"nominal\":")
        .num(r.unit.nominal)
        .raw("},\"rows\":[");
    for (i, x) in r.rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"i\":")
            .int(x.i as i64)
            .raw(",\"mz\":")
            .num(x.mz)
            .raw(",\"rt\":")
            .opt(x.rt);
        for (k, n) in [("km", x.km), ("nkm", x.nkm), ("kmd", x.kmd)] {
            o.raw(",\"").raw(k).raw("\":").num(n);
        }
        o.raw(",\"series\":")
            .opt(x.series.map(|s| s as f64))
            .raw("}");
    }
    o.raw("],\"series\":[");
    for (i, s) in r.series.iter().enumerate() {
        let members: Vec<String> = s.members.iter().map(|m| m.to_string()).collect();
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"n\":")
            .int(s.n as i64)
            .raw(",\"members\":[")
            .raw(&members.join(","))
            .raw("],\"kmd\":")
            .num(s.kmd);
        o.raw(",\"spanMDa\":")
            .num(s.span_mda)
            .raw(",\"steps\":")
            .floats(&s.steps)
            .raw(",\"consecutive\":")
            .num(s.consecutive);
        o.raw(",\"distinct\":")
            .int(s.distinct as i64)
            .raw(",\"rho\":")
            .opt(s.rho)
            .raw("}");
    }
    o.raw("]}");
    o.0
}
