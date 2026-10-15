//! What the engine gave for a Thermo `.raw` while the reader was linked in it, for tests_e2e/e2e_thermo_modulo.py: `raw_referenza <file.raw> <out.mzML>` writes the mzML as «Download .mzML» does
//! (`lettori_mzml::scrivi` over `rawlettore`) and prints the JSON `{scans, tic: {rt, y}, scan: {k, id, mz, y}}`: the TIC of the MS1 scans (from the index) and the scan in the middle of the index.
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::{json::Out, lettori_mzml, rawlettore};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (src, dst) = (a.get(1).expect("file.raw"), a.get(2).expect("out.mzML"));
    let bytes = std::fs::read(src).expect("read");
    let nome = std::path::Path::new(src)
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().to_string());
    let fs = FileSet {
        membri: vec![Membro {
            nome,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    };
    let mut l = (rawlettore::VOCE.apri)(fs, &Limiti::sito()).expect("open");
    let index = l.indice(0).expect("index").to_vec();
    let n = index.len();
    let k = n / 2;
    let s = l.scansione(0, k).expect("scan");
    let ms1: Vec<_> = index.iter().filter(|r| r.livello == 1).collect();
    let (rt, tic): (Vec<f64>, Vec<f64>) = ms1.iter().map(|r| (r.rt, r.tic)).unzip();
    let mut o = Out::default();
    o.raw("{\"scans\":")
        .int(n as i64)
        .raw(",\"tic\":{\"rt\":")
        .floats(&rt)
        .raw(",\"y\":")
        .floats(&tic);
    o.raw("},\"scan\":{\"k\":")
        .int(k as i64)
        .raw(",\"id\":")
        .str(&s.id)
        .raw(",\"mz\":")
        .floats(&s.mz);
    let y: Vec<f64> = (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect();
    o.raw(",\"y\":").floats(&y).raw("}}");
    let out = std::fs::File::create(dst).expect("create");
    lettori_mzml::scrivi(&mut *l, 0, std::io::BufWriter::new(out), &mut |_, _| {}).expect("mzML");
    println!("{}", o.0);
}
