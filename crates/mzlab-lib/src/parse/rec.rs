//! The record being parsed (known fields in a fixed array, peaks) and its conversion to a `Spectrum`; used by `Parser` only.

use super::*;

/// Declares the record fields that are read (normalised keys, see `key_of`): `K`, its number and `slot_of`. Any other key of a record is not kept.
macro_rules! keys {
    ($($name:ident = $lit:literal),* $(,)?) => {
        #[derive(Clone, Copy)]
        #[allow(clippy::enum_variant_names)]
        pub(super) enum K { $($name),* }
        pub(super) const N_KEYS: usize = [$($lit),*].len();
        pub(super) fn slot_of(k: &str) -> Option<K> {
            match k { $($lit => Some(K::$name),)* _ => None }
        }
    };
}

keys! {
    PrecursorMz = "precursormz", Precursor = "precursor", PrecursorSlash = "precursorm/z", PepMass = "pepmass",
    PrecursorType = "precursortype", Adduct = "adduct", IonAdduct = "ionadduct",
    IonMode = "ionmode", Polarity = "polarity", IonizationMode = "ionizationmode", Charge = "charge",
    Name = "name", Title = "title", CompoundName = "compoundname", ChName = "chname", RecordTitle = "recordtitle",
    CollisionEnergy = "collisionenergy", Ce = "ce", InstrumentType = "instrumenttype", Instrument = "instrument",
    Formula = "formula", MolFormula = "molformula", MolecularFormula = "molecularformula",
    InchiKey = "inchikey", InchiKeySmiles = "inchikeysmiles", Smiles = "smiles", IsomericSmiles = "isomericsmiles",
    Accession = "accession", SpectrumId = "spectrumid", LibId = "libid", Usi = "usi", Db = "db#", Id = "id",
    Authors = "authors", Author = "author", License = "license", Licence = "licence",
    RtInSeconds = "rtinseconds", RetentionTime = "retentiontime", Rt = "rt", RtInMinutes = "rtinminutes",
    RetentionTimeMin = "retentiontimemin",
    Ri = "ri", RetentionIndex = "retentionindex", Kovats = "kovats", KovatsRi = "kovatsri",
    KovatsRetentionIndex = "kovatsretentionindex",
    Ionization = "ionization", IonizationType = "ionizationtype", Ionisation = "ionisation", SpectrumType = "spectrumtype",
}

const _: () = assert!(N_KEYS <= 64);

/// `key_of` into a stack buffer: no known key is longer than the buffer, so a longer one gives `""`, which is none of them.
pub(super) fn norm_key<'a>(k: &str, buf: &'a mut [u8; 32]) -> &'a str {
    let mut n = 0;
    for c in k
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
    {
        let w = c.len_utf8();
        let Some(out) = buf.get_mut(n..n + w) else {
            return "";
        };
        c.encode_utf8(out);
        n += w;
    }
    buf.get(..n)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("")
}

pub(super) struct Rec {
    /// The fields kept, by `K`; `set` tells which of them the record has (the first occurrence of a key wins).
    pub(super) vals: [String; N_KEYS],
    pub(super) set: u64,
    pub(super) mz: Vec<f64>,
    pub(super) it: Vec<f64>,
    pub(super) extra_peaks: bool,
}

impl Default for Rec {
    fn default() -> Rec {
        Rec {
            vals: std::array::from_fn(|_| String::new()),
            set: 0,
            mz: Vec::new(),
            it: Vec::new(),
            extra_peaks: false,
        }
    }
}

impl Rec {
    /// Empties the record for the next one (the field strings keep their room).
    pub(super) fn reset(&mut self) {
        self.set = 0;
        self.mz.clear();
        self.it.clear();
        self.extra_peaks = false;
    }

    pub(super) fn field(&mut self, k: K, v: &str) {
        let bit = 1u64 << (k as u32);
        if self.set & bit == 0 {
            self.set |= bit;
            if let Some(slot) = self.vals.get_mut(k as usize) {
                slot.clear();
                slot.push_str(v);
            }
        }
    }

    /// `field` by a normalised key; a key that is not one of the fields is dropped.
    pub(super) fn field_str(&mut self, key: &str, v: &str) {
        if let Some(k) = slot_of(key) {
            self.field(k, v);
        }
    }

    pub(super) fn has(&self, k: K) -> bool {
        self.set & (1u64 << (k as u32)) != 0
    }

    pub(super) fn peak(&mut self, m: f64, i: f64) {
        if m.is_finite() && i.is_finite() && m > 0.0 && i > 0.0 {
            if self.mz.len() < MAX_RAW_PEAKS {
                self.mz.push(m);
                self.it.push(i);
            } else {
                self.extra_peaks = true;
            }
        }
    }

