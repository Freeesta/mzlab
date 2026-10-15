//! The reader of OLE2 compound files (the container of `.wiff` and `.dam`) against crafted containers: FAT chains that loop, DIFAT chains that make the FAT
//! larger than the file, sectors outside the file, directories that loop, streams of a declared size of 4 GB. Containers are built here, byte by byte.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod robustezza;

use mzlab_core::methodinfo;
use mzlab_core::ole::Ole;
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const FATSECT: u32 = 0xFFFF_FFFD;
const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn entrata(nome: &str, tipo: u8, figlio: u32, start: u32, dim: u32) -> Vec<u8> {
    let mut e = vec![0u8; 128];
    let u: Vec<u8> = nome.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    e[..u.len()].copy_from_slice(&u);
    e[0x40..0x42].copy_from_slice(&(u.len() as u16 + 2).to_le_bytes());
    e[0x42] = tipo;
    put(&mut e, 0x44, FREE);
    put(&mut e, 0x48, FREE);
    put(&mut e, 0x4C, figlio);
    put(&mut e, 0x74, start);
    put(&mut e, 0x78, dim);
    e
}

/// A container of 512-byte sectors: sector 0 is the FAT, sector 1 the directory (a root and the stream `S`), the following ones the data.
/// `fat` overrides the entries of the FAT (index, value); `dim` is the declared size of `S`, which starts at sector 2.
fn contenitore(fat: &[(usize, u32)], dim: u32, dati: usize) -> Vec<u8> {
    let mut b = vec![0u8; 512];
    b[..8].copy_from_slice(&MAGIC);
    b[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
    b[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
    put(&mut b, 0x2C, 1);
    put(&mut b, 0x30, 1);
    put(&mut b, 0x38, 4096);
    put(&mut b, 0x3C, END);
    put(&mut b, 0x44, END);
    (0..109).for_each(|i| put(&mut b, 0x4C + 4 * i, FREE));
    put(&mut b, 0x4C, 0);
    let mut t = vec![FREE; 128];
    t[0] = FATSECT;
    t[1] = END;
    (0..dati).for_each(|i| t[2 + i] = if i + 1 < dati { 3 + i as u32 } else { END });
    fat.iter().for_each(|&(i, v)| t[i] = v);
    b.extend(t.iter().flat_map(|x| x.to_le_bytes()));
    let mut d = entrata("Root Entry", 5, 1, END, 0);
    d.extend(entrata("S", 2, FREE, 2, dim));
    d.resize(512, 0);
    b.extend(d);
    b.extend(vec![0xAB; 512 * dati]);
    b
}

/// What the reader does with a container: open it, read every stream, ask for the methods.
fn legge(b: Vec<u8>) -> Vec<String> {
    let mut k = Vec::new();
    if let Some(ole) = nota(Ole::open(b.clone(), "x.wiff"), &mut k) {
        let percorsi: Vec<String> = ole.paths_list().take(4).map(String::from).collect();
        percorsi.iter().for_each(|p| drop(ole.read(p)));
    }
    #[cfg(feature = "estesi")]
    if let Some(ole) = nota(Ole::open(b.clone(), "x.wiff"), &mut k) {
        // the method document of the readers module: fallback method, GLP tables, DAD header, scheduled-MRM streams
        drop(mzlab_core::metodo_wiff::from_wiff(
            &ole,
            1,
            &Default::default(),
        ));
    }
    nota(methodinfo::read_methods(b, "x.wiff"), &mut k);
    k
}

fn caso(nome: &str, b: Vec<u8>) -> Vec<String> {
    let totale = b.len();
    uno(nome, totale, Box::new(move || legge(b)))
}

#[test]
fn the_valid_container_reads_its_stream() {
    let ole = Ole::open(contenitore(&[], 5000, 10), "x.wiff").unwrap();
    assert_eq!(ole.read("S").map(|s| s.len()), Some(5000));
}

#[test]
fn chains_that_loop_sectors_outside_the_file_and_sizes_of_4_gb() {
    let mut g = Vec::new();
    g.extend(caso(
        "data chain loops on itself",
        contenitore(&[(2, 2)], 5000, 2),
    ));
    g.extend(caso(
        "data chain loops back",
        contenitore(&[(3, 2)], 5000, 2),
    ));
    g.extend(caso(
        "directory chain loops",
        contenitore(&[(1, 1)], 5000, 2),
    ));
    g.extend(caso(
        "FAT points at itself",
        contenitore(&[(2, 0), (3, 0)], 5000, 2),
    ));
    g.extend(caso(
        "chain to a sector outside the file",
        contenitore(&[(2, 0x00FF_FFFF)], 5000, 2),
    ));
    g.extend(caso(
        "chain to 0xFFFFFFFC",
        contenitore(&[(2, 0xFFFF_FFFC)], 5000, 2),
    ));
    g.extend(caso("size 4 GB", contenitore(&[], u32::MAX, 2)));
    g.extend(caso("size 4 GB, loop", contenitore(&[(2, 2)], u32::MAX, 2)));
    g.extend(caso("size 0 and a start outside", contenitore(&[], 0, 0)));
    verifica("cfb", 9, &g);
}

/// A DIFAT chain of `k` sectors, all of 127 entries that point to the FAT sector: the FAT grows by 16 000 entries per 512 bytes of file, and a chain that loops
/// in it (here the directory) is followed for as many sectors as the FAT has entries.
fn con_difat(k: usize) -> Vec<u8> {
    let mut b = contenitore(&[(1, 1)], 5000, 2);
    put(&mut b, 0x2C, 109 + 127 * k as u32);
    (0..109).for_each(|i| put(&mut b, 0x4C + 4 * i, 0));
    put(&mut b, 0x44, 4);
    put(&mut b, 0x48, k as u32);
    for i in 0..k {
        let mut s = vec![0u8; 512];
        (0..127).for_each(|j| put(&mut s, 4 * j, 0));
        put(&mut s, 508, if i + 1 < k { (5 + i) as u32 } else { END });
        b.extend(s);
    }
    b
}

#[test]
fn a_difat_that_makes_the_fat_larger_than_the_file() {
    verifica("difat", 1, &caso("difat 10 sectors", con_difat(10)));
}
