//! The named views a chart can point to (`tic:f1`, `tic:*`, `bpc:*`, `features:area`, `features:mzrt`, `qc:cv`, `drift:rt`, `blank:ratio`, `peaks:width`) and the
//! charts offered without a model. The page hands over what the engine already produced (same sources as `contesto/1`, plus `qc`); only the views that
//! can be built now are listed. Used by `chart` (validation) and by the transport, which gives the list to the chart-drawing prompt.

use mzlab_core::error::Result;
use mzlab_core::json::{parse, Out, Value};

use crate::chart::{make, Col, Field, Kind, View};

const fn f(name: &'static str, kind: Kind, unit: &'static str) -> Field {
    Field { name, kind, unit }
}

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::f64)
        .collect()
}

/// The label of file `k` in a chart: its name, or the neutral alias when the page hides names.
fn label(file: &Value, k: usize) -> String {
    file.get("name")
        .and_then(Value::str)
        .map_or_else(|| format!("f{}", k + 1), str::to_string)
}

fn curve(file: &Value, key: &str) -> Option<(Vec<f64>, Vec<f64>)> {
    let c = file.get(key)?;
    let (rt, y) = (nums(c.get("rt")), nums(c.get("i")));
    let n = rt.len().min(y.len());
    (n > 0).then(|| (rt[..n].to_vec(), y[..n].to_vec()))
}

/// `tic:f1`.., `tic:*` and the same for `bpc`.
fn curves(src: &Value, key: &str, name: &str, out: &mut Vec<View>) {
    let (mut rt, mut y, mut who) = (Vec::new(), Vec::new(), Vec::new());
    let mut count = 0;
    for (k, file) in src
        .get("files")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
    {
        let Some((r, i)) = curve(file, key) else {
            continue;
        };
        count += 1;
        out.push(View {
            id: format!("{key}:f{}", k + 1),
            description: format!("{name} of file f{}", k + 1),
            fields: vec![
                f("rt", Kind::Quantitative, "min"),
                f("intensity", Kind::Quantitative, "cps"),
            ],
            cols: vec![Col::Num(r.clone()), Col::Num(i.clone())],
        });
        who.extend(std::iter::repeat_n(label(file, k), r.len()));
        rt.extend(r);
        y.extend(i);
    }
    if count > 0 {
        out.push(View {
            id: format!("{key}:*"),
            description: format!("{name} of every open file"),
            fields: vec![
                f("rt", Kind::Quantitative, "min"),
                f("intensity", Kind::Quantitative, "cps"),
                f("file", Kind::Nominal, ""),
            ],
            cols: vec![Col::Num(rt), Col::Num(y), Col::Text(who)],
        });
    }
}

