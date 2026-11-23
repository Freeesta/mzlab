//! `mzlab-cli sample table`: the samples/1 the names propose, the roles and times it chose logged on stderr (`auto:` lines) and the plain `--text` table.

use mzlab_core::json::{parse, Value};

use crate::Outcome;

/// The `auto:` lines of the roles and times the names decided: field, value and the part of the name (`propose_explained`), one line per file.
fn decisions(body: &str) -> String {
    let Ok(doc) = mzlab_core::samples::route_all("propose_explained", body)
        .map_err(|e| e.key)
        .and_then(|j| parse(j.as_bytes()).map_err(|e| e.key))
    else {
        return String::new();
    };
    let (rows, spans) = (
        doc.get("samples")
            .and_then(|s| s.get("samples"))
            .map_or(&[][..], Value::arr),
        doc.get("positions").map_or(&[][..], Value::arr),
    );
    let mut out = String::new();
    for (row, pos) in rows.iter().zip(spans) {
        let name = row.get("file").and_then(Value::str).unwrap_or("");
        let chars: Vec<char> = name.chars().collect();
        let mut line = format!("auto: {name}");
        for field in ["kind", "blank_kind", "time"] {
            let Some(p) = pos
                .arr()
                .iter()
                .find(|p| p.get("field").and_then(Value::str) == Some(field))
            else {
                continue;
            };
            let at = |k: &str| p.get(k).and_then(Value::f64).unwrap_or(0.0) as usize;
            let part: String = chars
                .get(at("start")..at("end").min(chars.len()))
                .unwrap_or(&[])
                .iter()
                .collect();
            let value = match field {
                "time" => row
                    .get("time_min")
                    .and_then(Value::f64)
                    .map_or(String::new(), |t| format!("{t} min")),
                _ => row
                    .get(field)
                    .and_then(Value::str)
                    .unwrap_or("")
                    .to_string(),
            };
            line += &format!(" {field}={value} \"{part}\"");
        }
        out += &line;
        out.push('\n');
    }
    out
}

/// A plain table of a `samples/1`: file, role (`kind`, with `/wash` for a wash; a time zero shows as `t0`), time in minutes, series, replicate.
fn text_table(json: &str) -> String {
    let Ok(v) = parse(json.as_bytes()) else {
        return String::new();
    };
    let mut t = String::from("file\trole\ttime_min\tseries\treplicate\n");
    for s in v.get("samples").map_or(&[][..], Value::arr) {
        let text = |k: &str| s.get(k).and_then(Value::str).unwrap_or("");
        let time = s.get("time_min").and_then(Value::f64);
        let role = match (text("kind"), s.get("blank_kind").and_then(Value::str)) {
            ("sample", _) if time == Some(0.0) => "t0".to_string(),
            ("blank", Some(b)) => format!("blank/{b}"),
            (k, _) => k.to_string(),
        };
        t += &format!(
            "{}\t{role}\t{}\t{}\t{}\n",
            text("file"),
            time.map_or("-".to_string(), |x| x.to_string()),
            text("series"),
            s.get("replicate").and_then(Value::f64).unwrap_or(1.0)
        );
    }
    t
}

/// `sample table`: the samples/1 the names propose (or `--text`, a plain table), with the roles and times it chose logged on stderr.
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let text = args.iter().any(|a| a == "--text");
    let rest: Vec<String> = args.into_iter().filter(|a| a != "--text").collect();
    let body = match crate::samples_cmd::table(rest) {
        Ok(b) => b,
        Err(e) => return Outcome::fail(&e),
    };
    match mzlab_core::samples::route_all("propose", &body) {
        Ok(t) => {
            let mut o = Outcome::ok(if text { text_table(&t) } else { t + "\n" }.into_bytes());
            o.stderr = decisions(&body);
            o
        }
        Err(e) => Outcome::fail(&format!("{} {:?}", e.key, e.params)),
    }
}
