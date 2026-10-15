//! A small OLE2 container writer for the synthetic files of the tests: 512-byte sectors, the mini-stream cutoff at 0 (every stream is a regular chain), a directory that is a chain of
//! siblings per storage. Streams are named by a path (`"Storage/Stream"`); the storages are made as needed.
#![allow(dead_code)]

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const FATSECT: u32 = 0xFFFF_FFFD;
const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

struct Voce {
    nome: String,
    tipo: u8,
    padre: usize,
    start: u32,
    dim: u32,
    figlio: u32,
    destro: u32,
}

fn voce(v: &Voce) -> Vec<u8> {
    let mut e = vec![0u8; 128];
    let u: Vec<u8> = v
        .nome
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    e[..u.len()].copy_from_slice(&u);
    e[0x40..0x42].copy_from_slice(&(u.len() as u16 + 2).to_le_bytes());
    e[0x42] = v.tipo;
    put(&mut e, 0x44, FREE);
    put(&mut e, 0x48, v.destro);
    put(&mut e, 0x4C, v.figlio);
    put(&mut e, 0x74, v.start);
    put(&mut e, 0x78, v.dim);
    e
}

/// The 512-byte header: `nf` FAT sectors from sector 0, the directory at `dir0`, the mini-stream cutoff at 0.
fn intestazione(nf: usize, dir0: usize) -> Vec<u8> {
    let mut h = vec![0u8; 512];
    h[..8].copy_from_slice(&MAGIC);
    h[0x18..0x1A].copy_from_slice(&0x3Eu16.to_le_bytes());
    h[0x1A..0x1C].copy_from_slice(&3u16.to_le_bytes());
    h[0x1C..0x1E].copy_from_slice(&0xFFFEu16.to_le_bytes());
    h[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
    h[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    put(&mut h, 0x2C, nf as u32);
    put(&mut h, 0x30, dir0 as u32);
    put(&mut h, 0x38, 0);
    put(&mut h, 0x3C, END);
    put(&mut h, 0x44, END);
    (0..109).for_each(|i| put(&mut h, 0x4C + 4 * i, if i < nf { i as u32 } else { FREE }));
    h
}

/// The directory entries (the root, then storages and streams in order of appearance) and the bytes of each stream.
fn voci<'a>(flussi: &'a [(&str, Vec<u8>)]) -> (Vec<Voce>, Vec<&'a Vec<u8>>) {
    let mut v = vec![Voce {
        nome: "Root Entry".into(),
        tipo: 5,
        padre: 0,
        start: END,
        dim: 0,
        figlio: FREE,
        destro: FREE,
    }];
    let mut dati: Vec<&Vec<u8>> = Vec::new();
    for (p, b) in flussi {
        let mut padre = 0;
        let parti: Vec<&str> = p.split('/').collect();
        for (i, n) in parti.iter().enumerate() {
            let ultimo = i + 1 == parti.len();
            let t = if ultimo { 2 } else { 1 };
            let es =
                (1..v.len()).find(|&j| v[j].padre == padre && v[j].nome == *n && v[j].tipo == t);
            padre = match es {
                Some(j) => j,
                None => {
                    v.push(Voce {
                        nome: (*n).into(),
                        tipo: t,
                        padre,
                        start: END,
                        dim: 0,
                        figlio: FREE,
                        destro: FREE,
                    });
                    if ultimo {
                        dati.push(b);
                    }
                    v.len() - 1
                }
            };
        }
    }
    (v, dati)
}

/// The container holding `flussi` (path, bytes).
pub fn ole(flussi: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let (mut v, dati) = voci(flussi);
    // siblings: the first child of a parent is its `figlio`, each next one the `destro` of the one before
    let mut ultimo: std::collections::HashMap<usize, usize> = Default::default();
    for j in 1..v.len() {
        let p = v[j].padre;
        match ultimo.insert(p, j) {
            None => v[p].figlio = j as u32,
            Some(prima) => v[prima].destro = j as u32,
        }
    }
    let nd = v.len().div_ceil(4);
    let setts = |n: usize| n.div_ceil(512);
    let ndati: usize = dati.iter().map(|b| setts(b.len())).sum();
    let mut nf = 1;
    while nf * 128 < nf + nd + ndati {
        nf += 1;
    }
    assert!(nf <= 109, "container too large for the test writer");
    // the streams start after the FAT and the directory
    let mut fat = vec![FREE; nf * 128];
    (0..nf).for_each(|i| fat[i] = FATSECT);
    let dir0 = nf;
    (0..nd).for_each(|i| {
        fat[dir0 + i] = if i + 1 < nd {
            (dir0 + i + 1) as u32
        } else {
            END
        }
    });
    let mut at = nf + nd;
    let mut k = 0;
    let mut corpo: Vec<u8> = Vec::new();
    for e in v.iter_mut().skip(1).filter(|e| e.tipo == 2) {
        let b = dati[k];
        k += 1;
        e.dim = b.len() as u32;
        let n = setts(b.len());
        if n > 0 {
            e.start = at as u32;
            (0..n).for_each(|i| fat[at + i] = if i + 1 < n { (at + i + 1) as u32 } else { END });
            corpo.extend(b.iter());
            corpo.resize(corpo.len().div_ceil(512) * 512, 0);
            at += n;
        }
    }
    let h = intestazione(nf, dir0);
    let mut out = h;
    out.extend(fat.iter().flat_map(|x| x.to_le_bytes()));
    let mut d: Vec<u8> = v.iter().flat_map(voce).collect();
    d.resize(nd * 512, 0);
    out.extend(d);
    out.extend(corpo);
    out
}
