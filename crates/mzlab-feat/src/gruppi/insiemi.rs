//! The exact sets of conditions in which the rows are present (the columns of an UpSet plot).

use super::presenza::CondModel;
use super::{RowOut, SetOut};

/// One set per distinct combination of present conditions (never the empty one), ordered by number of rows (descending), then by names.
pub(crate) fn sets(conds: &[CondModel], rows: &[RowOut]) -> Vec<SetOut> {
    let mut found: Vec<(Vec<String>, Vec<u32>)> = Vec::new();
    for r in rows {
        let key: Vec<String> = conds
            .iter()
            .filter(|c| r.present_in.contains(&c.id))
            .map(|c| c.id.clone())
            .collect();
        if key.is_empty() {
            continue;
        }
        match found.iter_mut().find(|(k, _)| k == &key) {
            Some(e) => e.1.push(r.meta.id),
            None => found.push((key, vec![r.meta.id])),
        }
    }
    let mut ix: Vec<usize> = (0..found.len()).collect();
    mzlab_core::sortc::by(&mut ix, |&a, &b| {
        found[b]
            .1
            .len()
            .cmp(&found[a].1.len())
            .then_with(|| found[a].0.cmp(&found[b].0))
    });
    ix.into_iter()
        .map(|i| SetOut {
            conditions: found[i].0.clone(),
            rows: found[i].1.clone(),
        })
        .collect()
}
