//! The UV detector of a Thermo `.raw` acquired through Chromeleon, for `metodo/1.rivelatori`: the lines of the instrument script that give the channels
//! (`UV_VIS_1.Wavelength = 260.0 [nm]`, `.Bandwidth`, `.RefWavelength`, `.RefBandwidth`), the 3D field (`3DFIELD.MinWavelength`, `.MaxWavelength`, `.BunchWidth`) and the
//! sampling rate (`Data_Collection_Rate = 2 [Hz]`). The first line of each key wins (later lines change the value during the run).

use crate::metodo::{num, obj, text, V};

/// The number of a `key = value [unit]` line whose key is `key` (`Off` and other words are not numbers).
fn number_of(line: &str, key: &str) -> Option<f64> {
    let rest = line.trim_start().strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?;
    let v = rest.split('[').next()?.trim();
    v.parse::<f64>().ok().filter(|x| x.is_finite())
}

fn first(blocks: &[String], key: &str) -> Option<f64> {
    blocks
        .iter()
        .flat_map(|b| b.lines())
        .find_map(|l| number_of(l, key))
}

/// The `rivelatori` array (`None`: the method has no UV lines).
pub fn section(blocks: &[String]) -> Option<V> {
    let channels: Vec<V> = (1..=8)
        .filter_map(|k| {
            let nm = first(blocks, &format!("UV_VIS_{k}.Wavelength"))?;
            obj(vec![
                ("nm", num(Some(nm))),
                (
                    "banda_nm",
                    num(first(blocks, &format!("UV_VIS_{k}.Bandwidth"))),
                ),
                (
                    "riferimento_nm",
                    num(first(blocks, &format!("UV_VIS_{k}.RefWavelength"))),
                ),
                (
                    "banda_riferimento_nm",
                    num(first(blocks, &format!("UV_VIS_{k}.RefBandwidth"))),
                ),
            ])
        })
        .collect();
    let rate = first(blocks, "Data_Collection_Rate");
    let uv = (!channels.is_empty()).then(|| {
        obj(vec![
            ("tipo", text("UV_VIS")),
            ("frequenza_Hz", num(rate)),
            ("canali", Some(V::Arr(channels))),
        ])
    });
    let (lo, hi) = (
        first(blocks, "3DFIELD.MinWavelength"),
        first(blocks, "3DFIELD.MaxWavelength"),
    );
    let field = lo.zip(hi).and_then(|(lo, hi)| {
        obj(vec![
            ("tipo", text("PDA")),
            ("nm", Some(V::Arr(vec![V::Num(lo), V::Num(hi)]))),
            ("passo_nm", num(first(blocks, "3DFIELD.BunchWidth"))),
            ("frequenza_Hz", num(rate)),
        ])
    });
    let all: Vec<V> = uv.flatten().into_iter().chain(field).collect();
    (!all.is_empty()).then_some(V::Arr(all))
}

/// True when the method has a 3D field (PDA spectra): the spectra are in a controller that is not decoded, so the caller warns.
pub fn has_3d_field(blocks: &[String]) -> bool {
    first(blocks, "3DFIELD.MinWavelength").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = "       Data_Collection_Rate =        2 [Hz]\n       UV_VIS_1.Wavelength =         260.0 [nm]\n       UV_VIS_1.Bandwidth =          5 [nm]\n       UV_VIS_1.RefWavelength =      Off\n       UV_VIS_1.RefBandwidth =       10 [nm]\n       UV_VIS_2.Wavelength =         280.0 [nm]\n       3DFIELD.MinWavelength =       210.0 [nm]\n       3DFIELD.MaxWavelength =       600.0 [nm]\n       3DFIELD.BunchWidth =          5 [nm]\n 5.000 UV_VIS_1.Wavelength =         300.0 [nm]\n";

    #[test]
    fn channels_field_and_rate_are_read() {
        let v = section(&[SCRIPT.to_string()]).unwrap().json();
        assert!(
            v.contains(r#""tipo": "UV_VIS""#) || v.contains(r#""tipo":"UV_VIS""#),
            "{v}"
        );
        assert!(v.contains("260"), "{v}");
        assert!(!v.contains("300"), "the first line of a key wins: {v}");
        assert!(v.contains("600") && v.contains("210"), "{v}");
        assert!(has_3d_field(&[SCRIPT.to_string()]));
    }

    #[test]
    fn a_method_without_uv_gives_none() {
        assert!(section(&["Flow = 0.35 [ml/min]".to_string()]).is_none());
        assert!(!has_3d_field(&[]));
    }

    /// Junk and torn texts never panic and never invent a section.
    #[test]
    fn hostile_texts_do_not_panic() {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let words = [
            "Lock Masses:",
            "Pos List Name:",
            "Mass List:",
            "Use lock masses",
            "UV_VIS_1.Wavelength =",
            "3DFIELD.MinWavelength =",
            "1e999",
            "NaN",
            "|",
            "\r\n",
            "[",
            "=",
            "Precursor (m/z)",
            "Product (m/z)",
        ];
        for _ in 0..500 {
            let mut t = String::new();
            for _ in 0..40 {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                t.push_str(words[(x >> 33) as usize % words.len()]);
                t.push(if x & 1 == 0 { ' ' } else { '\n' });
            }
            let _ = section(&[t.clone()]);
            let _ = has_3d_field(&[t]);
        }
    }
}
