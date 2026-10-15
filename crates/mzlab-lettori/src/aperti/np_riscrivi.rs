//! MS-Numpress arrays of one spectrum or chromatogram (the bytes of one `<spectrum>`/`<chromatogram>` element) rewritten as plain 64-bit arrays,
//! so that the engine's mzML parser (`mzml::parse`, `Mzml::chromatogram`), which does not know numpress, reads them unchanged.

use std::borrow::Cow;

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::Limiti;
use mzlab_core::numpress;

const PIENI: [&str; 6] = [
    "MS:1002312",
    "MS:1002313",
    "MS:1002314",
    "MS:1002746",
    "MS:1002747",
    "MS:1002748",
];
/// The cvParams of an array that say how it is stored: they are replaced by "64-bit float" and "no compression".
const FORMA: [&str; 4] = ["MS:1000574", "MS:1000521", "MS:1000523", "MS:1000576"];

fn attributo<'a>(tag: &'a str, nome: &str) -> Option<&'a str> {
    let chiave = format!(" {nome}=\"");
    let da = tag.find(&chiave)? + chiave.len();
    let fino = tag[da..].find('"')?;
    tag.get(da..da + fino)
}

fn errore(why: &str) -> Error {
    Error::new("err.file.xml").with("detail", why)
}

/// `xml` with every numpress array decoded and written as a plain 64-bit one (borrowed when it has none).
pub fn riscrivi<'a>(xml: &'a [u8], limiti: &Limiti) -> Result<Cow<'a, [u8]>> {
    let Ok(s) = std::str::from_utf8(xml) else {
        return Ok(Cow::Borrowed(xml));
    };
    if !PIENI.iter().any(|a| s.contains(a)) {
        return Ok(Cow::Borrowed(xml));
    }
    let atteso = s
        .find("defaultArrayLength=\"")
        .and_then(|i| attributo(&s[i.saturating_sub(1)..], "defaultArrayLength"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut out = String::with_capacity(s.len());
    let mut da = 0usize;
    while let Some(rel) = s[da..].find("<binaryDataArray") {
        let a = da + rel;
        let dopo = s.as_bytes().get(a + "<binaryDataArray".len()).copied();
        if !matches!(dopo, Some(b' ' | b'>' | b'\n' | b'\r' | b'\t')) {
            // `<binaryDataArrayList`
            out.push_str(&s[da..a + 1]);
            da = a + 1;
            continue;
        }
        let fine = s[a..]
            .find("</binaryDataArray>")
            .map(|e| a + e + "</binaryDataArray>".len())
            .ok_or_else(|| errore("unclosed binaryDataArray"))?;
        out.push_str(&s[da..a]);
        out.push_str(&blocco(&s[a..fine], atteso, limiti)?);
        da = fine;
    }
    out.push_str(&s[da..]);
    Ok(Cow::Owned(out.into_bytes()))
}

/// One `<binaryDataArray>`; unchanged when it is not numpress.
fn blocco<'a>(b: &'a str, atteso: usize, limiti: &Limiti) -> Result<Cow<'a, str>> {
    // the cvParams of the array: (start, end, accession)
    let mut cv: Vec<(usize, usize, &str)> = Vec::new();
    let mut da = 0usize;
    while let Some(rel) = b[da..].find("<cvParam") {
        let a = da + rel;
        let fine = a + b[a..].find('>').ok_or_else(|| errore("unclosed cvParam"))? + 1;
        cv.push((a, fine, attributo(&b[a..fine], "accession").unwrap_or("")));
        da = fine;
    }
    let Some((np, z)) = cv
        .iter()
        .find_map(|(_, _, acc)| numpress::of_accession(acc))
    else {
        return Ok(Cow::Borrowed(b));
    };
    let zlib = z || cv.iter().any(|(_, _, acc)| *acc == "MS:1000574");
    let (testo, bin_a, bin_f) = match b.find("<binary>") {
        Some(i) => {
            let f = b[i..]
                .find("</binary>")
                .ok_or_else(|| errore("unclosed binary"))?;
            (
                &b[i + "<binary>".len()..i + f],
                i,
                i + f + "</binary>".len(),
            )
        }
        None => match b.find("<binary/>") {
            Some(i) => ("", i, i + "<binary/>".len()),
            None => return Ok(Cow::Borrowed(b)),
        },
    };
    let valori = numpress::decode_text(np, zlib, testo, atteso)?;
    if (valori.len() as u64).saturating_mul(8) > limiti.byte {
        return Err(Error::memory_budget());
    }
    let mut out = String::with_capacity(b.len() + valori.len() * 11);
    let mut da = 0usize;
    for (a, f, acc) in &cv {
        if *a >= bin_a {
            break;
        }
        if PIENI.contains(acc) || FORMA.contains(acc) {
            out.push_str(&b[da..*a]);
            da = *f;
        }
    }
    out.push_str(&b[da..bin_a]);
    out.push_str(
        "<cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\" value=\"\"/>\
         <cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\" value=\"\"/>",
    );
    out.push_str("<binary>");
    out.push_str(&numpress::plain_base64(&valori));
    out.push_str("</binary>");
    out.push_str(&b[bin_f..]);
    Ok(Cow::Owned(out))
}
