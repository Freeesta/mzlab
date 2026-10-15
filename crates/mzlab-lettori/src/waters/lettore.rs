//! The `Lettore` of a MassLynx `.raw` folder: the functions of `_FUNCTNS.INF`, the scans of the scanning ones (MS, MSe, DDA) by position in `_FUNCnnn.DAT`, and the SIR/MRM channels,
//! the TIC, the lock mass and the analog traces as chromatograms (`cromi.rs`). The lock mass function is not in the spectra and its correction is not applied.

use super::cartella::{errore, testo, Cartella};
use super::dat::{decodifica, Codifica, Taratura};
use super::funzioni::{self, Funzione};
use super::idx::{self, Riga};
use super::inf::{self, Extern, FunzioneExt, Inlet, Modo, Testata};
use super::lati::{nomi_composti, Tabella};
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{
    Campione, Campo, Fiducia, FileSet, Formato, Lettore, Limiti, Posizione, VoceCromatogramma,
    VoceLettore, VoceScansione, CONTRATTO,
};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;

const FUNCTNS: &str = "_FUNCTNS.INF";
const PRIMA_DAT: &str = "_FUNC001.DAT";

pub const VOCE: VoceLettore = VoceLettore {
    id: "waters-raw",
    contratto: CONTRATTO,
    estensioni: &[".raw"],
    riconosci,
    apri,
};

/// A folder with `_FUNC001.DAT` is a MassLynx folder; it is surer when `_FUNCTNS.INF` is there and has whole records.
fn riconosci(fs: &mut FileSet) -> Fiducia {
    let base =
        |m: &mzlab_core::lettori::Membro| super::cartella::base(&m.nome).to_ascii_lowercase();
    if !fs.membri.iter().any(|m| base(m) == "_func001.dat") {
        return Fiducia::No;
    }
    match fs.membri.iter().find(|m| base(m) == "_functns.inf") {
        Some(m) if m.dim > 0 && m.dim % 416 == 0 => Fiducia::Si,
        _ => Fiducia::Forse,
    }
}

/// One function with everything read of it when the folder is opened (the DAT is read by position).
pub(super) struct Fun {
    pub(super) f: Funzione,
    pub(super) ext: FunzioneExt,
    pub(super) righe: Vec<Riga>,
    /// Index of `_FUNCnnn.DAT` among the members.
    pub(super) dat: Option<usize>,
    pub(super) dat_len: u64,
    /// Bytes per record, from the index.
    pub(super) larg: Option<u64>,
    pub(super) cod: Option<Codifica>,
    pub(super) tar: Taratura,
    pub(super) sts: Option<Tabella>,
    pub(super) ee: Option<Tabella>,
    pub(super) nomi: Vec<String>,
    pub(super) livello: u8,
    pub(super) polarita: i8,
}

pub struct LettoreWaters {
    pub(super) c: Cartella,
    pub(super) funs: Vec<Fun>,
    pub(super) testata: Testata,
    pub(super) ext: Extern,
    pub(super) inlet: Option<Inlet>,
    pub(super) analogiche: Vec<super::cromi::Analogica>,
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    /// For each row of `indice`: (function, row of its IDX).
    mappa: Vec<(usize, usize)>,
    /// What the reader did not decode: (key of the notice, where).
    pub(super) avvisi: Vec<(String, String)>,
    pub(super) sha1: Option<Vec<Option<String>>>,
    chroms: Option<Vec<Chromatogram>>,
}

fn opzionale(c: &mut Cartella, nome: &str) -> Option<Vec<u8>> {
    c.leggi(nome).ok().flatten()
}

fn livello_di(funs: &[Fun], i: usize) -> u8 {
    let (f, x) = (&funs[i].f, &funs[i].ext);
    match x.modo {
        Modo::Riferimento => 1,
        Modo::Msms | Modo::Figlie => 2,
        Modo::Ms => {
            let partner = funs.iter().any(|g| {
                g.f.indice != f.indice
                    && !g.f.riferimento()
                    && g.ext.modo == Modo::Ms
                    && g.ext.rampa.is_none()
            });
            if x.rampa.is_some() && partner {
                2
            } else {
                1
            }
        }
        _ if f.indice == 1 || funs.len() == 1 => 1,
        _ => 2,
    }
}

