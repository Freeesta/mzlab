//! A table whose columns are born as rows bring them: the union of the columns in order of first appearance, as `pandas.concat` does, a missing cell is null.

use crate::attributio::export::cells::Sheet;
use crate::attributio::frame::Cell;

#[derive(Clone, Debug, Default)]
pub struct Wide {
    pub cols: Vec<String>,
    pub rows: Vec<Vec<(usize, Cell)>>,
}

impl Wide {
    fn col(&mut self, name: &str) -> usize {
        match self.cols.iter().position(|c| c == name) {
            Some(i) => i,
            None => {
                self.cols.push(name.to_string());
                self.cols.len() - 1
            }
        }
    }

    pub fn push(&mut self, row: Vec<(String, Cell)>) {
        let r = row.into_iter().map(|(k, v)| (self.col(&k), v)).collect();
        self.rows.push(r);
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The sheet with the columns of `front` (those that exist) first, then the rest in the order they came.
    pub fn sheet(&self, name: &str, front: &[&str], empty_head: &str) -> Sheet {
        if self.rows.is_empty() {
            return Sheet::new(name, &[empty_head]);
        }
        let mut order: Vec<usize> = front
            .iter()
            .filter_map(|f| self.cols.iter().position(|c| c == f))
            .collect();
        let rest: Vec<usize> = (0..self.cols.len())
            .filter(|i| !order.contains(i))
            .collect();
        order.extend(rest);
        let mut s = Sheet::with_head(name, order.iter().map(|&i| self.cols[i].clone()).collect());
        for r in &self.rows {
            s.rows.push(
                order
                    .iter()
                    .map(|&i| {
                        r.iter()
                            .find(|(k, _)| *k == i)
                            .map_or(Cell::Null, |(_, v)| v.clone())
                    })
                    .collect(),
            );
        }
        s
    }
}
