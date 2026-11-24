//! The texts of `quant.targets`: the long table (one row per file and target, flags `;`-joined, `ok` when none), the log
//! (`quant-targets-log/1`: parameters, the offset of every file and polarity with its evidence, the automatic choices, the warnings) and the plan.

use mzlab_core::json::Out;

use super::params::Params;
use super::run::{Choice, FileLog, Row};
use super::targets::{Role, Target};

pub const HEADER: &str = "file,id,area,height,sn,rt,flag,state,points,shape,start_rt,end_rt,peaks_in_window,smooth_half,mz_offset_da";

/// A CSV cell: quoted when it holds a comma, a quote or a line break.
fn cell(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn pol_text(p: Option<i8>) -> &'static str {
    match p {
        Some(1) => "+",
        Some(_) => "-",
        None => "any",
    }
}

/// The log key of the reference choice of a polarity group.
pub(crate) fn pol_key(p: Option<i8>) -> &'static str {
    match p {
        Some(1) => "offset_reference.+",
        Some(_) => "offset_reference.-",
        None => "offset_reference.any",
    }
}

pub(crate) fn table(rows: &[Row], names: &[String], targets: &[Target]) -> String {
    let mut s = String::with_capacity(64 * (rows.len() + 1));
    s.push_str(HEADER);
    s.push('\n');
    for r in rows {
        let flag = if r.flags.is_empty() {
            "ok".to_string()
        } else {
            r.flags.join(";")
        };
        let (nums, points) = match &r.hit {
            Some(h) => (
                [h.area, h.height, h.sn, h.rt]
                    .map(|v| format!("{v}"))
                    .join(","),
                [
                    h.points.to_string(),
                    format!("{}", h.shape),
                    format!("{}", h.start_rt),
                    format!("{}", h.end_rt),
                ]
                .join(","),
            ),
            None => (",,,".to_string(), ",,,".to_string()),
        };
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            cell(&names[r.file]),
            cell(&targets[r.target].id),
            nums,
            flag,
            r.state(),
            points,
            r.peaks,
            r.half,
            r.offset
        ));
    }
    s
}

fn params_json(o: &mut Out, p: &Params) {
    o.raw("{\"da\":").num(p.da);
    o.raw(",\"min_sn\":").num(p.min_sn);
    o.raw(",\"min_points\":").int(p.min_points as i64);
    o.raw(",\"min_shape\":").num(p.min_shape);
    o.raw(",\"offset\":").str(&p.offset_text());
    o.raw(",\"blank_pattern\":").str(&p.blank_pattern);
    o.raw(",\"ms_level\":").int(p.ms_level as i64);
    o.raw(",\"smooth_half\":").int(p.smooth_half as i64);
    o.raw(",\"valley\":").num(p.valley_pct).raw("}");
}

fn strings(o: &mut Out, list: &[String]) {
    o.raw("[");
    for (i, w) in list.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(w);
    }
    o.raw("]");
}

pub(crate) fn log(
    p: &Params,
    files: &[FileLog],
    choices: &[Choice],
    warnings: &[String],
) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"quant-targets-log/1\",\"params\":");
    params_json(&mut o, p);
    o.raw(",\"files\":[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&f.name);
        o.raw(",\"blank\":")
            .raw(if f.blank { "true" } else { "false" });
        o.raw(",\"offsets\":[");
        for (j, g) in f.offsets.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("{\"polarity\":")
                .str(pol_text(g.polarity));
            o.raw(",\"mz_offset_da\":").num(g.offset.da);
            o.raw(",\"reference\":");
            match &g.reference {
                Some(r) => o.str(r),
                None => o.raw("null"),
            };
            o.raw(",\"evidence\":")
                .raw(&g.offset.evidence_json())
                .raw("}");
        }
        o.raw("],\"detected\":").int(f.detected as i64).raw("}");
    }
    o.raw("],\"choices\":[");
    for (i, c) in choices.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(c.key);
        o.raw(",\"value\":")
            .str(&c.value)
            .raw(",\"reason\":")
            .str(&c.reason)
            .raw("}");
    }
    o.raw("],\"warnings\":");
    strings(&mut o, warnings);
    o.raw("}");
    o.0
}

pub(crate) fn plan(p: &Params, names: &[String], targets: &[Target]) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"quant-targets-plan/1\",\"params\":");
    params_json(&mut o, p);
    o.raw(",\"files\":[");
    for (i, n) in names.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"file\":").str(n);
        o.raw(",\"blank\":")
            .raw(if p.is_blank(n) { "true" } else { "false" })
            .raw("}");
    }
    o.raw("],\"targets\":[");
    for (i, t) in targets.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&t.id);
        o.raw(",\"mz\":")
            .num(t.mz)
            .raw(",\"rt_min\":")
            .num(t.rt_min)
            .raw(",\"rt_max\":")
            .num(t.rt_max);
        o.raw(",\"polarity\":").str(pol_text(t.polarity));
        o.raw(",\"role\":")
            .str(if t.role == Role::Reference {
                "reference"
            } else {
                "target"
            })
            .raw("}");
    }
    o.raw("],\"rows\":")
        .int((names.len() * targets.len()) as i64)
        .raw("}");
    o.0
}

/// The rows and the log of a run as `quant-targets/1`: one object per row with the header names as keys, built from the rows themselves (not
/// from the CSV, so a file name or an id is written as it is), numbers written as in the table, empty cells `null`.
pub(crate) fn json(rows: &[Row], names: &[String], targets: &[Target], log: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"quant-targets/1\",\"rows\":[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { ",{\"file\":" } else { "{\"file\":" })
            .str(&names[r.file]);
        o.raw(",\"id\":").str(&targets[r.target].id);
        let h = r.hit.as_ref();
        let nums: [(&str, Option<String>); 4] = [
            ("area", h.map(|h| format!("{}", h.area))),
            ("height", h.map(|h| format!("{}", h.height))),
            ("sn", h.map(|h| format!("{}", h.sn))),
            ("rt", h.map(|h| format!("{}", h.rt))),
        ];
        for (k, v) in &nums {
            o.raw(",")
                .str(k)
                .raw(":")
                .raw(v.as_deref().unwrap_or("null"));
        }
        let flag = if r.flags.is_empty() {
            "ok".to_string()
        } else {
            r.flags.join(";")
        };
        o.raw(",\"flag\":")
            .str(&flag)
            .raw(",\"state\":")
            .str(r.state());
        let rest: [(&str, Option<String>); 4] = [
            ("points", h.map(|h| h.points.to_string())),
            ("shape", h.map(|h| format!("{}", h.shape))),
            ("start_rt", h.map(|h| format!("{}", h.start_rt))),
            ("end_rt", h.map(|h| format!("{}", h.end_rt))),
        ];
        for (k, v) in &rest {
            o.raw(",")
                .str(k)
                .raw(":")
                .raw(v.as_deref().unwrap_or("null"));
        }
        o.raw(",\"peaks_in_window\":").raw(&r.peaks.to_string());
        o.raw(",\"smooth_half\":").raw(&r.half.to_string());
        o.raw(",\"mz_offset_da\":")
            .raw(&format!("{}", r.offset))
            .raw("}");
    }
    o.raw("],\"log\":")
        .raw(if log.is_empty() { "null" } else { log })
        .raw("}");
    o.0
}
