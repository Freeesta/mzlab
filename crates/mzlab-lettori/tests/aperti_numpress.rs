//! The module `aperti` reads an mzML with MS-Numpress arrays (docs/agenti/lettori-aperti.md): files built here with the encoders of `mzlab_core::numpress`, hostile ones, and (with the
//! private data repository, `MZLAB_DATI`) the numpress files of OpenMS and pymzML. Without it the real-data tests are skipped.
#![cfg(feature = "aperti")]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::io::Write;
use std::path::PathBuf;

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::lettori_mzml;
use mzlab_core::mzml;
use mzlab_core::numpress::{self, Np};

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn zlib(b: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

/// How the arrays of the file are written.
#[derive(Clone, Copy)]
enum Cod {
    /// 64-bit floats and zlib: the twin of the numpress file.
    Piano,
    /// Linear prediction for m/z, `intensita` for the intensities, with or without zlib after.
    Np { intensita: Np, zlib: bool },
}

/// The kind of an array: its term and whether the codec of the m/z axis (linear prediction) is the one that applies.
#[derive(Clone, Copy, PartialEq)]
enum Genere {
    Mz,
    Intensita,
    Tempo,
}

fn array(v: &[f64], cod: Cod, g: Genere) -> String {
    let (accessions, dati): (String, Vec<u8>) = match cod {
        Cod::Piano => (
            "<cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\"/>".into(),
            zlib(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()),
        ),
        Cod::Np { intensita, zlib: z } => {
            let (np, bytes) = match (g, intensita) {
                (Genere::Intensita, Np::Pic) => (Np::Pic, numpress::encode_pic(v)),
                (Genere::Intensita, Np::Slof) => (Np::Slof, numpress::encode_slof(v, numpress::slof_fixed_point(v))),
                _ => (Np::Linear, numpress::encode_linear(v, numpress::linear_fixed_point(v))),
            };
            let acc = match (np, z) {
                (Np::Linear, false) => "MS:1002312",
                (Np::Pic, false) => "MS:1002313",
                (Np::Slof, false) => "MS:1002314",
                (Np::Linear, true) => "MS:1002746",
                (Np::Pic, true) => "MS:1002747",
                (Np::Slof, true) => "MS:1002748",
            };
            (
                format!("<cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"numpress\"/>"),
                if z { zlib(&bytes) } else { bytes },
            )
        }
    };
    let kind = match g {
        Genere::Mz => "<cvParam cvRef=\"MS\" accession=\"MS:1000514\" name=\"m/z array\"/>",
        Genere::Intensita => "<cvParam cvRef=\"MS\" accession=\"MS:1000515\" name=\"intensity array\"/>",
        Genere::Tempo => "<cvParam cvRef=\"MS\" accession=\"MS:1000595\" name=\"time array\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>",
    };
    format!("<binaryDataArray encodedLength=\"0\">{accessions}{kind}<binary>{}</binary></binaryDataArray>\n", b64(&dati))
}

/// One scan of the synthetic run.
struct Scan {
    mz: Vec<f64>,
    intensita: Vec<f64>,
    livello: u8,
    rt_min: f64,
    precursore: Option<f64>,
}

fn scansioni() -> Vec<Scan> {
    (0..6)
        .map(|k| {
            let n = 40 + 7 * k;
            Scan {
                mz: (0..n)
                    .map(|i| 100.0 + f64::from(i as u32) * 3.17 + f64::from(k as u32) * 0.013)
                    .collect(),
                intensita: (0..n)
                    .map(|i| 50.0 + f64::from(((i * 7919 + k * 31) % 997) as u32) * 11.0)
                    .collect(),
                livello: if k % 3 == 2 { 2 } else { 1 },
                rt_min: 0.5 + k as f64 * 0.25,
                precursore: (k % 3 == 2).then_some(412.25 + k as f64),
            }
        })
        .collect()
}

const TESTATA: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<indexedmzML xmlns=\"http://psi.hupo.org/ms/mzml\"><mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\">\n\
<cvList count=\"1\"><cv id=\"MS\" fullName=\"x\" URI=\"x\"/></cvList>\n\
<fileDescription><fileContent><cvParam cvRef=\"MS\" accession=\"MS:1000579\" name=\"MS1 spectrum\"/><cvParam cvRef=\"MS\" accession=\"MS:1000580\" name=\"MSn spectrum\"/></fileContent>\
<sourceFileList count=\"1\"><sourceFile id=\"RAW1\" name=\"campione.raw\" location=\"file:///x\"><cvParam cvRef=\"MS\" accession=\"MS:1000563\" name=\"Thermo RAW format\"/><cvParam cvRef=\"MS\" accession=\"MS:1000569\" name=\"SHA-1\" value=\"0123456789abcdef0123456789abcdef01234567\"/></sourceFile></sourceFileList></fileDescription>\n\
<referenceableParamGroupList count=\"1\"><referenceableParamGroup id=\"CommonInstrumentParams\"><cvParam cvRef=\"MS\" accession=\"MS:1001911\" name=\"Q Exactive\"/><cvParam cvRef=\"MS\" accession=\"MS:1000529\" name=\"instrument serial number\" value=\"SN-4711\"/></referenceableParamGroup></referenceableParamGroupList>\n\
<softwareList count=\"1\"><software id=\"pwiz\" version=\"3.0.1\"><cvParam cvRef=\"MS\" accession=\"MS:1000615\" name=\"ProteoWizard software\"/></software></softwareList>\n\
<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\"><referenceableParamGroupRef ref=\"CommonInstrumentParams\"/><componentList count=\"3\"><source order=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000073\" name=\"electrospray ionization\"/></source><analyzer order=\"2\"><cvParam cvRef=\"MS\" accession=\"MS:1000484\" name=\"orbitrap\"/></analyzer><detector order=\"3\"><cvParam cvRef=\"MS\" accession=\"MS:1000624\" name=\"inductive detector\"/></detector></componentList></instrumentConfiguration></instrumentConfigurationList>\n\
<run id=\"campione\" defaultInstrumentConfigurationRef=\"IC1\" startTimeStamp=\"2024-03-05T10:11:12Z\">\n";

/// The mzML: spectra, then a TIC chromatogram whose arrays are written as the spectra's. `metodo`: the userParam that carries a `metodo/1` document.
fn mzml_di(scan: &[Scan], cod: Cod, metodo: Option<&str>) -> Vec<u8> {
    let mut s = String::from(TESTATA);
    if let Some(m) = metodo {
        s += &format!(
            "<userParam name=\"mzLab metodo/1\" value=\"{}\"/>\n",
            m.replace('"', "&quot;")
        );
    }
    s += &format!("<spectrumList count=\"{}\">\n", scan.len());
    for (k, x) in scan.iter().enumerate() {
        s += &format!(
            "<spectrum index=\"{k}\" id=\"controllerType=0 controllerNumber=1 scan={}\" defaultArrayLength=\"{}\">\
<cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{}\"/><cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000130\" name=\"positive scan\"/><cvParam cvRef=\"MS\" accession=\"MS:1000285\" name=\"total ion current\" value=\"{}\"/>\n\
<scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>\
<scanWindowList count=\"1\"><scanWindow><cvParam cvRef=\"MS\" accession=\"MS:1000501\" name=\"scan window lower limit\" value=\"100\"/><cvParam cvRef=\"MS\" accession=\"MS:1000500\" name=\"scan window upper limit\" value=\"1000\"/></scanWindow></scanWindowList></scan></scanList>\n",
            k + 1,
            x.mz.len(),
            x.livello,
            x.intensita.iter().sum::<f64>(),
            x.rt_min
        );
        if let Some(p) = x.precursore {
            s += &format!("<precursorList count=\"1\"><precursor><isolationWindow><cvParam cvRef=\"MS\" accession=\"MS:1000827\" name=\"isolation window target m/z\" value=\"{p}\"/></isolationWindow><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{p}\"/></selectedIon></selectedIonList><activation><cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"35\"/></activation></precursor></precursorList>\n");
        }
        s += &format!(
            "<binaryDataArrayList count=\"2\">\n{}{}</binaryDataArrayList></spectrum>\n",
            array(&x.mz, cod, Genere::Mz),
            array(&x.intensita, cod, Genere::Intensita)
        );
    }
    s += "</spectrumList>\n";
    let t: Vec<f64> = scan.iter().map(|x| x.rt_min).collect();
    let y: Vec<f64> = scan.iter().map(|x| x.intensita.iter().sum()).collect();
    s += &format!(
        "<chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"TIC\" defaultArrayLength=\"{}\"><cvParam cvRef=\"MS\" accession=\"MS:1000235\" name=\"total ion current chromatogram\"/><binaryDataArrayList count=\"2\">\n",
        t.len()
    );
    s += &array(&t, cod, Genere::Tempo);
    s += &array(&y, cod, Genere::Intensita);
    s += "</binaryDataArrayList></chromatogram></chromatogramList></run></mzML></indexedmzML>\n";
    s.into_bytes()
}

fn insieme(nome: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
        }],
    }
}

