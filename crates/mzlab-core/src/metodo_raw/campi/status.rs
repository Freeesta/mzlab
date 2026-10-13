//! The status log of the instrument (`opentfraw::status_log`) as fields of `metodo/1`: the value of each channel when it is the same in every
//! record, else `min..max`; the series over time are `StatusLog::series`. The titles of the sections (`====  Ion Source:  ====`) carry no value.

use super::{shown, Campo};
use opentfraw::generic_data::GenericValue;
use opentfraw::status_log::{split_label, StatusLog};

const FROM: &str = "thermo-raw:StatusLog";

pub fn fields(log: &StatusLog) -> Vec<Campo> {
    let mut out = vec![Campo::new(FROM, "Records", log.records.len())];
    let times: Vec<f64> = log.times_min.iter().map(|t| f64::from(*t)).collect();
    if !times.is_empty() && times.iter().all(|t| t.is_finite()) {
        let (lo, hi) = range(&times);
        out.push(Campo::new(FROM, "Time (min..max)", format!("{lo}..{hi}")).unit("min"));
    }
    for (i, d) in log.header.fields.iter().enumerate() {
        let vals: Vec<&GenericValue> = log
            .records
            .iter()
            .filter_map(|r| r.values.get(i).map(|v| &v.1))
            .filter(|v| !matches!(v, GenericValue::Gap))
            .collect();
        if vals.is_empty() {
            continue;
        }
        out.extend(channel(&d.label, &vals));
    }
    out
}

fn range(nums: &[f64]) -> (f64, f64) {
    let lo = nums.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = nums.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lo, hi)
}

/// One channel: all its values, in record order.
fn channel(label: &str, vals: &[&GenericValue]) -> Option<Campo> {
    let (name, unit) = split_label(label);
    let texts: Vec<String> = vals.iter().map(|v| shown(v)).collect();
    if name.is_empty() || texts.iter().all(String::is_empty) {
        return None;
    }
    let same = texts.iter().all(|t| Some(t) == texts.first());
    let nums: Vec<f64> = vals
        .iter()
        .filter_map(|v| v.as_f64())
        .filter(|x| x.is_finite())
        .collect();
    let (value, key) = if same {
        (texts[0].clone(), name)
    } else if nums.len() == vals.len() {
        let (lo, hi) = range(&nums);
        (format!("{lo}..{hi}"), format!("{name} (min..max)"))
    } else {
        let mut u = texts.clone();
        u.sort();
        u.dedup();
        (u.len().to_string(), format!("{name} (distinct values)"))
    };
    Some(Campo::new(FROM, &key, value).unit(&unit))
}

#[cfg(test)]
mod tests;
