//! The LRU cache evicts exactly like a naive full-scan LRU.

use super::*;

/// The cache it replaces: the minimum of the ticks over the whole map at every eviction.
struct Naive {
    map: HashMap<Key, (usize, u64)>,
    bytes: usize,
    budget: usize,
    tick: u64,
}

impl Naive {
    fn get(&mut self, k: Key) -> bool {
        self.tick += 1;
        let t = self.tick;
        self.map.get_mut(&k).map(|e| e.1 = t).is_some()
    }

    fn put(&mut self, k: Key, len: usize) {
        self.tick += 1;
        self.bytes += len;
        if let Some(old) = self.map.insert(k, (len, self.tick)) {
            self.bytes -= old.0;
        }
        while self.bytes > self.budget && self.map.len() > 1 {
            let oldest = *self
                .map
                .iter()
                .min_by_key(|(_, e)| e.1)
                .map(|(k, _)| k)
                .unwrap();
            if oldest == k {
                break;
            }
            self.bytes -= self.map.remove(&oldest).unwrap().0;
        }
    }

    fn set_budget(&mut self, bytes: usize) {
        self.budget = bytes;
        while self.bytes > self.budget && !self.map.is_empty() {
            let oldest = *self
                .map
                .iter()
                .min_by_key(|(_, e)| e.1)
                .map(|(k, _)| k)
                .unwrap();
            self.bytes -= self.map.remove(&oldest).unwrap().0;
        }
    }
}

#[test]
fn evicts_the_same_blocks_as_the_full_scan() {
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut fast = Cache::new(5000);
    let mut slow = Naive {
        map: HashMap::new(),
        bytes: 0,
        budget: 5000,
        tick: 0,
    };
    for step in 0..60_000 {
        let r = rnd();
        let k = (
            if r & 1 == 0 { Kind::Peaks } else { Kind::Frag },
            ((r >> 8) % 90) as u32,
        );
        match (r >> 40) % 40 {
            0 => {
                let b = (rnd() % 8000) as usize;
                fast.set_budget(b);
                slow.set_budget(b);
            }
            1..=20 => {
                assert_eq!(fast.get(k).is_some(), slow.get(k), "step {step}");
            }
            _ => {
                let len = 100 + (rnd() % 900) as usize;
                fast.put(k, Rc::new(vec![0u8; len]));
                slow.put(k, len);
            }
        }
        assert_eq!(fast.bytes, slow.bytes, "step {step}");
        assert_eq!(fast.map.len(), slow.map.len(), "step {step}");
        if step % 97 == 0 {
            assert!(
                slow.map.keys().all(|k| fast.map.contains_key(k)),
                "step {step}"
            );
        }
        assert!(fast.order.len() <= 4 * fast.map.len() + 65);
    }
}