/// Everything of function `f`: its index, the DAT and the side tables.
fn carica(
    c: &mut Cartella,
    f: Funzione,
    ext: &Extern,
    tars: &Testata,
    spesa: &mut u64,
) -> Result<(Fun, Option<String>)> {
    let n = f.indice;
    let nome = |e: &str| format!("_FUNC{n:03}.{e}");
    let righe = match opzionale(c, &nome("IDX")) {
        Some(b) => idx::leggi(&b, &c.limiti)?,
        None => Vec::new(),
    };
    *spesa += righe.len() as u64;
    c.limiti.voci_ok(*spesa)?;
    let dat = c.cerca(&nome("DAT"));
    let dat_len = dat.map_or(0, |i| c.fs.membri[i].dim);
    let larg = idx::larghezza(&righe, dat_len);
    let cod = if f.mirato() {
        None
    } else {
        larg.and_then(Codifica::da_larghezza)
    };
    let lim = c.limiti;
    let tab =
        |c: &mut Cartella, e: &str| opzionale(c, &nome(e)).and_then(|b| Tabella::leggi(&b, &lim));
    let (sts, ee) = (tab(c, "STS"), tab(c, "EE"));
    let nomi = opzionale(c, &nome("CMP")).map_or(Vec::new(), |b| nomi_composti(&b, &lim));
    let x = ext.funzioni.get(&n).cloned().unwrap_or_default();
    let avviso = if dat.is_none() || righe.is_empty() {
        Some(format!("function {n}: no scans"))
    } else if !f.mirato() && cod.is_none() {
        Some(format!("function {n}: record width"))
    } else {
        None
    };
    let polarita = x.polarita.or(ext.polarita).unwrap_or(0);
    let tar = tars.tarature.get(&n).cloned().unwrap_or_default();
    let fun = Fun {
        f,
        ext: x,
        righe,
        dat,
        dat_len,
        larg,
        cod,
        tar,
        sts,
        ee,
        nomi,
        livello: 1,
        polarita,
    };
    Ok((fun, avviso))
}

fn apri(fs: FileSet, limiti: &Limiti) -> Result<Box<dyn Lettore>> {
    let mut c = Cartella::new(fs, *limiti);
    let inf = c
        .leggi(FUNCTNS)?
        .ok_or_else(|| errore("_FUNCTNS.INF is missing"))?;
    if c.cerca(PRIMA_DAT).is_none() {
        return Err(errore("_FUNC001.DAT is missing"));
    }
    let lista = funzioni::leggi(&inf, limiti)?;
    let testata = opzionale(&mut c, "_HEADER.TXT")
        .map_or_else(Testata::default, |b| inf::testata(&testo(&b)));
    let ext = opzionale(&mut c, "_extern.inf")
        .map_or_else(Extern::default, |b| inf::extern_inf(&testo(&b)));
    let inlet = opzionale(&mut c, "_INLET.INF").map(|b| inf::inlet(&testo(&b)));
    let (mut funs, mut avvisi, mut spesa) = (Vec::new(), Vec::new(), 0u64);
    for f in lista {
        let (fun, av) = carica(&mut c, f, &ext, &testata, &mut spesa)?;
        if let Some(a) = av {
            avvisi.push((
                "meta.nonDecodificato".to_string(),
                format!("{}: {a}", c.nome),
            ));
        }
        funs.push(fun);
    }
    for i in 0..funs.len() {
        funs[i].livello = livello_di(&funs, i);
    }
    if funs.iter().any(|f| f.ext.mobilita) {
        avvisi.push((
            "meta.nonDecodificato".into(),
            format!("{}: ion mobility", c.nome),
        ));
    }
    let analogiche = super::cromi::analogiche(&mut c);
    let mut l = LettoreWaters {
        c,
        funs,
        testata,
        ext,
        inlet,
        analogiche,
        campioni: Vec::new(),
        indice: Vec::new(),
        mappa: Vec::new(),
        avvisi,
        sha1: None,
        chroms: None,
    };
    l.costruisci_indice();
    let nome = l
        .testata
        .get("Acquired Name")
        .map_or_else(|| l.c.nome.clone(), str::to_string);
    l.campioni = vec![Campione {
        id: 0,
        nome,
        scansioni: l.indice.len(),
    }];
    Ok(Box::new(l))
}