    /// Peak text: segments split by «;», the first two numbers of each (JS `LibParser.peak`).
    pub(super) fn peaks_text(&mut self, line: &str) {
        for seg in line.split(';') {
            let b = seg.as_bytes();
            let mut pos = 0;
            if let (Some(m), Some(i)) = (next_number(b, &mut pos), next_number(b, &mut pos)) {
                self.peak(m, i);
            }
        }
    }

    pub(super) fn get(&self, keys: &[K]) -> Option<&str> {
        keys.iter()
            .find(|k| self.has(**k))
            .and_then(|k| self.vals.get(*k as usize))
            .map(|s| s.as_str())
    }
}

/// The retention time of a record in minutes: minute keys as they are (a trailing «s»/«sec» unit means seconds), «rtinseconds» (MGF) divided by 60.
pub(super) fn rt_minutes(c: &Rec) -> Option<f64> {
    let secs = parse_float(c.get(&[K::RtInSeconds]).unwrap_or(""));
    if secs.is_finite() && secs >= 0.0 {
        return Some(secs / 60.0);
    }
    let raw = c.get(&[K::RetentionTime, K::Rt, K::RtInMinutes, K::RetentionTimeMin])?;
    let v = parse_float(raw);
    if !v.is_finite() || v < 0.0 {
        return None;
    }
    let unit = raw
        .trim_start()
        .trim_start_matches(|ch: char| {
            ch.is_ascii_digit() || matches!(ch, '.' | ',' | '+' | '-' | 'e' | 'E')
        })
        .trim()
        .to_lowercase();
    Some(if unit.starts_with('s') { v / 60.0 } else { v })
}

/// The retention index of a record: `RI`, `RetentionIndex`, `Kovats`, MassBank `KOVATS_RI`; a plausible positive number only.
pub(super) fn ri_of(c: &Rec) -> Option<f64> {
    let v = parse_float(c.get(&[
        K::Ri,
        K::RetentionIndex,
        K::Kovats,
        K::KovatsRi,
        K::KovatsRetentionIndex,
    ])?);
    (v.is_finite() && v > 0.0 && v < 20_000.0).then_some(v)
}

/// Electron ionisation named by the ionisation or the spectrum type («EI», «EI-B», «electron ionization»).
pub(super) fn is_ei(c: &Rec) -> bool {
    c.get(&[
        K::Ionization,
        K::IonizationType,
        K::Ionisation,
        K::SpectrumType,
    ])
    .is_some_and(|v| {
        let v = v.trim().to_ascii_lowercase();
        v == "ei" || v.starts_with("ei-") || v.starts_with("ei ") || v.contains("electron")
    })
}

/// Turns a record into a spectrum or counts why not.
pub(super) fn finish(c: &mut Rec, stats: &mut Stats) -> Option<Spectrum> {
    let prec = parse_float(
        c.get(&[K::PrecursorMz, K::Precursor, K::PrecursorSlash, K::PepMass])
            .unwrap_or(""),
    );
    if prec.is_nan() || prec <= 0.0 {
        stats.dropped += 1;
        stats.no_prec += 1;
        return None;
    }
    if c.mz.is_empty() {
        stats.dropped += 1;
        stats.no_peaks += 1;
        return None;
    }
    let adduct = c
        .get(&[K::PrecursorType, K::Adduct, K::IonAdduct])
        .unwrap_or("");
    let pol = pol_from(
        c.get(&[K::IonMode, K::Polarity, K::IonizationMode])
            .unwrap_or(""),
        adduct,
        c.get(&[K::Charge]).unwrap_or(""),
    );
    let g = |keys: &[K]| limit(c.get(keys).unwrap_or(""));
    let meta = Meta {
        name: g(&[
            K::Name,
            K::Title,
            K::CompoundName,
            K::ChName,
            K::RecordTitle,
        ]),
        adduct: limit(adduct),
        ce: g(&[K::CollisionEnergy, K::Ce]),
        instrument: g(&[K::InstrumentType, K::Instrument]),
        formula: g(&[K::Formula, K::MolFormula, K::MolecularFormula]),
        inchikey: g(&[K::InchiKey, K::InchiKeySmiles]),
        smiles: g(&[K::Smiles, K::IsomericSmiles]),
        accession: g(&[K::Accession, K::SpectrumId, K::LibId, K::Usi, K::Db, K::Id]),
        authors: g(&[K::Authors, K::Author]),
        license: g(&[K::License, K::Licence]),
    };
    let rt = rt_minutes(c);
    let (ri, ei) = (ri_of(c), is_ei(c));
    stats.read += 1;
    Some(Spectrum {
        prec,
        pol,
        mz: std::mem::take(&mut c.mz),
        it: std::mem::take(&mut c.it),
        meta,
        rt,
        ri,
        ei,
    })
}
