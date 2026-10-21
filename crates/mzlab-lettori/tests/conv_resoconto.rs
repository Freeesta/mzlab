//! `conv/1` of every reader that can be built here without laboratory data: the reader named, the version, and each gap declared with its reason.
//! The readers of the other families (Thermo, SCIEX, Bruker) are covered by the data of the private repository (`tests/thermo_dati.rs`, ...).
#![allow(dead_code)]

mod comune;
#[cfg(feature = "waters")]
mod comune_waters;

use mzlab_core::json;
use mzlab_core::lettori::Lettore;
use mzlab_core::lettori_mzml::Conversione;

fn conv_di(l: &mut dyn Lettore) -> json::Value {
    let xml = Conversione::default()
        .scrivi(l, 0, Vec::new(), &mut |_, _| {})
        .unwrap();
    let t = String::from_utf8_lossy(&xml);
    let key = "name=\"mzLab conv/1\" value=\"";
    let v = &t[t.find(key).expect("conv/1 in the run") + key.len()..];
    let v = &v[..v.find("\"/>").unwrap()];
    let v = v
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    json::parse(v.as_bytes()).unwrap()
}

fn controlla(l: &mut dyn Lettore, id: &str) {
    assert_eq!(l.formato().id(), id);
    let doc = conv_di(l);
    assert_eq!(doc.get("schema").and_then(|s| s.str()), Some("conv/1"));
    assert_eq!(doc.get("lettore").and_then(|s| s.str()), Some(id));
    assert!(doc
        .get("versione")
        .and_then(|s| s.str())
        .is_some_and(|v| !v.is_empty()));
    assert!(doc.get("campi_cv").and_then(|n| n.f64()).is_some());
    assert!(doc.get("campi_user").and_then(|n| n.f64()).is_some());
    let declared = l.non_portati();
    let written = doc.get("non_portati").expect("non_portati").arr();
    assert_eq!(
        written.len(),
        declared.len(),
        "{id}: every declared gap is written"
    );
    for (campo, motivo) in declared {
        assert!(
            !campo.is_empty() && !motivo.is_empty(),
            "{id}: a gap without a reason: {campo:?}"
        );
    }
}

#[cfg(feature = "aperti")]
#[test]
fn mzxml_and_numpress_mzml_carry_conv() {
    use comune::mzxml::{apri_mzxml, corsa, mzxml, PIATTO};
    use comune::{apri, mzml_di, scansioni, Cod};
    controlla(&mut *apri_mzxml(mzxml(&corsa(), PIATTO)), "mzxml");
    let np = Cod::Np {
        intensita: mzlab_core::numpress::Np::Slof,
        zlib: true,
    };
    controlla(&mut *apri(mzml_di(&scansioni(), np, None)), "mzml-numpress");
}

#[cfg(feature = "andi")]
#[test]
fn andi_carries_conv() {
    use comune::andi::{andi_ms, apri_andi, PIATTA};
    controlla(&mut *apri_andi(andi_ms(&PIATTA)), "andi-ms");
}

#[cfg(feature = "shimadzu")]
#[test]
fn shimadzu_carries_conv() {
    use comune::shimadzu::qgd;
    use mzlab_core::lettori::{FileSet, Limiti, Membro};
    let b = qgd(6);
    let fs = FileSet {
        membri: vec![Membro {
            nome: "a.qgd".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    controlla(
        &mut *mzlab_lettori::apri_nudo(fs, &Limiti::sito()).unwrap(),
        "shimadzu",
    );
}

#[cfg(feature = "waters")]
#[test]
fn waters_carries_conv() {
    use mzlab_core::lettori::Limiti;
    let mut l =
        mzlab_lettori::apri_nudo(comune_waters::insieme_sintetico(), &Limiti::sito()).unwrap();
    controlla(&mut *l, "waters-raw");
}
