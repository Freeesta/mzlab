//! The pieces of the `kinetics-series/1` text that only write: the `uv` object and the whole document. The choices are made in `kinetics.rs`; nothing here decides.

use mzlab_core::json::Value;
use mzlab_core::jsonb::{opt, text, write, Obj};
use mzlab_core::stats::wspearman;

use super::kinetics::{Choice, File, Pt};
use super::kinetics_read::Set;

pub(super) fn uv_json(set: &Set, uv: Option<(f64, f64)>, files: &[File]) -> Value {
    let Some((rt, reference)) = uv else {
        return Obj::new().b("wired", false).done();
    };
    let used: Vec<&File> = files
        .iter()
        .filter(|f| f.uv.is_some() && f.out.is_none())
        .collect();
    let (t, a): (Vec<f64>, Vec<f64>) = used
        .iter()
        .filter_map(|f| f.time.zip(f.uv.map(|u| u.1)))
        .unzip();
    let rho = (t.len() >= 4).then(|| wspearman(&t, &a, None));
    Obj::new()
        .b("wired", true)
        .put(
            "mode",
            text(if set.normalise { "normalise" } else { "check" }),
        )
        .n("rt_min", rt)
        .n("rt_tol_min", set.uv_rt_tol)
        .n("tol_pct", set.uv_tol)
        .n("reference_area", reference)
        .put("time_spearman", opt(rho))
        .done()
}

fn strs(v: &[&str]) -> Value {
    Value::Arr(v.iter().map(|s| text(s)).collect())
}

fn params_json(set: &Set) -> Value {
    Obj::new()
        .put(
            "ids",
            strs(&set.ids.iter().map(String::as_str).collect::<Vec<_>>()),
        )
        .b("use_t0", set.use_t0)
        .put(
            "missing",
            text(if set.zero_missing { "zero" } else { "skip" }),
        )
        .put(
            "uv_mode",
            text(if set.normalise { "normalise" } else { "check" }),
        )
        .n("uv_rt", set.uv_rt)
        .n("uv_rt_tol", set.uv_rt_tol)
        .n("uv_tol", set.uv_tol)
        .i("boot", set.boot as i64)
        .i("seed", set.seed as i64)
        .done()
}

pub(super) fn document(
    set: &Set,
    files: &[File],
    pts: &[Pt],
    fits: Vec<Value>,
    log: &[Choice],
    uv: Value,
    warnings: &[String],
) -> String {
    let params = params_json(set);
    let file_rows = files.iter().map(|f| {
        let flag = f
            .factor
            .is_some_and(|v| (v - 1.0).abs() * 100.0 > set.uv_tol);
        Obj::new()
            .put("file", text(&f.name))
            .put("kind", text(&f.kind))
            .put("series", text(&f.series))
            .put("time_min", opt(f.time))
            .put("uv_rt_min", opt(f.uv.map(|u| u.0)))
            .put("uv_area", opt(f.uv.map(|u| u.1)))
            .put("factor", opt(f.factor))
            .b("uv_off", flag)
            .b("used", f.out.is_none())
            .put("reason", f.out.map_or(Value::Null, text))
            .done()
    });
    let point_rows = pts.iter().map(|p| {
        let factor = files[p.file].factor;
        Obj::new()
            .put("id", text(&p.id))
            .put("series", text(&p.series))
            .put("file", text(&files[p.file].name))
            .put("time_min", opt(files[p.file].time))
            .put("area", opt(p.area))
            .put("factor", opt(factor))
            .put("value", opt(p.y))
            .put("state", text(&p.state))
            .b("used", p.y.is_some())
            .put("reason", p.out.map_or(Value::Null, text))
            .done()
    });
    let choices = log.iter().map(|c| {
        Obj::new()
            .put("key", text(&c.0))
            .put("file", text(&c.1))
            .put("value", text(&c.2))
            .put("reason", text(&c.3))
            .done()
    });
    let o = Obj::new()
        .put("format", text("kinetics-series/1"))
        .put("params", params)
        .put("uv", uv)
        .put("files", Value::Arr(file_rows.collect()))
        .put("points", Value::Arr(point_rows.collect()))
        .put("fits", Value::Arr(fits))
        .put("choices", Value::Arr(choices.collect()))
        .put(
            "warnings",
            Value::Arr(warnings.iter().map(|w| text(w)).collect()),
        )
        .done();
    write(&o) + "\n"
}
