//! `Sqlite::walk_with`: the iterative walk of a table tree. In `tolerant` mode a page that is damaged (unreadable, a wrong type or count, a cycle) is left
//! out with what hangs from it, and a record that does not decode is left out alone; both are counted (`take_skips`), the rest of the table is read.

use super::*;
use mzlab_core::readers::damages_one;

impl Sqlite {
    /// `scorri` that decodes only the record positions `voglio` marks `true` (the others are `Val::Null`; `None` = all) and hands `f` one vector reused from row to row, which `f` may take from.
    pub fn walk_with<F>(&mut self, root: u32, want: Option<&[bool]>, mut f: F) -> Result<()>
    where
        F: FnMut(i64, &mut Vec<Val>) -> Result<bool>,
    {
        let mut stack: Vec<Step> = vec![Step::Page(root, 0)];
        let mut views = HashSet::new();
        let mut bufs = (Vec::new(), Vec::new(), Vec::new());
        while let Some(step) = stack.pop() {
            let (n, prof) = match step {
                Step::Row(mut v) => {
                    if !f(0, &mut v)? {
                        return Ok(());
                    }
                    continue;
                }
                Step::Page(n, prof) => (n, prof),
            };
            self.limits.profondita_ok(prof)?;
            match self.page_step(n, prof, want, &mut views, &mut stack, &mut bufs, &mut f) {
                Ok(true) => {}
                Ok(false) => return Ok(()),
                Err(e) if self.tolerant && damages_one(&e) => self.skips.0 += 1,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// The pages and records left out since the last call (`tolerant` mode), and starts counting again.
    pub fn take_skips(&mut self) -> (usize, usize) {
        std::mem::take(&mut self.skips)
    }

    /// Moves what the last walk left out into `left_out` under `table`; a table walked again (its damage counted once) adds nothing.
    pub fn note_skips(&mut self, table: &str) {
        let (pages, records) = self.take_skips();
        if pages + records > 0 && !self.left_out.iter().any(|t| t.0 == table) {
            self.left_out.push((table.to_string(), pages, records));
        }
    }

    /// One page of the walk: its children (interior) or its rows (leaf) given to `f`; `false` when `f` asks to stop.
    #[allow(clippy::too_many_arguments)]
    fn page_step<F>(
        &mut self,
        n: u32,
        prof: usize,
        want: Option<&[bool]>,
        views: &mut HashSet<u32>,
        stack: &mut Vec<Step>,
        (p, q, row): &mut (Vec<u8>, Vec<u8>, Vec<Val>),
        f: &mut F,
    ) -> Result<bool>
    where
        F: FnMut(i64, &mut Vec<Val>) -> Result<bool>,
    {
        if !views.insert(n) {
            return Err(broken("page cycle"));
        }
        self.page_in(n, p)?;
        let (base, kind, ncells, cap) = self.page_header(p, n)?;
        let npag = self.page;
        let cell_list = |p: &[u8], i: usize| -> Result<usize> {
            let c = u16_be(p, cap + 2 * i).ok_or_else(|| broken("cell pointer"))?;
            (c < npag).then_some(c).ok_or_else(|| broken("cell offset"))
        };
        match kind {
            INT_TAB => {
                let right = u32_be(p, base + 8).ok_or_else(|| broken("page"))?;
                stack.push(Step::Page(right, prof + 1));
                for i in (0..ncells).rev() {
                    let c = cell_list(p, i)?;
                    let child = u32_be(p, c).ok_or_else(|| broken("cell"))?;
                    stack.push(Step::Page(child, prof + 1));
                }
            }
            INT_IDX => {
                // left subtree, the key of the cell, the next left subtree, ..., the right-most child
                let destra = u32_be(p, base + 8).ok_or_else(|| broken("page"))?;
                stack.push(Step::Page(destra, prof + 1));
                for i in (0..ncells).rev() {
                    let c = cell_list(p, i)?;
                    let mut v = Vec::new();
                    self.cell(p, c + 4, true, want, q, &mut v)?;
                    stack.push(Step::Row(v));
                    let child = u32_be(p, c).ok_or_else(|| broken("cell"))?;
                    stack.push(Step::Page(child, prof + 1));
                }
            }
            LEAF_TAB | LEAF_IDX => {
                for i in 0..ncells {
                    let read = cell_list(p, i)
                        .and_then(|c| self.cell(p, c, kind == LEAF_IDX, want, q, row));
                    let rowid = match read {
                        Ok(r) => r,
                        Err(e) if self.tolerant && damages_one(&e) => {
                            self.skips.1 += 1;
                            continue;
                        }
                        Err(e) => return Err(e),
                    };
                    if !f(rowid, row)? {
                        return Ok(false);
                    }
                }
            }
            _ => return Err(broken("page type")),
        }
        Ok(true)
    }
}
