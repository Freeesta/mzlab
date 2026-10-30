//! Helpers of the NTS options of the filters: the groups of replicates, the samples that fell back to all blanks, and the extras (labels, standard row)
//! a view hands to `filters::apply_extra`.

use crate::filters::Rule;
use crate::filters_blank::{self as fb, Labels, Matching};
use crate::norm::decides;
use crate::samples::Sample;

/// Files a replicate fraction needs of a group of `n`: the fraction of `n` rounded to a whole file, never under one.
pub(crate) fn needed(fraction: f64, n: usize) -> usize {
    ((fraction * n as f64).round() as usize).max(1)
}

/// The groups of replicates: the sample files (not blanks, dark files or QC) with the same series, role, time and concentration, or with the same group when
/// the group is set to something other than the role. Groups in order of their first file.
pub fn replicate_groups(samples: &[Sample], labels: &Labels) -> Vec<Vec<usize>> {
    let mut keys: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (k, s) in samples
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.kind.as_str(), "sample" | "t0" | "std" | "calibration"))
    {
        let group = match labels.group_of(k) {
            "" if s.group != s.kind => s.group.as_str(),
            g => g,
        };
        let key = if !group.is_empty() {
            format!("g|{group}")
        } else {
            format!(
                "r|{}|{}|{:?}|{:?}",
                labels.series_of(k),
                s.kind,
                s.time_min,
                s.conc
            )
        };
        match keys.iter().position(|x| *x == key) {
            Some(i) => groups[i].push(k),
            None => {
                keys.push(key);
                groups.push(vec![k]);
            }
        }
    }
    groups
}

/// The samples that took the fallback of a matched blank rule (all blanks), per file; `None` when no rule matches blanks.
pub fn blank_fallback(samples: &[Sample], rules: &[Rule], labels: &Labels) -> Option<Vec<bool>> {
    let mut out: Option<Vec<bool>> = None;
    let (blank, sample) = blank_and_samples(samples);
    for r in rules {
        if let Rule::BlankWith(_, o) = r {
            if o.matching != Matching::All {
                let (_, fb) = fb::blank_groups(o.matching, labels, &sample, &blank, samples.len());
                let acc = out.get_or_insert_with(|| vec![false; samples.len()]);
                acc.iter_mut().zip(fb).for_each(|(a, b)| *a |= b);
            }
        }
    }
    out
}

pub(crate) fn blank_and_samples(samples: &[Sample]) -> (Vec<usize>, Vec<usize>) {
    let blank = (0..samples.len())
        .filter(|&k| samples[k].kind == "blank")
        .collect();
    let sample = (0..samples.len())
        .filter(|&k| decides(&samples[k]))
        .collect();
    (blank, sample)
}

/// What a view adds to the plain rules: the labels of the files and the row of the internal standard.
#[derive(Debug, Clone, Default)]
pub struct Extra<'a> {
    pub labels: Option<&'a Labels>,
    pub istd_row: Option<usize>,
}
