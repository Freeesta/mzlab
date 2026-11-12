//! `mol/1` into a molfile: V2000, or V3000 when the record has more than 999 atoms or bonds or enhanced-stereo groups. Written so that
//! `Mol::from_molfile` reads the same record back: hydrogens through the valence field where the reader's rule would give another count,
//! tetrahedral centres as one wedge chosen with the reader's own parity rule, `unknown` centres and double bonds as "either" marks. Stereo
//! needs the 2D coordinate layer; without coordinates every atom sits at the origin and only the constitution travels.

use crate::model::{Mol, Status, Stereo, StereoGroup};
use crate::molfile::{Draft, DraftAtom, DraftBond, Wedge};
use crate::molfile_stereo::{adjacency, parity};
use crate::valence::shortfall;

/// Elements whose hydrogens the reader computes (the others get none, whatever the file says).
const ORGANIC: &[&str] = &["B", "C", "N", "O", "P", "S", "F", "Cl", "Br", "I"];
/// V2000 counts are three digits.
const V2000_MAX: usize = 999;
/// V3000 lines are at most 80 characters; longer ones continue with `-`.
const V3000_WIDTH: usize = 80;

/// One bond as it is written: the atom it starts from first (a wedge belongs to that atom), order 1-3 and the stereo mark.
struct OutBond {
    a: usize,
    b: usize,
    o: u8,
    mark: Wedge,
}

impl Mol {
    /// The molfile of the record, `title` on its first line (cut at the first line break, at most 80 bytes).
    pub fn to_molfile(&self, title: &str) -> String {
        let mut title = title.lines().next().unwrap_or("");
        let mut cut = title.len().min(80);
        while !title.is_char_boundary(cut) {
            cut -= 1;
        }
        title = &title[..cut];
        let xyz = self.xyz();
        let bonds = self.out_bonds(&xyz);
        let valence = self.valences();
        let dim = if self.coords.as_ref().is_some_and(|c| c.dim == 3) {
            "3D"
        } else {
            "2D"
        };
        let mut s = format!("{title}\n  mzLab             {dim}\n\n");
        if self.atoms.len() > V2000_MAX || bonds.len() > V2000_MAX || !self.groups.is_empty() {
            self.v3000(&mut s, &xyz, &bonds, &valence);
        } else {
            self.v2000(&mut s, &xyz, &bonds, &valence);
        }
        s.push_str("M  END\n");
        s
    }

    /// Each connected piece as its own record, in the order of its first atom; atoms keep their relative order and their layers.
    pub fn pieces(&self) -> Vec<Mol> {
        let n = self.atoms.len();
        let mut piece = vec![usize::MAX; n];
        let mut count = 0;
        let mut nb = vec![Vec::new(); n];
        for b in &self.bonds {
            nb[b.a as usize].push(b.b as usize);
            nb[b.b as usize].push(b.a as usize);
        }
        for s in 0..n {
            if piece[s] != usize::MAX {
                continue;
            }
            let mut stack = vec![s];
            piece[s] = count;
            while let Some(x) = stack.pop() {
                for &y in &nb[x] {
                    if piece[y] == usize::MAX {
                        piece[y] = count;
                        stack.push(y);
                    }
                }
            }
            count += 1;
        }
        if count <= 1 {
            return vec![self.clone()];
        }
        let mut out = Vec::new();
        for k in 0..count {
            out.extend(self.only_piece(&piece, k));
        }
        out
    }