fn apri(byte: Vec<u8>) -> Box<dyn Lettore> {
    mzlab_lettori::apri(insieme("campione.mzML", byte), &Limiti::sito()).unwrap()
}

fn errore(byte: Vec<u8>) -> &'static str {
    match mzlab_lettori::apri(insieme("campione.mzML", byte), &Limiti::sito()) {
        Err(e) => e.key,
        Ok(mut l) => {
            // a hostile array is found when the scan is read
            for k in 0..l.indice(0).map(|i| i.len()).unwrap_or(0) {
                if let Err(e) = l.scansione(0, k) {
                    return e.key;
                }
            }
            "ok"
        }
    }
}

fn mzml_scritto(l: &mut Box<dyn Lettore>) -> Vec<u8> {
    lettori_mzml::scrivi(l.as_mut(), 0, Vec::new(), &mut |_, _| {}).unwrap()
}

#[test]
fn ogni_codec_si_apre_e_torna_entro_la_sua_precisione() {
    let scan = scansioni();
    for (intensita, z) in [
        (Np::Slof, false),
        (Np::Slof, true),
        (Np::Pic, false),
        (Np::Pic, true),
    ] {
        let mut l = apri(mzml_di(&scan, Cod::Np { intensita, zlib: z }, None));
        assert_eq!(l.campioni()[0].scansioni, scan.len());
        for (k, x) in scan.iter().enumerate() {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(s.mz.len(), x.mz.len());
            let fp = numpress::linear_fixed_point(&x.mz);
            for (a, b) in s.mz.iter().zip(&x.mz) {
                assert!((a - b).abs() <= 0.5 / fp + 1e-9, "mz {a} {b}");
            }
            for i in 0..s.mz.len() {
                let (a, b) = (s.intensity_f64(i), x.intensita[i]);
                let tol = match intensita {
                    Np::Pic => 0.5,
                    Np::Slof => {
                        (b + 1.0) * ((0.5 / numpress::slof_fixed_point(&x.intensita)).exp() - 1.0)
                            + 1e-6
                    }
                    Np::Linear => 0.5 / numpress::linear_fixed_point(&x.intensita) + 1e-6,
                };
                assert!((a - b).abs() <= tol, "{intensita:?} {z} intensity {a} {b}");
            }
            assert_eq!(s.level, x.livello);
            assert_eq!(s.precursor, x.precursore);
            assert!((s.rt - x.rt_min).abs() < 1e-9);
        }
    }
}