fn cells(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// `features:area` (row, file, area) and `features:mzrt` (mz, rt, area) from the table: columns `id` (or `row`), `mz`, `rt`, then `area` or one numeric column per file.
fn features(src: &Value, out: &mut Vec<View>) {
    let Some(csv) = src.get("features_csv").and_then(Value::str) else {
        return;
    };
    let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
    let head = lines.next().map(cells).unwrap_or_default();
    let rows: Vec<Vec<String>> = lines.map(cells).collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let (id, mz, rt) = (col(&["id", "row", "row ID"]), col(&["mz"]), col(&["rt"]));
    let num = |r: &Vec<String>, c: usize| r.get(c).and_then(|x| x.trim().parse::<f64>().ok());
    let files: Vec<String> = src
        .get("files")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
        .map(|(k, x)| label(x, k))
        .collect();
    let areas: Vec<(usize, String)> = head
        .iter()
        .enumerate()
        .filter(|(_, h)| h.as_str() == "area" || files.contains(h))
        .map(|(c, h)| {
            (
                c,
                if h == "area" {
                    "all".to_string()
                } else {
                    h.clone()
                },
            )
        })
        .collect();
    if areas.is_empty() {
        return;
    }
    let (mut r, mut who, mut a) = (Vec::new(), Vec::new(), Vec::new());
    for (n, row) in rows.iter().enumerate() {
        let name = id
            .and_then(|c| row.get(c).cloned())
            .unwrap_or_else(|| n.to_string());
        for (c, file) in &areas {
            if let Some(x) = num(row, *c) {
                r.push(name.clone());
                who.push(file.clone());
                a.push(x);
            }
        }
    }
    out.push(View {
        id: "features:area".into(),
        description: "Area of each feature in each file".into(),
        fields: vec![
            f("row", Kind::Nominal, ""),
            f("file", Kind::Nominal, ""),
            f("area", Kind::Quantitative, "counts"),
        ],
        cols: vec![Col::Text(r), Col::Text(who), Col::Num(a)],
    });
    if let (Some(mz), Some(rt)) = (mz, rt) {
        out.push(mzrt(&rows, &areas, mz, rt));
    }
}

/// `features:mzrt`: m/z against RT of each feature, with its largest area.
fn mzrt(rows: &[Vec<String>], areas: &[(usize, String)], mz: usize, rt: usize) -> View {
    let num = |r: &Vec<String>, c: usize| r.get(c).and_then(|x| x.trim().parse::<f64>().ok());
    let (mut m, mut t, mut a) = (Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        let best = areas
            .iter()
            .filter_map(|(c, _)| num(row, *c))
            .fold(f64::NAN, f64::max);
        if let (Some(x), Some(y)) = (num(row, mz), num(row, rt))
            && best.is_finite()
        {
            m.push(x);
            t.push(y);
            a.push(best);
        }
    }
    View {
        id: "features:mzrt".into(),
        description: "m/z against RT of each feature, with its largest area".into(),
        fields: vec![
            f("mz", Kind::Quantitative, "m/z"),
            f("rt", Kind::Quantitative, "min"),
            f("area", Kind::Quantitative, "counts"),
        ],
        cols: vec![Col::Num(m), Col::Num(t), Col::Num(a)],
    }
}

/// A table of objects given by the page (`qc.cv`, `qc.drift`, `qc.blank`): the numeric columns by key, the nominal ones as text.
fn table(v: Option<&Value>, nominal: &[&str], numeric: &[&str]) -> Option<Vec<Col>> {
    let a = v?.arr();
    let ok: Vec<&Value> = a
        .iter()
        .filter(|r| {
            numeric
                .iter()
                .all(|k| r.get(k).and_then(Value::f64).is_some())
        })
        .collect();
    if ok.is_empty() {
        return None;
    }
    let text = |k: &str| {
        Col::Text(
            ok.iter()
                .map(|r| {
                    r.get(k).map_or(String::new(), |x| {
                        x.str().map_or_else(
                            || x.f64().map_or(String::new(), |n| n.to_string()),
                            str::to_string,
                        )
                    })
                })
                .collect(),
        )
    };
    let num = |k: &str| {
        Col::Num(
            ok.iter()
                .filter_map(|r| r.get(k).and_then(Value::f64))
                .collect(),
        )
    };
    Some(
        nominal
            .iter()
            .map(|k| text(k))
            .chain(numeric.iter().map(|k| num(k)))
            .collect(),
    )
}

fn qc(src: &Value, out: &mut Vec<View>) {
    let q = src.get("qc");
    if let Some(cols) = table(q.and_then(|q| q.get("cv")), &["row"], &["cv_pct"]) {
        out.push(View {
            id: "qc:cv".into(),
            description: "CV of the QC areas per row".into(),
            fields: vec![
                f("row", Kind::Nominal, ""),
                f("cv_pct", Kind::Quantitative, "%"),
            ],
            cols,
        });
    }
    if let Some(cols) = table(
        q.and_then(|q| q.get("drift")),
        &["file"],
        &["order", "drift_min"],
    ) {
        let (file, order, drift) = (cols[0].clone(), cols[1].clone(), cols[2].clone());
        out.push(View {
            id: "drift:rt".into(),
            description: "RT drift of the QC injections against the first one".into(),
            fields: vec![
                f("order", Kind::Ordinal, "injection"),
                f("file", Kind::Nominal, ""),
                f("drift_min", Kind::Quantitative, "min"),
            ],
            cols: vec![order, file, drift],
        });
    }
    if let Some(cols) = table(q.and_then(|q| q.get("blank")), &["row"], &["ratio"]) {
        out.push(View {
            id: "blank:ratio".into(),
            description: "Sample to blank area ratio per row".into(),
            fields: vec![
                f("row", Kind::Nominal, ""),
                f("ratio", Kind::Quantitative, ""),
            ],
            cols,
        });
    }
}

fn widths(src: &Value, out: &mut Vec<View>) {
    let (mut who, mut w) = (Vec::new(), Vec::new());
    for (k, file) in src
        .get("files")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
    {
        for x in nums(file.get("widths")) {
            who.push(label(file, k));
            w.push(x);
        }
    }
    if !w.is_empty() {
        out.push(View {
            id: "peaks:width".into(),
            description: "Width at half height of the detected peaks".into(),
            fields: vec![
                f("file", Kind::Nominal, ""),
                f("width_s", Kind::Quantitative, "s"),
            ],
            cols: vec![Col::Text(who), Col::Num(w)],
        });
    }
}

/// The views that can be built from `sources` now, in a fixed order.
pub fn views(src: &Value) -> Vec<View> {
    let mut out = Vec::new();
    curves(src, "tic", "TIC", &mut out);
    curves(src, "bpc", "Base peak chromatogram", &mut out);
    features(src, &mut out);
    qc(src, &mut out);
    widths(src, &mut out);
    out
}

pub fn views_of(sources: &str) -> Result<Vec<View>> {
    let v = parse(sources.as_bytes())
        .map_err(|_| mzlab_core::error::Error::new("err.llm.chart.sources"))?;
    Ok(views(&v))
}

/// The list the drawing prompt gets (`<data source="views">`): ref, description, typed fields with unit, number of rows.
pub fn views_json(views: &[View]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (k, v) in views.iter().enumerate() {
        o.raw(if k > 0 { ",{\"ref\":" } else { "{\"ref\":" })
            .str(&v.id)
            .raw(",\"description\":")
            .str(&v.description)
            .raw(",\"fields\":[");
        for (j, fl) in v.fields.iter().enumerate() {
            o.raw(if j > 0 { ",{\"name\":" } else { "{\"name\":" })
                .str(fl.name)
                .raw(",\"type\":")
                .str(fl.kind.name());
            if !fl.unit.is_empty() {
                o.raw(",\"unit\":").str(fl.unit);
            }
            o.raw("}");
        }
        o.raw("],\"rows\":").int(v.rows() as i64).raw("}");
    }
    o.raw("]");
    o.0
}

/// The charts that need no model (state «rules only»): overlaid TIC, widths of the peaks, RT drift of the QC, CV of the QC, sample to blank ratio.
/// Each is a `grafico/1` with its rows, built through the same path as a chart of the model; only those whose view exists. The `title` is a stable id
/// (`tic`, `width`, `drift`, `cv`, `blank`) that the page turns into its own text.
pub fn presets(views: &[View]) -> Vec<String> {
    let specs: [(&str, &str, &str, &str, &str, &str); 5] = [
        ("tic", "tic:*", "line", "rt", "intensity", "file"),
        ("width", "peaks:width", "hist", "width_s", "", ""),
        ("drift", "drift:rt", "line", "order", "drift_min", "file"),
        ("cv", "qc:cv", "hist", "cv_pct", "", ""),
        ("blank", "blank:ratio", "hist", "ratio", "", ""),
    ];
    let mut out = Vec::new();
    for (title, view, mark, x, y, color) in specs {
        if !views.iter().any(|v| v.id == view) {
            continue;
        }
        // more series than colours: the same chart with the series as `detail`
        for channel in ["color", "detail"] {
            let mut s = Out::default();
            s.raw("{\"schema\":\"grafico/1\",\"title\":")
                .str(title)
                .raw(",\"data\":{\"ref\":")
                .str(view)
                .raw("},\"mark\":")
                .str(mark)
                .raw(",\"encoding\":{\"x\":{\"field\":")
                .str(x)
                .raw("}");
            for (k, v) in [("y", y), (channel, color)] {
                if !v.is_empty() {
                    s.raw(",").str(k).raw(":{\"field\":").str(v).raw("}");
                }
            }
            s.raw("},\"why\":").str(&format!("preset:{title}")).raw("}");
            if let Ok(c) = make(&s.0, views) {
                out.push(c);
                break;
            }
        }
    }
    out
}