    fn only_piece(&self, piece: &[usize], k: usize) -> Option<Mol> {
        // plain loops: iterator adapters and `retain` closures cost the `.wasm` more
        let n = piece.len();
        let mut new_of_old = vec![0u32; n];
        let mut size = 0u32;
        for i in 0..n {
            if piece[i] == k {
                new_of_old[i] = size;
                size += 1;
            }
        }
        let mut next = size;
        for i in 0..n {
            if piece[i] != k {
                new_of_old[i] = next;
                next += 1;
            }
        }
        let mut m = self.renumbered(&new_of_old).ok()?;
        let s = size as usize;
        m.atoms.truncate(s);
        let mut bonds = Vec::new();
        for b in m.bonds {
            if b.b < size {
                bonds.push(b);
            }
        }
        m.bonds = bonds;
        let mut stereo = Vec::new();
        for x in m.stereo {
            let at = match x {
                Stereo::Tetra { c, .. } => c,
                Stereo::Db { b, .. } => b,
            };
            if at < size {
                stereo.push(x);
            }
        }
        m.stereo = stereo;
        let mut groups = Vec::new();
        for g in m.groups {
            let mut atoms = Vec::new();
            for a in g.atoms {
                if a < size {
                    atoms.push(a);
                }
            }
            if !atoms.is_empty() {
                groups.push(StereoGroup {
                    kind: g.kind,
                    atoms,
                });
            }
        }
        m.groups = groups;
        if let Some(c) = m.coords.as_mut() {
            c.xy.truncate(s);
        }
        for p in &mut m.props {
            p.v.truncate(s);
        }
        let mut regions = Vec::new();
        for r in m.regions {
            if r.atoms.iter().all(|&a| a < size) {
                regions.push(r);
            }
        }
        m.regions = regions;
        m.id = None;
        Some(m)
    }

    /// x, y, z of every atom (zeros without a coordinate layer).
    fn xyz(&self) -> Vec<[f64; 3]> {
        let mut out = vec![[0.0; 3]; self.atoms.len()];
        if let Some(c) = &self.coords {
            for (o, r) in out.iter_mut().zip(&c.xy) {
                for k in 0..r.len().min(3) {
                    o[k] = r[k];
                }
            }
        }
        out
    }

    /// The valence to write per atom: `Some(total)` when the reader's default would give another hydrogen count, `Some(0)` for none at all.
    fn valences(&self) -> Vec<Option<u32>> {
        let mut sum = vec![0u32; self.atoms.len()];
        for b in &self.bonds {
            sum[b.a as usize] += u32::from(b.o);
            sum[b.b as usize] += u32::from(b.o);
        }
        let mut out = vec![None; self.atoms.len()];
        for (i, a) in self.atoms.iter().enumerate() {
            let s = sum[i];
            let default = shortfall(&a.el, i32::from(a.chg), s).saturating_sub(u32::from(a.rad));
            if ORGANIC.contains(&a.el.as_str()) && default != u32::from(a.h) {
                out[i] = Some(s + u32::from(a.h));
            }
        }
        out
    }