#[test]
fn l_indice_dice_livello_precursore_tic_senza_leggere_i_picchi() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let ind = l.indice(0).unwrap();
    assert_eq!(ind.len(), scan.len());
    for (v, x) in ind.iter().zip(&scan) {
        assert_eq!(v.livello, x.livello);
        assert_eq!(v.precursore, x.precursore);
        assert_eq!(v.polarita, 1);
        assert!((v.tic - x.intensita.iter().sum::<f64>()).abs() < 1e-6);
        assert!(v.id.starts_with("controllerType=0"));
        assert!(v.posizione.is_some_and(|p| p.lunghezza > 0));
    }
}

#[test]
fn riscritto_come_mzml_zlib_senza_numpress_e_uguale_al_gemello() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: true,
        },
        None,
    ));
    let scritto = mzml_scritto(&mut np);
    let testo = String::from_utf8_lossy(&scritto).to_string();
    for a in [
        "MS:1002312",
        "MS:1002313",
        "MS:1002314",
        "MS:1002746",
        "MS:1002747",
        "MS:1002748",
    ] {
        assert!(!testo.contains(a), "{a} still in the output");
    }
    let run = mzml::read(&scritto[..]).unwrap();
    let gemello = mzml::read(&mzml_di(&scan, Cod::Piano, None)[..]).unwrap();
    assert_eq!(run.spectra.len(), gemello.spectra.len());
    for (a, b) in run.spectra.iter().zip(&gemello.spectra) {
        assert_eq!(
            (a.level, a.polarity, a.precursor, a.profile),
            (b.level, b.polarity, b.precursor, b.profile)
        );
        assert_eq!(a.id, b.id);
        assert!((a.rt - b.rt).abs() < 1e-9);
        assert_eq!(a.mz.len(), b.mz.len());
        for i in 0..a.mz.len() {
            assert!((a.mz[i] - b.mz[i]).abs() < 1e-4);
            let (x, y) = (a.intensity_f64(i), b.intensity_f64(i));
            assert!((x - y).abs() <= y * 1e-3 + 1e-3, "{x} {y}");
        }
    }
    // the TIC chromatogram is decoded too (numpress arrays after the spectra)
    assert_eq!(run.chromatograms.len(), 1);
    assert_eq!(run.chromatograms[0].time.len(), scan.len());
    // the scan window and the collision energy of the MS2 survive
    assert!(testo.contains("scan window lower limit"));
    assert!(testo.contains("MS:1000045 collision energy"));
}

