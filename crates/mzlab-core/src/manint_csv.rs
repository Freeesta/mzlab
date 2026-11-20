//! The CSV of `manint::table_text`: one block per channel, the optional `sum` row, and the five columns after `origin` for page-supplied traces.

use super::*;

/// The columns appended after `origin` when a trace names its panel (`Trace::panel`): the panel, the concentration of the sample and its unit, the
/// correction note of the window, the part of the area under the baseline (counts x s; empty for `edges`), the unit of the area.
pub const EXT_HEAD: &str = ",panel,conc,conc_unit,correction,below,area_unit";
/// The CSV: the header, then for each channel (in the order of `keys`) its rows, and the `sum` row after them when `sums`.
pub(super) fn csv_text(
    rows: &[(usize, Row)],
    labels: &[String],
    corrs: &[String],
    keys: &[Channel],
    samples: &[Sample],
    sums: bool,
    cols: Cols,
) -> String {
    // the extra columns only when a trace was supplied with its panel: the bytes of a table of file transitions do not change
    let ext = samples
        .iter()
        .any(|s| s.1.iter().any(|t| t.panel.is_some()));
    let mut out = String::from(HEAD.trim_end());
    if ext {
        out += EXT_HEAD;
    }
    out += &cols.head();
    out.push('\n');
    for (ch, (_, label, _)) in keys.iter().enumerate() {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.0 == ch).map(|r| &r.1).collect();
        if mine.is_empty() {
            continue;
        }
        let head = field(label);
        for r in &mine {
            out += &format!(
                "{head},{},{},{},{:.3},{},{:.3},{:.3},{},{},{},{},{},{},{},{},{}\n",
                field(&samples[r.file].0),
                minutes_text(samples[r.file].2),
                field(&labels[r.label]),
                r.apex,
                sci(r.area, 5),
                r.a,
                r.b,
                sci(r.height, 5),
                r.snr.map_or(String::new(), |v| format!("{v:.1}")),
                r.base,
                if r.drop { "drop" } else { "" },
                r.pv.map_or(String::new(), |v| format!("{v:.1}")),
                r.origin,
                r.file,
                field(&samples[r.file].3),
                r.found.map_or(String::new(), |d| format!("{d:.3}"))
            );
            if ext {
                out.pop();
                out += &ext_cells(&samples[r.file].1[r.tr], r, &corrs[r.label]);
            }
            let more = cols.cells(&samples[r.file].1[r.tr], r);
            if !more.is_empty() {
                out.pop();
                out += &more;
                out.push('\n');
            }
        }
        if sums {
            out += &format!(
                "{head},sum,,,,{},,,,,,,,,,,{}\n",
                sci(mine.iter().map(|r| r.area).sum(), 5),
                if ext { ",,,,,," } else { "" }
            );
            if cols.any() {
                out.pop();
                out += &",".repeat(cols.count());
                out.push('\n');
            }
        }
    }
    out
}

/// The five extra cells of a row (with the leading comma and the line break): panel, concentration and its unit, correction note, below, area unit.
fn ext_cells(t: &Trace, r: &Row, corr: &str) -> String {
    format!(
        ",{},{},{},{},{},{}\n",
        field(t.panel.as_deref().unwrap_or("")),
        t.conc.map_or(String::new(), |c| format!("{c}")),
        field(if t.conc.is_some() { &t.conc_unit } else { "" }),
        field(corr),
        r.below.map_or(String::new(), |v| sci(v, 5)),
        field(&t.unit)
    )
}
