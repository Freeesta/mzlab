//! The family `andi` against hostile netCDF files: a count, a dimension, an offset or a name that lies must be an error with a key, never a panic or a huge allocation; and (with the private data
//! repository, `MZLAB_DATI`) against the real files exported by ChemStation and MassHunter. The real-data tests are skipped without it.
#![cfg(feature = "andi")]

mod comune;

use comune::andi::*;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn buono() -> Vec<u8> {
    andi_ms(&PIATTA)
}

fn pos(b: &[u8], ago: &[u8]) -> usize {
    b.windows(ago.len()).position(|w| w == ago).unwrap()
}

#[test]
fn un_file_netcdf4_ha_la_sua_chiave() {
    let mut b = b"\x89HDF\r\n\x1a\n".to_vec();
    b.resize(512, 0);
    assert_eq!(errore_andi(b), "err.file.netcdf4");
}

#[test]
fn nessuna_lunghezza_del_file_manda_in_panico_il_lettore() {
    let b = buono();
    for n in (0..b.len()).step_by(7).chain([b.len() - 1]) {
        let _ = errore_andi(b[..n].to_vec());
    }
}

#[test]
fn conteggi_e_dimensioni_enormi_sono_errori_senza_allocare() {
    let b = buono();
    // the number of dimensions, of attributes and of variables
    for da in [
        8usize,
        pos(&b, b"experiment_title") - 4 - 4 - 4,
        pos(&b, b"scan_acquisition_time") - 8 - 4,
    ] {
        let mut x = b.clone();
        x[da + 4..da + 8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_ne!(errore_andi(x), "ok", "{da}");
    }
    // the length of the dimension scan_number: more scans than the file has points for
    let mut x = b.clone();
    let i = pos(&x, b"scan_number") + 12;
    x[i..i + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_ne!(errore_andi(x), "ok");
}

#[test]
fn un_nome_troppo_lungo_e_un_errore() {
    let mut x = buono();
    let i = pos(&x, b"point_number") - 4;
    x[i..i + 4].copy_from_slice(&(1u32 << 30).to_be_bytes());
    assert_eq!(errore_andi(x), "err.dam.unreadable");
}

#[test]
fn un_begin_fuori_dal_file_e_un_errore_con_chiave() {
    let b = buono();
    // the `begin` of scan_index: the last four bytes of its entry
    let i = pos(&b, b"scan_index") + 12 + 4 + 4 + 8 + 4 + 4;
    let mut x = b.clone();
    x[i..i + 4].copy_from_slice(&0x7fff_fff0u32.to_be_bytes());
    assert_eq!(errore_andi(x), "err.dam.unreadable");
}

#[test]
fn point_count_negativo_o_indici_fuori_dai_punti_non_panicano() {
    let b = buono();
    for (nome, valore) in [
        ("point_count", -5i32),
        ("point_count", 1 << 30),
        ("scan_index", 1 << 30),
        ("scan_index", -1),
    ] {
        let i = pos(&b, nome.as_bytes());
        let mut x = b.clone();
        // the data of the variable: its `begin` is in the header, found through the variable table
        let begin = u32::from_be_bytes(x[i + 36..i + 40].try_into().unwrap()) as usize;
        x[begin..begin + 4].copy_from_slice(&valore.to_be_bytes());
        let _ = errore_andi(x);
    }
}

/// Every real `.cdf` of the data repository: the scans counted, the points sorted and finite, the TIC of the file equal to the sum of the points, the times increasing.
#[test]
fn i_cdf_veri_si_aprono_e_i_conti_tornano() {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/netcdf_cdf") else {
        eprintln!("SKIP andi: no data repository (MZLAB_DATI)");
        return;
    };
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cdf")) {
            continue;
        }
        let nome = p.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_andi(std::fs::read(&p).unwrap());
        let n = l.campioni()[0].scansioni;
        assert!(n > 100, "{nome}");
        for (prefisso, attese) in [
            ("MTBLS7740", 7231),
            ("MTBLS8819", 14960),
            ("MTBLS13991", 4109),
        ] {
            if nome.starts_with(prefisso) {
                assert_eq!(n, attese, "{nome}");
            }
        }
        let indice = l.indice(0).unwrap().to_vec();
        assert!(indice.windows(2).all(|w| w[0].rt <= w[1].rt), "{nome}");
        for k in (0..n).step_by((n / 40).max(1)) {
            let s = l.scansione(0, k).unwrap();
            assert!(
                !s.mz.is_empty() && s.mz.windows(2).all(|w| w[0] <= w[1]),
                "{nome} scan {k}"
            );
            let somma: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
            assert!(
                (somma - indice[k].tic).abs() <= 1e-3 * somma.max(1.0),
                "{nome} scan {k}: {somma} {}",
                indice[k].tic
            );
        }
        assert!(l.metodo(0).unwrap().contains("\"andi-ms\""), "{nome}");
    }
}
