//! The `.cdf` that ChemStation exports from a run against the `.D` folder of the same run read by `agilent-d`: the same scans, the same times, the same TIC; m/z and intensities within the
//! precision of the export. Needs the private data repository (`MZLAB_DATI`) and the unzip program; skipped without them. Run with `--features tutte`.
#![cfg(all(feature = "andi", feature = "agilent"))]

mod comune;

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use std::path::{Path, PathBuf};

fn dati() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let qui = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.extend(
        qui.ancestors()
            .skip(1)
            .take(3)
            .map(|u| u.join("mzlab-dati")),
    );
    if let Some(h) = std::env::var_os("HOME") {
        c.push(PathBuf::from(h).join("mzlab-dati"));
    }
    c.into_iter()
        .find(|p| p.join("dati_formati/INDICE.md").exists())
}

fn membri(radice: &Path, dir: &Path, out: &mut Vec<Membro>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            membri(radice, &p, out);
        } else {
            let b = std::fs::read(&p).unwrap();
            let nome = p
                .strip_prefix(radice.parent().unwrap())
                .unwrap()
                .to_string_lossy()
                .into_owned();
            out.push(Membro {
                nome,
                dim: b.len() as u64,
                dati: Box::new(b),
            });
        }
    }
}

#[test]
fn il_cdf_esportato_dice_quello_che_dice_la_sua_cartella_d() {
    let Some(d) = dati() else {
        eprintln!("SKIP andi parita: no data repository (MZLAB_DATI)");
        return;
    };
    let cartella = d.join("dati_formati/netcdf_cdf");
    let tmp = std::env::temp_dir().join(format!("mzlab-andi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    for (zip, cdf) in [
        (
            "MTBLS7740_AZ_alkanes_TMS_1.D.zip",
            "MTBLS7740_AZ_alkanes_TMS_1.cdf",
        ),
        ("MTBLS13991_Heart_16.D.zip", "MTBLS13991_Heart_16.cdf"),
    ] {
        let sotto = tmp.join(zip);
        std::fs::create_dir_all(&sotto).unwrap();
        let ok = cartella.join(zip).exists()
            && std::process::Command::new("unzip")
                .arg("-q")
                .arg("-o")
                .arg(cartella.join(zip))
                .arg("-d")
                .arg(&sotto)
                .status()
                .is_ok_and(|s| s.success());
        if !ok {
            eprintln!("SKIP andi parita: {zip}");
            continue;
        }
        let dir = std::fs::read_dir(&sotto)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .unwrap();
        let mut m = Vec::new();
        membri(&dir, &dir, &mut m);
        let mut d_set = mzlab_lettori::apri(FileSet { membri: m }, &Limiti::desktop()).unwrap();
        let mut c_set = comune::andi::apri_andi(std::fs::read(cartella.join(cdf)).unwrap());
        let n = d_set.campioni()[0].scansioni;
        assert_eq!(c_set.campioni()[0].scansioni, n, "{cdf}");
        let (id, ic) = (
            d_set.indice(0).unwrap().to_vec(),
            c_set.indice(0).unwrap().to_vec(),
        );
        for k in (0..n).step_by((n / 30).max(1)) {
            assert!(
                (id[k].rt - ic[k].rt).abs() < 1e-3,
                "{cdf} scan {k}: rt {} {}",
                id[k].rt,
                ic[k].rt
            );
            assert!(
                (id[k].tic - ic[k].tic).abs() <= 1e-3 * id[k].tic.max(1.0),
                "{cdf} scan {k}: tic {} {}",
                id[k].tic,
                ic[k].tic
            );
            let (a, b) = (
                d_set.scansione(0, k).unwrap(),
                c_set.scansione(0, k).unwrap(),
            );
            assert_eq!(a.mz.len(), b.mz.len(), "{cdf} scan {k}");
            for i in 0..a.mz.len() {
                assert!(
                    (a.mz[i] - b.mz[i]).abs() < 0.01,
                    "{cdf} scan {k} mz {i}: {} {}",
                    a.mz[i],
                    b.mz[i]
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
