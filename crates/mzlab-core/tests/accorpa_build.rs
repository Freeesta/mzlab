//! The `.mzidx` built through `accorpa::Accorpa` (the coalescing the browser uses for its OPFS output and scratch file) has the bytes of the
//! direct build, with a small budget that sends the peaks to the scratch area, and far fewer calls to the destinations.
mod lettore_finto;
use std::io::Cursor;

use lettore_finto::Finto;
use mzlab_core::accorpa::Accorpa;
use mzlab_core::mzidx_spill::Spill;
use mzlab_core::{lettori_mzml, mzidx, upload};

#[derive(Default)]
struct Conta {
    c: Cursor<Vec<u8>>,
    calls: usize,
}
impl mzidx::Output for Conta {
    fn write_at(&mut self, o: u64, d: &[u8]) -> mzlab_core::Result<()> {
        self.calls += 1;
        self.c.write_at(o, d)
    }
}
impl Spill for Conta {
    fn put(&mut self, o: u64, d: &[u8]) -> mzlab_core::Result<()> {
        self.calls += 1;
        Spill::put(&mut self.c, o, d)
    }
    fn get(&mut self, o: u64, b: &mut [u8]) -> mzlab_core::Result<()> {
        Spill::get(&mut self.c, o, b)
    }
}

#[test]
fn coalesced_index_is_the_same() {
    let mut l = Finto::vario(1500, 400, 31);
    let mut mz = Vec::new();
    lettori_mzml::scrivi(&mut l, 0, &mut mz, &mut |_, _| {}).unwrap();
    let p = upload::prepare(&mut mz.clone(), mz.len() as u64, "c.mzML").unwrap();
    let budget = 1 << 20;
    let build = |join: bool| {
        let (mut out, mut sp) = (Conta::default(), Conta::default());
        let st = if join {
            let (mut o, mut s) = (Accorpa::new(out), Accorpa::new(sp));
            let st =
                mzidx::build_mzml_spill(&p.table, Cursor::new(&mz), &mut o, budget, Some(&mut s))
                    .unwrap();
            o.fine().unwrap();
            (out, sp) = (o.into_inner(), s.into_inner());
            st
        } else {
            mzidx::build_mzml_spill(&p.table, Cursor::new(&mz), &mut out, budget, Some(&mut sp))
                .unwrap()
        };
        assert_eq!(st.passes, 1, "the peaks go to the scratch area");
        (out.c.into_inner(), out.calls, sp.calls)
    };
    let (direct, out_d, sp_d) = build(false);
    let (joined, out_j, sp_j) = build(true);
    assert!(sp_d > 0, "no scratch writes");
    assert!(joined == direct, "the coalesced index differs");
    assert!(sp_j * 8 <= sp_d, "scratch puts {sp_j} against {sp_d}");
    assert!(out_j < out_d, "output writes {out_j} against {out_d}");
}
