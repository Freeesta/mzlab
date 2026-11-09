//! `conv/1` of every reader that can be built here without laboratory data: the reader named, the version, and each gap declared with its reason.
//! The readers of the other families (Thermo, SCIEX, Bruker) are covered by the data of the private repository (`tests/thermo_data.rs`, ...).
#![allow(dead_code)]

mod common;
#[cfg(feature = "waters")]
mod common_waters;

use mzlab_core::json;
use mzlab_core::lettori::Reader;
use mzlab_core::lettori_mzml::Conversion;

fn conv_di(l: &mut dyn Reader) -> json::Value {
    let xml = Conversion::default()
        .write(l, 0, Vec::new(), &mut |_, _| {})
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

fn check(l: &mut dyn Reader, id: &str) {
    assert_eq!(l.format().id(), id);
    let doc = conv_di(l);
    assert_eq!(doc.get("schema").and_then(|s| s.str()), Some("conv/1"));
    assert_eq!(doc.get("lettore").and_then(|s| s.str()), Some(id));
    assert!(doc
        .get("versione")
        .and_then(|s| s.str())
        .is_some_and(|v| !v.is_empty()));
    assert!(doc.get("campi_cv").and_then(|n| n.f64()).is_some());
    assert!(doc.get("campi_user").and_then(|n| n.f64()).is_some());
    let declared = l.not_carried();
    let written = doc.get("non_portati").expect("non_portati").arr();
    assert_eq!(
        written.len(),
        declared.len(),
        "{id}: every declared gap is written"
    );
    for (field, reason) in declared {
        assert!(
            !field.is_empty() && !reason.is_empty(),
            "{id}: a gap without a reason: {field:?}"
        );
    }
}

#[cfg(feature = "aperti")]
#[test]
fn mzxml_and_numpress_mzml_carry_conv() {
    use common::mzxml::{mzxml, open_mzxml, run, FLAT};
    use common::{mzml_di, open, scans, Cod};
    check(&mut *open_mzxml(mzxml(&run(), FLAT)), "mzxml");
    let np = Cod::Np {
        intensities: mzlab_core::numpress::Np::Slof,
        zlib: true,
    };
    check(&mut *open(mzml_di(&scans(), np, None)), "mzml-numpress");
}

#[cfg(feature = "andi")]
#[test]
fn andi_carries_conv() {
    use common::andi::{andi_ms, open_andi, FLAT_RUN};
    check(&mut *open_andi(andi_ms(&FLAT_RUN)), "andi-ms");
}

#[cfg(feature = "shimadzu")]
#[test]
fn shimadzu_carries_conv() {
    use common::shimadzu::qgd;
    use mzlab_core::lettori::{FileSet, Limits, Member};
    let b = qgd(6);
    let fs = FileSet {
        membri: vec![Member {
            nome: "a.qgd".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    check(
        &mut *mzlab_lettori::open_bare(fs, &Limits::sito()).unwrap(),
        "shimadzu",
    );
}

#[cfg(feature = "waters")]
#[test]
fn waters_carries_conv() {
    use mzlab_core::lettori::Limits;
    let mut l = mzlab_lettori::open_bare(common_waters::synthetic_set(), &Limits::sito()).unwrap();
    check(&mut *l, "waters-raw");
}
