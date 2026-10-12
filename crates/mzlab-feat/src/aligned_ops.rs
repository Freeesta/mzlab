//! Queries and the byte layout of `Aligned`.

use crate::aligned::Aligned;

impl Aligned {
    /// Rows with the reference ion or any member within `ppm` of `mz`, ascending (the way to find an ion that is not the reference of its family).
    pub fn rows_near(&self, mz: f64, ppm: f64) -> Vec<usize> {
        let tol = mz * ppm * 1e-6;
        (0..self.len())
            .filter(|&r| {
                (self.mz[r] - mz).abs() <= tol
                    || self
                        .members(r)
                        .any(|m| (self.member_mz[m] - mz).abs() <= tol)
            })
            .collect()
    }

    /// Bytes of the table in a fixed layout (little-endian, column by column): the input of the digest used to compare two runs.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.heap_bytes() + 32);
        b.extend_from_slice(&(self.len() as u64).to_le_bytes());
        b.extend_from_slice(&(self.n_files as u64).to_le_bytes());
        macro_rules! put {
            ($($c:ident),*) => { $( for v in &self.$c { b.extend_from_slice(&v.to_le_bytes()); } )* };
        }
        put!(
            mz,
            rt,
            area,
            height,
            apex_rt,
            width,
            member_start,
            member_mz,
            member_rt
        );
        b.extend_from_slice(&self.state);
        b.extend_from_slice(&self.member_role);
        b
    }
}
