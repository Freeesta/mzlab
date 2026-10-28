//! The block cache of a `Library`: decoded blocks and index pages under a byte budget, least recently used out first.

use super::Kind;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

type Key = (Kind, u32);

pub(super) struct Cache {
    /// Block and the tick of its last use.
    map: HashMap<Key, (Rc<Vec<u8>>, u64)>,
    /// Uses in order, newest last. A use whose tick is not the one stored in `map` is stale (the block was used again or dropped); stale uses
    /// are skipped when the oldest block is looked for and swept out when they outnumber the live ones.
    order: VecDeque<(u64, Key)>,
    bytes: usize,
    budget: usize,
    tick: u64,
}

impl Cache {
    pub(super) fn new(budget: usize) -> Cache {
        Cache {
            map: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            budget,
            tick: 0,
        }
    }

    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }

    fn touch(&mut self, k: Key) -> u64 {
        self.tick += 1;
        self.order.push_back((self.tick, k));
        if self.order.len() > 4 * self.map.len() + 64 {
            let map = &self.map;
            self.order
                .retain(|(t, k)| map.get(k).is_some_and(|e| e.1 == *t));
        }
        self.tick
    }

    pub(super) fn contains(&self, k: Key) -> bool {
        self.map.contains_key(&k)
    }

    pub(super) fn get(&mut self, k: Key) -> Option<Rc<Vec<u8>>> {
        if !self.map.contains_key(&k) {
            return None;
        }
        let t = self.touch(k);
        self.map.get_mut(&k).map(|e| {
            e.1 = t;
            e.0.clone()
        })
    }

    pub(super) fn put(&mut self, k: Key, v: Rc<Vec<u8>>) {
        let t = self.touch(k);
        self.bytes += v.len();
        if let Some(old) = self.map.insert(k, (v, t)) {
            self.bytes -= old.0.len();
        }
        while self.bytes > self.budget && self.map.len() > 1 {
            let Some(oldest) = self.pop_oldest() else {
                break;
            };
            if oldest == k {
                // cannot happen with more than one block, since `k` is the newest
                self.order.push_front((t, k));
                break;
            }
            self.drop_block(oldest);
        }
    }

    /// Changes the budget; the excess is dropped at once, oldest first.
    pub(super) fn set_budget(&mut self, bytes: usize) {
        self.budget = bytes;
        while self.bytes > self.budget {
            let Some(oldest) = self.pop_oldest() else {
                break;
            };
            self.drop_block(oldest);
        }
    }

    fn drop_block(&mut self, k: Key) {
        if let Some(e) = self.map.remove(&k) {
            self.bytes -= e.0.len();
        }
    }

    /// The least recently used block, taken out of the order (not out of the map).
    fn pop_oldest(&mut self) -> Option<Key> {
        while let Some((t, k)) = self.order.pop_front() {
            if self.map.get(&k).is_some_and(|e| e.1 == t) {
                return Some(k);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