#[test]
fn la_testata_originale_resta_e_mzlab_scrive() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let t = String::from_utf8_lossy(&mzml_scritto(&mut np)).to_string();
    assert!(t.contains("MS:1001911"), "instrument model");
    assert!(t.contains("SN-4711"), "serial number");
    assert!(
        t.contains("campione.raw") && t.contains("0123456789abcdef0123456789abcdef01234567"),
        "source and SHA-1"
    );
    assert!(t.contains("MS:1000563"), "source format");
    assert!(
        t.contains("ProteoWizard software") && t.contains("3.0.1"),
        "original software"
    );
    assert!(t.contains("startTimeStamp=\"2024-03-05T10:11:12Z\""));
    assert!(
        t.contains("electrospray ionization")
            && t.contains("orbitrap")
            && t.contains("inductive detector")
    );
    assert!(
        t.contains("<software id=\"mzlab\"") && t.contains("mzLab"),
        "mzLab as the converter"
    );
    assert!(
        !t.contains("mzLab metodo/1"),
        "no document when the original had none"
    );
}

#[test]
fn il_metodo_si_ripete_solo_se_l_mzml_lo_porta() {
    let scan = scansioni();
    let doc = "{\"schema\":\"metodo/1\",\"strumento\":{\"modello\":\"x\"}}";
    let mut con = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: false,
        },
        Some(doc),
    ));
    assert_eq!(con.metodo(0).unwrap(), doc);
    let t = String::from_utf8_lossy(&mzml_scritto(&mut con)).to_string();
    assert!(t.contains("mzLab metodo/1"));
    let mut senza = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: false,
        },
        None,
    ));
    assert_eq!(senza.metodo(0).unwrap(), "{}");
}

#[test]
fn un_mzml_senza_numpress_o_un_altro_file_non_e_suo() {
    let scan = scansioni();
    let piano = mzml_di(&scan, Cod::Piano, None);
    assert_eq!(errore(piano), "err.file.notAccepted");
    assert_eq!(
        errore(b"<?xml version=\"1.0\"?><x/>".to_vec()),
        "err.file.notAccepted"
    );
    assert_eq!(errore(vec![0xAB; 4096]), "err.file.notAccepted");
}

#[test]
fn i_cromatogrammi_numpress_si_leggono() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Linear,
            zlib: false,
        },
        None,
    ));
    let v = l.cromatogrammi(0).unwrap();
    assert_eq!(v.len(), 1);
    let c = l.cromatogramma(0, &v[0].id).unwrap();
    assert_eq!(c.time.len(), scan.len());
    assert!((c.time[1] - 0.75).abs() < 1e-3);
}

// ---- hostile files ----

#[test]
fn piu_valori_di_quelli_dichiarati_e_un_errore() {
    let scan = scansioni();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    b = b.replacen("defaultArrayLength=\"40\"", "defaultArrayLength=\"3\"", 1);
    assert_eq!(errore(b.into_bytes()), "err.file.numpress");
}

