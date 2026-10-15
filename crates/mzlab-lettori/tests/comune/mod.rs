//! The synthetic mzML files and the helpers shared by the tests of the module `aperti` (numpress).
#![allow(dead_code)]

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::lettori_mzml;
use mzlab_core::numpress::{self, Np};
use std::io::Write;

pub fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

pub fn zlib(b: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

/// How the arrays of the file are written.
#[derive(Clone, Copy)]
pub enum Cod {
    /// 64-bit floats and zlib: the twin of the numpress file.
    Piano,
    /// Linear prediction for m/z, `intensita` for the intensities, with or without zlib after.
    Np { intensita: Np, zlib: bool },
}

/// The kind of an array: its term and whether the codec of the m/z axis (linear prediction) is the one that applies.
#[derive(Clone, Copy, PartialEq)]
pub enum Genere {
    Mz,
    Intensita,
    Tempo,
}

pub fn array(v: &[f64], cod: Cod, g: Genere) -> String {
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
pub struct Scan {
    pub mz: Vec<f64>,
    pub intensita: Vec<f64>,
    pub livello: u8,
    pub rt_min: f64,
    pub precursore: Option<f64>,
}

pub fn scansioni() -> Vec<Scan> {
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

pub const TESTATA: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<indexedmzML xmlns=\"http://psi.hupo.org/ms/mzml\"><mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\">\n\
<cvList count=\"1\"><cv id=\"MS\" fullName=\"x\" URI=\"x\"/></cvList>\n\
<fileDescription><fileContent><cvParam cvRef=\"MS\" accession=\"MS:1000579\" name=\"MS1 spectrum\"/><cvParam cvRef=\"MS\" accession=\"MS:1000580\" name=\"MSn spectrum\"/></fileContent>\
<sourceFileList count=\"1\"><sourceFile id=\"RAW1\" name=\"campione.raw\" location=\"file:///x\"><cvParam cvRef=\"MS\" accession=\"MS:1000563\" name=\"Thermo RAW format\"/><cvParam cvRef=\"MS\" accession=\"MS:1000569\" name=\"SHA-1\" value=\"0123456789abcdef0123456789abcdef01234567\"/></sourceFile></sourceFileList></fileDescription>\n\
<referenceableParamGroupList count=\"1\"><referenceableParamGroup id=\"CommonInstrumentParams\"><cvParam cvRef=\"MS\" accession=\"MS:1001911\" name=\"Q Exactive\"/><cvParam cvRef=\"MS\" accession=\"MS:1000529\" name=\"instrument serial number\" value=\"SN-4711\"/></referenceableParamGroup></referenceableParamGroupList>\n\
<softwareList count=\"1\"><software id=\"pwiz\" version=\"3.0.1\"><cvParam cvRef=\"MS\" accession=\"MS:1000615\" name=\"ProteoWizard software\"/></software></softwareList>\n\
<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\"><referenceableParamGroupRef ref=\"CommonInstrumentParams\"/><componentList count=\"3\"><source order=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000073\" name=\"electrospray ionization\"/></source><analyzer order=\"2\"><cvParam cvRef=\"MS\" accession=\"MS:1000484\" name=\"orbitrap\"/></analyzer><detector order=\"3\"><cvParam cvRef=\"MS\" accession=\"MS:1000624\" name=\"inductive detector\"/></detector></componentList></instrumentConfiguration></instrumentConfigurationList>\n\
<run id=\"campione\" defaultInstrumentConfigurationRef=\"IC1\" startTimeStamp=\"2024-03-05T10:11:12Z\">\n";

/// The mzML: spectra, then a TIC chromatogram whose arrays are written as the spectra's. `metodo`: the userParam that carries a `metodo/1` document.
pub fn mzml_di(scan: &[Scan], cod: Cod, metodo: Option<&str>) -> Vec<u8> {
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

pub fn insieme(nome: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
        }],
    }
}

pub fn apri(byte: Vec<u8>) -> Box<dyn Lettore> {
    mzlab_lettori::apri(insieme("campione.mzML", byte), &Limiti::sito()).unwrap()
}

pub fn errore(byte: Vec<u8>) -> &'static str {
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

pub fn mzml_scritto(l: &mut Box<dyn Lettore>) -> Vec<u8> {
    lettori_mzml::scrivi(l.as_mut(), 0, Vec::new(), &mut |_, _| {}).unwrap()
}

pub mod andi;
pub mod mzxml;
