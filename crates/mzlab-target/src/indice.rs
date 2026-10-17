// Binary search indexes and fast lookup for target screening collections.
use std::collections::HashMap;

use crate::entry::{AdductType, TargetEntry};

impl TargetList {
    /// Construct a target list and build sorted m/z binary search indexes
    /// and hash maps for identifier and Neutral formula lookups.
    pub fn from_entries(entries: Vec<TargetEntry>) -> Self {
        let n = entries.len();
        let mut idx_mh = Vec::with_capacity(n);
        let mut idx_mna = Vec::with_capacity(n);
        let mut idx_mnh4 = Vec::with_capacity(n);
        let mut id_index = HashMap::with_capacity(n);
        let mut formula_index: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, entry) in entries.iter().enumerate() {
            idx_mh.push((entry.mz_mh, i));
            idx_mna.push((entry.mz_mna, i));
            idx_mnh4.push((entry.mz_mnh4, i));
            id_index.insert(entry.id.clone(), i);
            formula_index
                .entry(entry.neutral_formula.clone())
                .or_default()
                .push(i);
        }

        let cmp_mz = |a: &(f64, usize), b: &(f64, usize)| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        };

        idx_mh.sort_unstable_by(cmp_mz);
        idx_mna.sort_unstable_by(cmp_mz);
        idx_mnh4.sort_unstable_by(cmp_mz);

        Self {
            entries,
            idx_mh,
            idx_mna,
            idx_mnh4,
            id_index,
            formula_index,
        }
    }

    /// Find all entries matching the target m/z within ppm tolerance for the selected adduct.
    /// Executes via binary search in O(log N + K).
    pub fn find_by_mz(&self, mz: f64, ppm: f64, adduct: AdductType) -> Vec<&TargetEntry> {
        let sorted_idx = match adduct {
            AdductType::MH => &self.idx_mh,
            AdductType::MNa => &self.idx_mna,
            AdductType::MNH4 => &self.idx_mnh4,
        };

        let tol = mz * ppm * 1e-6;
        let min_mz = mz - tol;
        let max_mz = mz + tol;

        let start = sorted_idx.partition_point(|&(m, _)| m < min_mz);
        let mut results = Vec::new();
        for &(m, idx) in &sorted_idx[start..] {
            if m > max_mz {
                break;
            }
            if let Some(entry) = self.entries.get(idx) {
                results.push(entry);
            }
        }
        results
    }

    /// Lookup an entry by unique identifier slug in O(1).
    pub fn find_by_id(&self, id: &str) -> Option<&TargetEntry> {
        let &idx = self.id_index.get(id)?;
        self.entries.get(idx)
    }

    /// Lookup entries matching the specified exact Neutral formula in O(1).
    pub fn find_by_formula(&self, neutral_formula: &str) -> Vec<&TargetEntry> {
        match self.formula_index.get(neutral_formula) {
            Some(indices) => indices
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Total count of target entries in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the list contains no target entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Direct reference to the slice of all target entries.
    #[inline]
    pub fn entries(&self) -> &[TargetEntry] {
        &self.entries
    }
}

/// Indexed target list enabling rapid binary search queries across m/z and adducts.
#[derive(Debug, Clone)]
pub struct TargetList {
    entries: Vec<TargetEntry>,
    idx_mh: Vec<(f64, usize)>,
    idx_mna: Vec<(f64, usize)>,
    idx_mnh4: Vec<(f64, usize)>,
    id_index: HashMap<String, usize>,
    formula_index: HashMap<String, Vec<usize>>,
}