#[test]
fn uno_zlib_che_si_gonfia_oltre_il_dichiarato_e_un_errore() {
    let scan = scansioni();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: true,
        },
        None,
    ))
    .unwrap();
    // an array of 40 values inflates to a few hundred bytes at most; this stream inflates to 8 MB of zeros
    let bomba = b64(&zlib(&vec![0u8; 8 << 20]));
    let da = b.find("<binary>").unwrap() + 8;
    let a = da + b[da..].find("</binary>").unwrap();
    b.replace_range(da..a, &bomba);
    assert_eq!(errore(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn base64_rotta_e_dati_troncati_sono_errori_con_chiave() {
    let scan = scansioni();
    let ok = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    let da = ok.find("<binary>").unwrap() + 8;
    let rotta = format!("{}!!!{}", &ok[..da], &ok[da + 3..]);
    assert_eq!(errore(rotta.into_bytes()), "err.file.base64");
    // a linear array cut to 5 bytes: shorter than its fixed point
    let a = da + ok[da..].find("</binary>").unwrap();
    let corta = format!("{}{}{}", &ok[..da], b64(&[1, 2, 3, 4, 5]), &ok[a..]);
    assert_eq!(errore(corta.into_bytes()), "err.file.numpress");
}

#[test]
fn un_file_troncato_non_manda_in_panico() {
    let scan = scansioni();
    let ok = mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: true,
        },
        None,
    );
    for n in [0, 10, 700, 2500, ok.len() / 2, ok.len() - 5] {
        let _ = errore(ok[..n].to_vec());
    }
}

#[test]
fn righe_oltre_il_limite_sono_un_errore() {
    let scan = scansioni();
    let limiti = Limiti {
        voci: 2,
        ..Limiti::sito()
    };
    let r = mzlab_lettori::apri(
        insieme(
            "campione.mzML",
            mzml_di(
                &scan,
                Cod::Np {
                    intensita: Np::Pic,
                    zlib: false,
                },
                None,
            ),
        ),
        &limiti,
    );
    assert_eq!(r.err().map(|e| e.key), Some("error.memoryBudget"));
}

// ---- real files (the private data repository) ----

fn dati() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let qui = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in qui.ancestors().skip(1).take(3) {
        c.push(up.join("mzlab-dati"));
    }
    if let Some(h) = std::env::var_os("HOME") {
        c.push(PathBuf::from(h).join("mzlab-dati"));
    }
    c.into_iter()
        .find(|p| p.join("dati_formati/INDICE.md").exists())
}

fn vero(nome: &str) -> Option<Vec<u8>> {
    let Some(d) = dati() else {
        eprintln!("SKIP aperti_numpress {nome}: no data repository (MZLAB_DATI)");
        return None;
    };
    std::fs::read(d.join("dati_formati/mzML_numpress").join(nome)).ok()
}

/// The TIC an mzML header states against the sum of the decoded intensities.
#[test]
fn openms_lineare_e_slof_hanno_il_tic_dell_intestazione() {
    let Some(b) = vero("OpenMS_FileFilter_37_output.mzML") else {
        return;
    };
    let mut l = apri(b);
    let n = l.campioni()[0].scansioni;
    assert_eq!(n, 10);
    for k in 0..n {
        let s = l.scansione(0, k).unwrap();
        assert!(!s.mz.is_empty());
        assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "m/z sorted");
        assert!(s.intensity.iter().all(|x| *x >= 0.0));
        let tic = l.indice(0).unwrap()[k].tic;
        if tic > 0.0 {
            let somma: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
            assert!(
                (somma - tic).abs() <= tic * 1e-3,
                "scan {k}: sum {somma} TIC {tic}"
            );
        }
    }
    let t = String::from_utf8_lossy(&mzml_scritto(&mut l)).to_string();
    assert!(!t.contains("MS:1002312"));
}

#[test]
fn openms_lineare_e_pic_si_leggono() {
    let Some(b) = vero("OpenMS_FileFilter_38_output.mzML") else {
        return;
    };
    let mut l = apri(b);
    for k in 0..l.campioni()[0].scansioni {
        let s = l.scansione(0, k).unwrap();
        assert!(!s.mz.is_empty() && s.mz.windows(2).all(|w| w[0] <= w[1]));
        // pic gives whole numbers
        assert!(s.intensity.iter().all(|x| x.fract() == 0.0));
    }
}

#[test]
fn pymzml_cromatogrammi_srm_numpress() {
    let Some(b) = vero("pymzML_mini_numpress.chrom.mzML") else {
        return;
    };
    let mut l = apri(b);
    assert_eq!(l.campioni()[0].scansioni, 0);
    let v = l.cromatogrammi(0).unwrap();
    assert!(!v.is_empty());
    for c in v {
        let t = l.cromatogramma(0, &c.id).unwrap();
        assert_eq!(t.time.len(), t.intensity.len());
        assert!(!t.time.is_empty(), "{}", c.id);
        assert!(t.time.windows(2).all(|w| w[0] <= w[1]));
    }
}