impl LettoreWaters {
    /// The spectra: every scan with points of a scanning function that is not the lock mass, in the order of time.
    fn costruisci_indice(&mut self) {
        let mut v: Vec<(f32, usize, usize)> = Vec::new();
        for (i, f) in self.funs.iter().enumerate() {
            if f.f.mirato() || f.f.riferimento() || f.cod.is_none() || f.dat.is_none() {
                continue;
            }
            v.extend(
                f.righe
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.conta > 0)
                    .map(|(k, r)| (r.rt, i, k)),
            );
        }
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for &(_, i, k) in &v {
            let f = &self.funs[i];
            let r = f.righe[k];
            let precursore = (f.livello == 2)
                .then(|| {
                    f.sts
                        .as_ref()
                        .and_then(|t| t.valore("Set Mass", k))
                        .filter(|&m| m > 0.0)
                        .or(f.ext.set_mass)
                })
                .flatten();
            self.indice.push(VoceScansione {
                id: format!("function={} process=0 scan={}", f.f.indice, k + 1),
                rt: f64::from(r.rt),
                livello: f.livello,
                polarita: f.polarita,
                tic: f64::from(r.tic),
                picco_base: None,
                filtro: format!("function {}", f.f.indice),
                precursore,
                posizione: f.dat.map(|m| Posizione {
                    membro: u16::try_from(m).unwrap_or(0),
                    offset: r.offset,
                    lunghezza: u32::try_from(
                        u64::from(r.conta) * f.cod.map_or(0, |c| c.larghezza() as u64),
                    )
                    .unwrap_or(u32::MAX),
                }),
            });
            self.mappa.push((i, k));
        }
    }

    fn voce(&self, c: usize, k: usize) -> Result<(&Fun, usize, &VoceScansione)> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        let (i, r) = *self
            .mappa
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        Ok((&self.funs[i], r, &self.indice[k]))
    }
}

impl Lettore for LettoreWaters {
    fn formato(&self) -> Formato {
        Formato::Altro("waters-raw")
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        Ok(&self.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let (f, r, v) = self.voce(c, k)?;
        let (cod, dat) = (
            f.cod.ok_or_else(|| errore("record width"))?,
            f.dat.ok_or_else(|| errore("no DAT"))?,
        );
        let (off, len) = idx::estensione(&f.righe, r, cod.larghezza() as u64, f.dat_len)?;
        let (tar, livello, polarity, rt, precursor) =
            (f.tar.clone(), v.livello, v.polarita, v.rt, v.precursore);
        let id = v.id.clone();
        let filter = v.filtro.clone();
        let b = self.c.leggi_a(dat, off, len)?;
        let (mz, intensity) = decodifica(cod, &b, &tar)?;
        Ok(Spectrum {
            index: k,
            id,
            level: livello,
            rt,
            polarity,
            profile: cod.profilo(),
            filter,
            precursor,
            mz,
            intensity,
            wide: None,
        })
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        let (f, r, v) = self.voce(c, k)?;
        let campo = |nome: &str, valore: String, unita: Option<&str>| Campo {
            nome: nome.into(),
            valore,
            unita: unita.map(Into::into),
            cv: None,
        };
        let mut out = vec![
            campo("function", f.f.indice.to_string(), None),
            campo("scan", (r + 1).to_string(), None),
        ];
        if let Some((a, b)) = f.f.intervallo() {
            out.push(campo("scan range", format!("{a}-{b}"), Some("m/z")));
        }
        let ce = f
            .sts
            .as_ref()
            .and_then(|t| t.valore("Collision Energy", r))
            .filter(|&x| x > 0.0)
            .or(f.ext.ce);
        if let Some(ce) = ce {
            out.push(campo("collision energy", ce.to_string(), Some("eV")));
        }
        if let Some((a, b)) = f.ext.rampa {
            out.push(campo(
                "collision energy ramp",
                format!("{a}-{b}"),
                Some("eV"),
            ));
        }
        if let Some(p) = v.precursore {
            out.push(campo("precursor", p.to_string(), Some("m/z")));
        }
        if let Some(t) = &f.sts {
            for (n, x) in t
                .tutti(r)
                .into_iter()
                .filter(|(n, _)| n.contains("ock mass"))
            {
                out.push(campo(&format!("{n} (not applied)"), x.to_string(), None));
            }
        }
        Ok(out)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(self
            .cromatogrammi_tutti(c)?
            .iter()
            .map(|h| VoceCromatogramma {
                id: h.id.clone(),
                tipo: h.kind,
                unita: h.unit.clone(),
            })
            .collect())
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.cromatogrammi_tutti(c)?
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        super::doc::testata(self).map(Some)
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        super::doc::documento(self)
    }
}

impl LettoreWaters {
    /// The chromatograms of the folder, built once.
    fn cromatogrammi_tutti(&mut self, c: usize) -> Result<&[Chromatogram]> {
        if c != 0 {
            return Err(Error::new("error.badIndex").with("c", c));
        }
        if self.chroms.is_none() {
            let v = super::cromi::tutte(self)?;
            self.chroms = Some(v);
        }
        Ok(self.chroms.as_deref().unwrap_or_default())
    }
}