    /// The bonds with their marks: one wedge per defined centre, a wavy bond per `unknown` centre, "either" on `unknown` double bonds.
    fn out_bonds(&self, xyz: &[[f64; 3]]) -> Vec<OutBond> {
        let mut out = Vec::with_capacity(self.bonds.len());
        for b in &self.bonds {
            out.push(OutBond {
                a: b.a as usize,
                b: b.b as usize,
                o: b.o,
                mark: Wedge::None,
            });
        }
        let draft = self.draft(xyz, &out);
        let adj = adjacency(&draft);
        let mut centres = Vec::new();
        for s in &self.stereo {
            if let Stereo::Tetra { c, .. } = s {
                centres.push(*c as usize);
            }
        }
        for s in &self.stereo {
            match s {
                Stereo::Tetra {
                    c,
                    par: Some(p),
                    status: Status::Defined,
                    ..
                } => {
                    let c = *c as usize;
                    let mut cands: Vec<(usize, usize)> = adj[c]
                        .iter()
                        .copied()
                        .filter(|&(_, bi)| out[bi].o == 1 && out[bi].mark == Wedge::None)
                        .collect();
                    // insertion sort: few candidates, and a generic sort costs the `.wasm` more than this loop
                    let key = |&(n, _): &(usize, usize)| (centres.contains(&n), adj[n].len(), n);
                    for i in 1..cands.len() {
                        let mut j = i;
                        while j > 0 && key(&cands[j - 1]) > key(&cands[j]) {
                            cands.swap(j - 1, j);
                            j -= 1;
                        }
                    }
                    'pick: for (n, bi) in cands {
                        for w in [Wedge::Up, Wedge::Down] {
                            if parity(&draft, c, &adj[c], &[(n, w)]) == Some(*p) {
                                out[bi] = OutBond {
                                    a: c,
                                    b: n,
                                    o: 1,
                                    mark: w,
                                };
                                break 'pick;
                            }
                        }
                    }
                }
                Stereo::Tetra {
                    c,
                    status: Status::Unknown,
                    ..
                } => {
                    let c = *c as usize;
                    if let Some(&(n, bi)) = adj[c]
                        .iter()
                        .find(|&&(_, bi)| out[bi].o == 1 && out[bi].mark == Wedge::None)
                    {
                        out[bi] = OutBond {
                            a: c,
                            b: n,
                            o: 1,
                            mark: Wedge::Either,
                        };
                    }
                }
                Stereo::Db {
                    a,
                    b,
                    status: Status::Unknown,
                    ..
                } => {
                    if let Some(x) = out
                        .iter_mut()
                        .find(|x| x.a.min(x.b) == *a as usize && x.a.max(x.b) == *b as usize)
                    {
                        x.mark = Wedge::Either;
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The draft the reader would build from these coordinates and bonds, for its parity rule.
    fn draft(&self, xyz: &[[f64; 3]], bonds: &[OutBond]) -> Draft {
        let mut d = Draft {
            atoms: Vec::new(),
            bonds: Vec::new(),
            collections: Vec::new(),
        };
        for (i, a) in self.atoms.iter().enumerate() {
            d.atoms.push(DraftAtom {
                el: a.el.clone(),
                iso: a.iso,
                chg: a.chg,
                rad: a.rad,
                xyz: xyz[i],
                valence: None,
                query: false,
            });
        }
        for b in bonds {
            d.bonds.push(DraftBond {
                a: b.a,
                b: b.b,
                order: b.o,
                wedge: b.mark,
                arom: false,
                query: 0,
                multi_wedge: false,
            });
        }
        d
    }

    fn v2000(&self, s: &mut String, xyz: &[[f64; 3]], bonds: &[OutBond], valence: &[Option<u32>]) {
        let chiral = u8::from(self.stereo.iter().any(|x| {
            matches!(
                x,
                Stereo::Tetra {
                    status: Status::Defined,
                    ..
                }
            )
        }));
        s.push_str(&format!(
            "{:>3}{:>3}  0  0{chiral:>3}  0  0  0  0  0999 V2000\n",
            self.atoms.len(),
            bonds.len()
        ));
        for (i, a) in self.atoms.iter().enumerate() {
            let (p, v) = (
                xyz[i],
                valence[i].map_or(0, |v| if v == 0 { 15 } else { v }),
            );
            s.push_str(&format!(
                "{:>10}{:>10}{:>10} {:<3} 0  0  0  0  0{v:>3}  0  0  0  0  0  0\n",
                fixed(p[0], 4),
                fixed(p[1], 4),
                fixed(p[2], 4),
                a.el
            ));
        }
        for b in bonds {
            let mark = match (b.mark, b.o) {
                (Wedge::Up, _) => 1,
                (Wedge::Down, _) => 6,
                (Wedge::Either, 1) => 4,
                (Wedge::Either, _) => 3,
                (Wedge::None, _) => 0,
            };
            s.push_str(&format!(
                "{:>3}{:>3}{:>3}{mark:>3}  0  0  0\n",
                b.a + 1,
                b.b + 1,
                b.o
            ));
        }
        let prop = |s: &mut String, tag: &str, v: Vec<(u32, i32)>| {
            for chunk in v.chunks(8) {
                s.push_str(&format!("M  {tag}{:>3}", chunk.len()));
                for (a, x) in chunk {
                    s.push_str(&format!(" {:>3} {x:>3}", a + 1));
                }
                s.push('\n');
            }
        };
        let per = |f: &dyn Fn(&crate::model::Atom) -> i32| -> Vec<(u32, i32)> {
            let mut v = Vec::new();
            for (i, a) in self.atoms.iter().enumerate() {
                if f(a) != 0 {
                    v.push((i as u32, f(a)));
                }
            }
            v
        };
        prop(s, "CHG", per(&|a| i32::from(a.chg)));
        prop(s, "ISO", per(&|a| i32::from(a.iso)));
        prop(s, "RAD", per(&|a| radical_code(a.rad)));
    }

    fn v3000(&self, s: &mut String, xyz: &[[f64; 3]], bonds: &[OutBond], valence: &[Option<u32>]) {
        s.push_str("  0  0  0     0  0            999 V3000\n");
        let mut v = |line: String| v30(s, &line);
        v("BEGIN CTAB".into());
        v(format!("COUNTS {} {} 0 0 0", self.atoms.len(), bonds.len()));
        v("BEGIN ATOM".into());
        for (i, a) in self.atoms.iter().enumerate() {
            let (p, val) = (xyz[i], &valence[i]);
            let (x, y, z) = (fixed(p[0], 6), fixed(p[1], 6), fixed(p[2], 6));
            let mut l = format!("{} {} {x} {y} {z} 0", i + 1, a.el);
            if a.chg != 0 {
                l += &format!(" CHG={}", i32::from(a.chg));
            }
            if radical_code(a.rad) != 0 {
                l += &format!(" RAD={}", radical_code(a.rad));
            }
            if a.iso != 0 {
                l += &format!(" MASS={}", u32::from(a.iso));
            }
            if let Some(n) = val {
                l += &format!(" VAL={}", if *n == 0 { -1 } else { *n as i32 });
            }
            v(l);
        }
        v("END ATOM".into());
        if !bonds.is_empty() {
            v("BEGIN BOND".into());
            for (i, b) in bonds.iter().enumerate() {
                let cfg = match b.mark {
                    Wedge::Up => " CFG=1",
                    Wedge::Down => " CFG=3",
                    Wedge::Either => " CFG=2",
                    Wedge::None => "",
                };
                v(format!("{} {} {} {}{cfg}", i + 1, b.o, b.a + 1, b.b + 1));
            }
            v("END BOND".into());
        }
        if !self.groups.is_empty() {
            v("BEGIN COLLECTION".into());
            let (mut rac, mut rel) = (0, 0);
            for g in &self.groups {
                let name = match g.kind {
                    crate::model::GroupKind::Abs => "STEABS".to_string(),
                    crate::model::GroupKind::And => {
                        rac += 1;
                        format!("STERAC{rac}")
                    }
                    crate::model::GroupKind::Or => {
                        rel += 1;
                        format!("STEREL{rel}")
                    }
                };
                let mut l = format!("MDLV30/{name} ATOMS=({}", g.atoms.len());
                for a in &g.atoms {
                    l += &format!(" {}", a + 1);
                }
                v(l + ")");
            }
            v("END COLLECTION".into());
        }
        v("END CTAB".into());
    }
}

/// `M  RAD` / `RAD=` code: 2 for one unpaired electron (doublet), 3 for two (triplet); the reader reads 1 and 3 as two electrons.
fn radical_code(rad: u8) -> i32 {
    match rad {
        1 => 2,
        2 => 3,
        _ => 0,
    }
}

/// `v` with `nd` decimals, rounded half away from zero (integer arithmetic: the precision formatter of `core::fmt` costs the `.wasm` more).
pub fn fixed(v: f64, nd: u32) -> String {
    let p = 10u64.pow(nd);
    let n = (v * p as f64).round() as i64;
    // the sign of the value, as printf writes it: a -0.0 or -0.00001 read from a file goes back as -0.0000
    let sign = if v.is_sign_negative() { "-" } else { "" };
    let a = n.unsigned_abs();
    // two u32 parts: the u64 formatter is not otherwise in the `.wasm`
    format!(
        "{sign}{}.{:0w$}",
        (a / p) as u32,
        (a % p) as u32,
        w = nd as usize
    )
}

/// One `M  V30` line, cut between tokens with `-` continuations so that no line passes 80 characters.
fn v30(s: &mut String, body: &str) {
    let mut line = String::from("M  V30");
    for tok in body.split(' ') {
        if line.len() + 1 + tok.len() > V3000_WIDTH - 1 && line.len() > 6 {
            s.push_str(&line);
            s.push_str(" -\n");
            line = String::from("M  V30");
        }
        line.push(' ');
        line.push_str(tok);
    }
    s.push_str(&line);
    s.push('\n');
}
