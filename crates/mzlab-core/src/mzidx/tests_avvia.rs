//! The run-wise merge of `avvia` against the merge one peak at a time.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::explicit_counter_loop
)]
use super::*;

/// `avvia` one peak at a time: the reference of the run-wise merge.
fn per_peak(gruppi: &mut [Gruppo], sp: &mut dyn Spill) -> Result<Parti> {
    let mut counts = vec![0u64; NB];
    for g in gruppi.iter() {
        for (c, &x) in counts.iter_mut().zip(&g.counts) {
            *c += u64::from(x);
        }
    }
    let mut parti = Parti::new(&counts);
    let mut next = vec![0usize; gruppi.len()];
    loop {
        let mut best: Option<(usize, (f64, f64, u32))> = None;
        for (i, g) in gruppi.iter().enumerate() {
            if let Some(p) = g.picco(next[i])
                && best.is_none_or(|(_, b)| p.2 < b.2)
            {
                best = Some((i, p));
            }
        }
        let Some((i, (mz, it, pos))) = best else {
            break;
        };
        next[i] += 1;
        parti.push(sp, mz, it, pos)?;
    }
    Ok(parti)
}
#[test]
fn the_run_merge_writes_the_bytes_of_the_peak_merge() {
    for f32 in [false, true] {
        let mut gs = vec![
            Gruppo::nuovo(1, 1, f32),
            Gruppo::nuovo(2, 1, false),
            Gruppo::nuovo(1, -1, f32),
        ];
        let mut pos = 0u32;
        for s in 0..300u32 {
            let g = &mut gs[(s % 7 % 3) as usize];
            for k in 0..(s % 50) {
                let mz = 100.0 + f64::from((k * 7919 + s) % 90000) / 100.0;
                g.metti(
                    true,
                    Peak {
                        mz,
                        it: f64::from(k),
                        pos,
                    },
                    None,
                )
                .unwrap();
            }
            pos += 1;
        }
        let mut a = std::io::Cursor::new(Vec::new());
        let mut pa = per_peak(&mut gs, &mut a).unwrap();
        pa.fine(&mut a).unwrap();
        let mut b = std::io::Cursor::new(Vec::new());
        let mut pb = avvia(&mut gs, &mut b).unwrap();
        pb.fine(&mut b).unwrap();
        assert!(!a.get_ref().is_empty());
        assert_eq!(a.get_ref(), b.get_ref(), "f32={f32}");
    }
}
