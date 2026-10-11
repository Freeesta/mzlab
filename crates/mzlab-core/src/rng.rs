//! numpy's `default_rng(seed)` generator (SeedSequence + PCG64) and the draws the Python engine uses: `permutation`, `integers`, `choice`,
//! so that the Monte Carlo and bootstrap values are the same as Python's.

/// numpy's `SeedSequence(seed).generate_state(4, uint64)` for a non-negative integer seed below 2^32.
fn seed_state(seed: u32) -> [u64; 4] {
    const MULT_A: u32 = 0x931e8875;
    const MULT_B: u32 = 0x58f38ded;
    const MIX_L: u32 = 0xca01f9dd;
    const MIX_R: u32 = 0x4973f715;
    let hashmix = |v: u32, h: &mut u32| {
        let mut v = v ^ *h;
        *h = h.wrapping_mul(MULT_A);
        v = v.wrapping_mul(*h);
        v ^ (v >> 16)
    };
    let mix = |x: u32, y: u32| {
        let r = MIX_L.wrapping_mul(x).wrapping_sub(MIX_R.wrapping_mul(y));
        r ^ (r >> 16)
    };
    let mut h = 0x43b0d7e5u32;
    let mut pool = [0u32; 4];
    for (i, p) in pool.iter_mut().enumerate() {
        *p = hashmix(if i == 0 { seed } else { 0 }, &mut h);
    }
    for s in 0..4 {
        for d in 0..4 {
            if s != d {
                let hs = hashmix(pool[s], &mut h);
                pool[d] = mix(pool[d], hs);
            }
        }
    }
    let mut hb = 0x8b51f9ddu32;
    let mut w = [0u32; 8];
    for (i, o) in w.iter_mut().enumerate() {
        let mut v = pool[i % 4] ^ hb;
        hb = hb.wrapping_mul(MULT_B);
        v = v.wrapping_mul(hb);
        *o = v ^ (v >> 16);
    }
    [0, 1, 2, 3].map(|i| w[2 * i] as u64 | (w[2 * i + 1] as u64) << 32)
}

/// numpy's PCG64 (XSL-RR 128/64) with its buffered 32-bit output (`np.random.default_rng(seed)`).
pub struct Pcg64 {
    state: u128,
    inc: u128,
    half: Option<u32>,
}

impl Pcg64 {
    const MULT: u128 = 0x2360ED051FC65DA44385DF649FCCF645;
    pub fn new(seed: u32) -> Pcg64 {
        let v = seed_state(seed);
        let init = (v[0] as u128) << 64 | v[1] as u128;
        let seq = (v[2] as u128) << 64 | v[3] as u128;
        let mut g = Pcg64 {
            state: 0,
            inc: seq << 1 | 1,
            half: None,
        };
        g.step();
        g.state = g.state.wrapping_add(init);
        g.step();
        g
    }
    fn step(&mut self) {
        self.state = self.state.wrapping_mul(Self::MULT).wrapping_add(self.inc);
    }
    pub fn next64(&mut self) -> u64 {
        self.step();
        let x = ((self.state >> 64) as u64) ^ (self.state as u64);
        x.rotate_right((self.state >> 122) as u32)
    }
    pub fn next32(&mut self) -> u32 {
        if let Some(h) = self.half.take() {
            return h;
        }
        let n = self.next64();
        self.half = Some((n >> 32) as u32);
        n as u32
    }
    /// `random_interval(max)`: a uniform integer in `0..=max` by masked rejection.
    pub fn interval(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        let mask = u32::MAX >> max.leading_zeros();
        loop {
            let v = self.next32() & mask;
            if v <= max {
                return v;
            }
        }
    }
    /// `rng.permutation(n)`.
    pub fn permutation(&mut self, n: usize) -> Vec<usize> {
        let mut a: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = self.interval(i as u32) as usize;
            a.swap(i, j);
        }
        a
    }

    /// `random_bounded_uint64(0, max)` for `max <= 0xFFFFFFFF`: Lemire's method on 32-bit draws (`rng.integers`, Floyd's `choice`).
    pub fn bounded(&mut self, max: u32) -> u32 {
        if max == u32::MAX {
            return self.next32();
        }
        let excl = max as u64 + 1;
        let mut m = self.next32() as u64 * excl;
        let mut left = m & 0xFFFF_FFFF;
        if left < excl {
            let threshold = (u32::MAX - max) as u64 % excl;
            while left < threshold {
                m = self.next32() as u64 * excl;
                left = m & 0xFFFF_FFFF;
            }
        }
        (m >> 32) as u32
    }
    /// `rng.integers(0, high, size)` (`high >= 1`).
    pub fn integers(&mut self, high: usize, size: usize) -> Vec<usize> {
        (0..size).map(|_| self.bounded((high - 1) as u32) as usize).collect()
    }
    /// `rng.choice(population, size, replace=True)` as indices into the population.
    pub fn choice_replace(&mut self, pop: usize, size: usize) -> Vec<usize> {
        self.integers(pop, size)
    }
    /// `rng.choice(population, size, replace=False)` as indices (Floyd's algorithm and a shuffle, `pop <= 10000` or `size <= pop / 50`).
    pub fn choice_no_replace(&mut self, pop: usize, size: usize) -> Vec<usize> {
        let mut idx = vec![0usize; size];
        let mask = {
            let set = (1.2 * size as f64) as u64;
            if set == 0 { 0 } else { u64::MAX >> set.leading_zeros() }
        };
        let mut table = vec![u64::MAX; mask as usize + 1];
        for j in pop - size..pop {
            let val = self.bounded(j as u32) as u64;
            let mut loc = (val & mask) as usize;
            while table[loc] != u64::MAX && table[loc] != val {
                loc = (loc + 1) & mask as usize;
            }
            let slot = j + size - pop;
            if table[loc] == u64::MAX {
                table[loc] = val;
                idx[slot] = val as usize;
            } else {
                let mut loc = j & mask as usize;
                while table[loc] != u64::MAX {
                    loc = (loc + 1) & mask as usize;
                }
                table[loc] = j as u64;
                idx[slot] = j;
            }
        }
        for i in (1..size).rev() {
            let j = self.bounded(i as u32) as usize;
            idx.swap(i, j);
        }
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // reference values from numpy: default_rng(3): integers(0,17,6), choice(40,12,False), choice(9,9,True), permutation(8)
    #[test]
    fn draws_match_numpy() {
        let mut g = Pcg64::new(3);
        assert_eq!(g.integers(17, 6), vec![13, 1, 3, 4, 3, 13]);
        assert_eq!(g.choice_no_replace(40, 12), vec![21, 29, 26, 6, 14, 1, 9, 35, 3, 10, 17, 25]);
        assert_eq!(g.choice_replace(9, 9), vec![6, 6, 8, 7, 2, 2, 5, 5, 6]);
        assert_eq!(g.permutation(8), vec![2, 4, 0, 7, 6, 1, 3, 5]);
        let mut g = Pcg64::new(0);
        assert_eq!(
            g.choice_no_replace(100, 30),
            vec![62, 65, 55, 53, 98, 23, 82, 41, 74, 94, 92, 19, 84, 1, 75, 47, 97, 49, 72, 25, 13, 52, 5, 45, 50, 60, 0, 37, 3, 95]
        );
    }
}
