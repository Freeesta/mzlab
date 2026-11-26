//! `mzlab-cli spectrum list`: the scans of a file as `scanlist/1` or CSV, the answer of the page's `api/scanlist` (the number tokens are the route's own).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

use super::{finish, one_file, query};
use crate::extract::{self, Sample};
use crate::Outcome;

pub const HELP_LIST: &str = "mzlab-cli spectrum list: the scans of a file as JSON (scanlist/1) or CSV, the answer of the page's api/scanlist.

usage: mzlab-cli spectrum list FILE [--ms-level N] [--id CHANNEL] [--sample ID] [--format json|csv] [--out PATH]

  --ms-level N one level; default every level, in time order
  --id CHANNEL only the scans of an ms:<key> or dd:<level> channel
  --sample ID the sample of a file with several (.wiff); default the first
  --format F  json (default) or csv: scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity
  --out PATH  write there instead of the standard output";

const COLS: [(&str, &str); 7] = [
    ("sid", "scan_id"),
    ("no", "scan_no"),
    ("rt", "rt_min"),
    ("tic", "tic"),
    ("prec", "precursor_mz"),
    ("bpmz", "base_peak_mz"),
    ("bpint", "base_peak_intensity"),
];

/// The rows of `api/scanlist` for the levels asked, in time order: `(rt, [level, sid, no, rt, tic, prec, bpmz, bpint] tokens)`.
fn list_rows(s: &mut Sample, level: Option<u8>, chan: Option<&str>) -> Result<Vec<Vec<String>>> {
    let mut levels: Vec<u8> = match level {
        Some(l) => vec![l],
        None => s.routes.idx.sections.iter().map(|x| x.level).collect(),
    };
    levels.sort_unstable();
    levels.dedup();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for l in levels {
        let mut q = vec![("level", l.to_string())];
        if let Some(c) = chan {
            q.push(("channel", c.into()));
        }
        let text = s
            .routes
            .handle("/api/scanlist", &query(&q))?
            .ok_or_else(|| Error::new("err.scan.none"))?;
        let cols: Vec<Vec<&str>> = COLS
            .iter()
            .map(|(k, _)| extract::tokens(&text, k).unwrap_or_default())
            .collect();
        for i in 0..cols[0].len() {
            let mut r = vec![l.to_string()];
            r.extend(
                cols.iter()
                    .map(|c| c.get(i).copied().unwrap_or("null").to_string()),
            );
            rows.push(r);
        }
    }
    // rt is column 3 (after the level, scan id and scan number)
    rows.sort_by(|a, b| {
        a[3].parse::<f64>()
            .unwrap_or(0.0)
            .total_cmp(&b[3].parse::<f64>().unwrap_or(0.0))
    });
    Ok(rows)
}

fn list_doc(s: &mut Sample, level: Option<u8>, chan: Option<&str>, csv: bool) -> Result<String> {
    let rows = list_rows(s, level, chan)?;
    let cell = |t: &str| {
        if t == "null" {
            String::new()
        } else {
            t.to_string()
        }
    };
    if csv {
        let mut o =
            "scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity\n"
                .to_string();
        for r in &rows {
            let v = [&r[1], &r[2], &r[0], &r[3], &r[4], &r[5], &r[6], &r[7]];
            o += &v.iter().map(|t| cell(t)).collect::<Vec<_>>().join(",");
            o += "\n";
        }
        return Ok(o);
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"scanlist/1\",\"file\":").str(&s.name);
    o.raw(",\"sample\":")
        .raw(&s.sample.map_or("null".to_string(), |i| i.to_string()));
    o.raw(",\"x_unit\":\"min\",\"n_scans\":")
        .int(rows.len() as i64)
        .raw(",\"scans\":[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        o.raw(&format!(
            "{{\"scan_id\":{},\"scan_no\":{},\"ms_level\":{},\"rt_min\":{},\"tic\":{},\"precursor_mz\":{},\"base_peak_mz\":{},\"base_peak_intensity\":{}}}",
            r[1], r[2], r[0], r[3], r[4], r[5], r[6], r[7]
        ));
    }
    o.raw("],\"evidence\":{\"route\":\"/api/scanlist\",\"levels\":");
    let levels: Vec<String> = {
        let mut l: Vec<String> = rows.iter().map(|r| r[0].clone()).collect();
        l.dedup();
        l
    };
    o.raw(&format!("[{}]", levels.join(",")));
    o.raw("}}");
    Ok(o.0)
}

pub fn list(args: Vec<String>) -> Outcome {
    let go = || -> std::result::Result<_, String> {
        let Some(p) = extract::parse(
            args,
            &["--ms-level", "--id", "--sample", "--format", "--out"],
            &[],
        )?
        else {
            return Ok(Err(HELP_LIST));
        };
        let file = one_file(&p, "spectrum list")?;
        let level = p.get("--ms-level").map(|_| p.level()).transpose()?;
        Ok(Ok((
            p.sample(&file)?,
            p.csv()?,
            p.out(),
            file,
            level,
            p.get("--id").map(String::from),
        )))
    };
    let (sample, csv, out, file, level, chan) = match go() {
        Ok(Ok(a)) => a,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    finish(
        extract::with_tmp("scanlist", |tmp| {
            let mut s = extract::open_sample(&file, sample, tmp)?;
            Ok((
                list_doc(&mut s, level, chan.as_deref(), csv)?,
                s.note.clone(),
            ))
        }),
        &out,
    )
}
