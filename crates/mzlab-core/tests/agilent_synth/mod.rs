//! The synthetic `.D` encoder shared by the Agilent tests (layout of `MSScan.xsd`, no code of the reader).

#![allow(dead_code)]

pub fn put_i32(b: &mut Vec<u8>, v: i32) {
    b.extend(v.to_le_bytes());
}
pub fn put_f64(b: &mut Vec<u8>, v: f64) {
    b.extend(v.to_le_bytes());
}

pub struct Scan {
    pub time: f64,
    pub level: i32,
    pub kind: i32,
    pub pol: i32,
    pub precursor: f64,
    pub pts: Vec<(f64, f64)>,
}

/// (MSScan.bin, MSPeak.bin) for the scans.
pub fn encode(scans: &[Scan]) -> (Vec<u8>, Vec<u8>) {
    let mut peak = vec![0u8; 68];
    let mut idx = vec![0u8; 160];
    idx[0] = 1;
    idx[1] = 1;
    idx[88..92].copy_from_slice(&160i32.to_le_bytes());
    for (k, s) in scans.iter().enumerate() {
        let off = peak.len() as i64;
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.0));
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.1));
        let tic: f64 = s.pts.iter().map(|p| p.1).sum();
        let (bmz, bv) = s
            .pts
            .iter()
            .fold((0.0, 0.0), |a, p| if p.1 > a.1 { *p } else { a });
        let r = &mut idx;
        put_i32(r, k as i32 + 1);
        put_i32(r, 1);
        put_i32(r, 0);
        put_f64(r, s.time);
        put_i32(r, s.level);
        put_i32(r, s.kind);
        put_f64(r, tic);
        put_f64(r, bmz);
        put_f64(r, bv);
        put_i32(r, k as i32 + 1);
        put_i32(r, 0);
        put_i32(r, 2);
        put_i32(r, s.pol);
        for v in [0.0, 0.0, s.precursor, 0.0, 0.0, 1050.0, 0.0] {
            put_f64(r, v);
        }
        put_i32(r, 0);
        put_i32(r, 0);
        put_i32(r, 1);
        r.extend(off.to_le_bytes());
        put_i32(r, s.pts.len() as i32 * 16);
        put_i32(r, s.pts.len() as i32);
        for v in [f64::MAX, bv, 0.0, 0.0] {
            put_f64(r, v);
        }
    }
    (idx, peak)
}

pub fn sample() -> Vec<Scan> {
    let spec = |t: f64, k: f64| Scan {
        time: t,
        level: 1,
        kind: 1,
        pol: 0,
        precursor: 0.0,
        pts: vec![(50.5, 10.0 * k), (84.0, 500.0 * k), (86.25, 320.5 * k)],
    };
    vec![
        spec(3.0, 1.0),
        spec(3.01, 2.0),
        Scan {
            time: 3.02,
            level: 1,
            kind: 1,
            pol: 0,
            precursor: 0.0,
            pts: vec![],
        },
        spec(3.03, 0.5),
    ]
}
