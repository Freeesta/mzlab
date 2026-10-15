use crate::error::Result;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// Precursor reaction info for MS2+ scans (32 bytes).
#[derive(Debug, Clone)]
pub struct Reaction {
    pub precursor_mz: f64,
    pub unknown_double: f64,
    pub energy: f64,
    pub unknown_long1: u32,
    pub unknown_long2: u32,
}

/// M/z acquisition range.
#[derive(Debug, Clone)]
pub struct FractionCollector {
    pub low_mz: f64,
    pub high_mz: f64,
}

/// Scan event preamble - byte array encoding scan parameters.
#[derive(Debug, Clone)]
pub struct ScanEventPreamble {
    pub bytes: Vec<u8>,
}

/// Complete scan event.
#[derive(Debug, Clone)]
pub struct ScanEvent {
    pub preamble: ScanEventPreamble,
    pub reactions: Vec<Reaction>,
    pub fraction_collectors: Vec<FractionCollector>,
    pub coefficients: Vec<f64>,
}

/// The scan events of a file, one per scan, kept compactly: the preambles (136 bytes, a handful of distinct ones in a DDA file) once each, and the reactions,
/// the fraction collectors and the calibration coefficients of all the events in three flat tables with the end of each event's part. [`Self::get`] builds the
/// event of a scan when asked.
#[derive(Debug, Clone, Default)]
pub struct ScanEventTable {
    presets: Vec<ScanEventPreamble>,
    preset_of: Vec<u32>,
    reactions: Vec<Reaction>,
    reactions_end: Vec<u32>,
    collectors: Vec<FractionCollector>,
    collectors_end: Vec<u32>,
    coefficients: Vec<f64>,
    coefficients_end: Vec<u32>,
}

impl ScanEventTable {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            preset_of: Vec::with_capacity(n),
            reactions_end: Vec::with_capacity(n),
            collectors_end: Vec::with_capacity(n),
            coefficients_end: Vec::with_capacity(n),
            ..Self::default()
        }
    }

    /// Appends an event (its preamble is shared with an earlier one that has the same bytes).
    pub fn push(&mut self, e: ScanEvent) {
        let i = match self
            .presets
            .iter()
            .rposition(|p| p.bytes == e.preamble.bytes)
        {
            Some(i) => i,
            None => {
                self.presets.push(e.preamble);
                self.presets.len() - 1
            }
        };
        self.preset_of.push(i as u32);
        self.reactions.extend(e.reactions);
        self.reactions_end.push(self.reactions.len() as u32);
        self.collectors.extend(e.fraction_collectors);
        self.collectors_end.push(self.collectors.len() as u32);
        self.coefficients.extend(e.coefficients);
        self.coefficients_end.push(self.coefficients.len() as u32);
    }

    pub fn len(&self) -> usize {
        self.preset_of.len()
    }

    pub fn is_empty(&self) -> bool {
        self.preset_of.is_empty()
    }

    /// Event `i`.
    pub fn get(&self, i: usize) -> Option<ScanEvent> {
        let start = |ends: &[u32]| if i == 0 { 0 } else { ends[i - 1] as usize };
        let preamble = self.presets.get(*self.preset_of.get(i)? as usize)?.clone();
        Some(ScanEvent {
            preamble,
            reactions: self.reactions[start(&self.reactions_end)..self.reactions_end[i] as usize]
                .to_vec(),
            fraction_collectors: self.collectors
                [start(&self.collectors_end)..self.collectors_end[i] as usize]
                .to_vec(),
            coefficients: self.coefficients
                [start(&self.coefficients_end)..self.coefficients_end[i] as usize]
                .to_vec(),
        })
    }

    pub fn first(&self) -> Option<ScanEvent> {
        self.get(0)
    }

    pub fn iter(&self) -> impl Iterator<Item = ScanEvent> + '_ {
        (0..self.len()).filter_map(|i| self.get(i))
    }
}

impl ScanEventPreamble {
    pub(crate) fn size_for_version(version: u32) -> usize {
        match version {
            0..=8 => 41,
            57 | 60 => 80,
            62 => 120,
            63 | 64 => 128,
            _ => 136, // v66+
        }
    }

    /// Polarity: byte 4.
    pub fn polarity(&self) -> Option<crate::Polarity> {
        self.bytes
            .get(4)
            .and_then(|&b| crate::Polarity::from_byte(b))
    }

    /// Scan mode (centroid/profile): byte 5.
    pub fn scan_mode(&self) -> Option<crate::ScanMode> {
        self.bytes
            .get(5)
            .and_then(|&b| crate::ScanMode::from_byte(b))
    }

    /// MS power: byte 6.
    pub fn ms_power(&self) -> Option<crate::MsPower> {
        self.bytes
            .get(6)
            .and_then(|&b| crate::MsPower::from_byte(b))
    }

    /// Scan type: byte 7.
    pub fn scan_type(&self) -> Option<crate::ScanType> {
        self.bytes
            .get(7)
            .and_then(|&b| crate::ScanType::from_byte(b))
    }

    /// Dependent scan flag: byte 10.
    pub fn is_dependent(&self) -> bool {
        self.bytes.get(10).copied() == Some(1)
    }

    /// True if this scan is a Data-Independent Acquisition (DIA) MS2+ scan:
    /// ms_power >= 2 and the dependent flag is NOT set.  In DIA mode the
    /// instrument selects a wide isolation window and fragments all ions in
    /// that window together, without targeting a specific precursor.
    pub fn is_dia(&self) -> bool {
        let ms_power = self.bytes.get(6).copied().unwrap_or(0);
        ms_power >= 2 && !self.is_dependent()
    }

    /// Ionization mode: byte 11.
    pub fn ionization(&self) -> Option<crate::Ionization> {
        self.bytes
            .get(11)
            .and_then(|&b| crate::Ionization::from_byte(b))
    }

    /// Activation method: byte 24.
    pub fn activation(&self) -> Option<crate::Activation> {
        self.bytes
            .get(24)
            .and_then(|&b| crate::Activation::from_byte(b))
    }

    /// Wideband (broadband isolation) flag: byte 32.
    pub fn is_wideband(&self) -> bool {
        self.bytes.get(32).copied() == Some(1)
    }

    /// Analyzer type: byte 40.
    pub fn analyzer(&self) -> Option<crate::Analyzer> {
        self.bytes
            .get(40)
            .and_then(|&b| crate::Analyzer::from_byte(b))
    }

    /// Raw value of the activation byte (byte 24). Useful for diagnostics when
    /// `activation()` returns `None` (unrecognised code).
    pub fn activation_byte(&self) -> u8 {
        self.bytes.get(24).copied().unwrap_or(0)
    }
}

impl ScanEvent {
    /// Read one scan event.
    ///
    /// For v66 files, `body_primary` is the body size for primary (MS1) scans
    /// and `body_dependent` is the body size for dependent (MS2+) scans.
    /// For uniform-event files these two values are identical.
    /// Pass `(0, 0)` for pre-v66 files (body size is self-describing).
    pub(crate) fn read<R: Read + Seek>(
        r: &mut BinaryReader<R>,
        version: u32,
        sizes: [usize; 3],
    ) -> Result<Self> {
        let [body_primary, body_dependent, body_independent] = sizes;
        let preamble_size = ScanEventPreamble::size_for_version(version);
        let preamble_bytes = r.read_bytes(preamble_size)?;
        let preamble = ScanEventPreamble {
            bytes: preamble_bytes,
        };

        if version >= 66 {
            // Select body size: primary (MS1) vs dependent (MS2+).
            // Primary = ms_power <= Ms1 AND not dependent.
            let is_primary = preamble.bytes.get(6).copied().unwrap_or(0) <= 1
                && preamble.bytes.get(10).copied() != Some(1);
            let body_size = if is_primary {
                body_primary
            } else if preamble.bytes.get(10).copied() != Some(1) {
                body_independent
            } else {
                body_dependent
            };
            // Tribrid instruments (Eclipse, Fusion Lumos) use variable-length events
            // detected when body_primary != body_dependent.  Dependent events on
            // these instruments use a different body layout than QExactive/Exploris.
            let is_tribrid_dep = body_primary != body_dependent
                && !is_primary
                && preamble.bytes.get(10).copied() == Some(1);
            Self::read_v66(r, preamble, body_size, is_tribrid_dep)
        } else {
            Self::read_pre_v66(r, preamble)
        }
    }

    /// V66 scan events have a fixed-size body (size determined by the caller
    /// from the stream's address-space: body_size = event_size - preamble_size).
    ///
    /// Two body layouts are in use across instrument families:
    ///
    /// **QExactive / Exploris / uniform-event files** (body_primary == body_dependent):
    ///   body[0..4]:   u32 unknown_long[0] (always 1)
    ///   body[4..8]:   u32 flags (0 for MS1, 0xA0000000 for MS2)
    ///   body[8..64]:  opaque fields (precursor-related for MS2, range aux for MS1)
    ///   body[fc_off..fc_off+16]: FractionCollector (scan window) at body_size-64
    ///
    /// **Tribrid dependent events** (Eclipse, Fusion Lumos; is_tribrid_dep=true):
    ///   body[0..4]:   u32 n_reactions  (0 for MS1, 1 for HCD, 2 for EThcD)
    ///   body[4..]:    n_reactions * 32-byte Reaction records
    ///   body[body_size-88..body_size-72]: FractionCollector (scan window)
    ///
    /// In both cases nparam + coefficients live at body[body_size-64..].
    fn read_v66<R: Read + Seek>(
        r: &mut BinaryReader<R>,
        preamble: ScanEventPreamble,
        body_size: usize,
        is_tribrid_dep: bool,
    ) -> Result<Self> {
        let body = r.read_bytes(body_size)?;

        // FractionCollector (scan window) location varies by instrument family:
        //   - Q Exactive / Exploris / Astral (body_size ≥ 136): offset 64
        //   - Orbitrap Elite / Fusion / Fusion Lumos / Velos Pro (body_size=96):
        //     MS1 → offset 8, MS2 → offset 64
        //   - Orbitrap Ascend (body_size=152) MS2 → offset 128
        //   - LTQ ion-trap only files (body_size < 96): offset 8
        // Empirically verified across a 24-file multi-instrument corpus.
        //
        // Strategy: try a small list of candidate offsets in priority order
        // and accept the first that yields a plausible m/z window. This is
        // robust across the observed zoo of body layouts.
        // Tribrid dependent events (Eclipse, Fusion Lumos) store the FractionCollector
        // at body[body_size-88] = body[120] for a 208-byte body.  All other v66
        // instruments use one of the legacy candidate offsets.
        let tribrid_fc = body_size.saturating_sub(88);
        let tribrid_candidates;
        let legacy_candidates;
        let candidates: &[usize] = if is_tribrid_dep {
            tribrid_candidates = [tribrid_fc, 8usize, 64, 128];
            &tribrid_candidates
        } else {
            legacy_candidates = [64usize, 8, 128, body_size.saturating_sub(80)];
            &legacy_candidates
        };
        let fc_match = candidates.iter().copied().find_map(|off| {
            if off + 16 > body_size {
                return None;
            }
            let low_mz = crate::bytes::read_f64_le(&body, off).ok()?;
            let high_mz = crate::bytes::read_f64_le(&body, off + 8).ok()?;
            // A valid scan window must be finite, monotonic, and within
            // physically realistic m/z bounds (instruments top out well
            // below 1e5 m/z). Accept lo == hi as well because some
            // SIM / tSIM scans use a single-point window.
            if low_mz.is_finite()
                && high_mz.is_finite()
                && low_mz >= 0.1
                && low_mz <= high_mz
                && high_mz <= 50_000.0
            {
                Some((off, FractionCollector { low_mz, high_mz }))
            } else {
                None
            }
        });
        let (fc_offset, fraction_collectors) = match fc_match {
            Some((off, fc)) => (Some(off), vec![fc]),
            None => (None, Vec::new()),
        };

        // The nparam + coefficients block immediately follows the scan-window
        // FractionCollector. On Q Exactive / Exploris / Astral (FC at offset 64)
        // that is offset 80; the legacy `body_size - 64` only coincides with it
        // when body_size == 144 (e.g. Q Exactive), so Exploris (body_size 136)
        // came back with no coefficients and its profile m/z was mis-converted.
        // Use the FC-relative offset for that family; other layouts keep the
        // legacy offset.
        // The parameter count follows the scan window directly in every layout seen (MS1 of the tribrid instruments: window at
        // 8, count at 24); when the count found there is not plausible the legacy offset is used.
        let plausible = |off: usize| {
            off + 4 <= body_size
                && crate::bytes::read_u32_le(&body, off).is_ok_and(|n| (1..=16).contains(&n))
        };
        let np_off = match fc_offset {
            Some(off) if plausible(off + 16) => off + 16,
            Some(64) => 80,
            _ => body_size.saturating_sub(64),
        };
        let mut coefficients = Vec::new();
        if np_off + 4 <= body_size {
            let nparam_raw = crate::bytes::read_u32_le(&body, np_off)? as usize;
            // Cap nparam at the number of f64s that actually fit in the remaining body.
            // Without this cap, a garbage nparam (e.g. 0xFFFFFFFF from uninitialised
            // bytes) causes billions of loop iterations just to evaluate the guard.
            let max_nparam = (body_size.saturating_sub(np_off + 4)) / 8;
            let nparam = nparam_raw.min(max_nparam);
            for i in 0..nparam {
                let off = np_off + 4 + i * 8;
                coefficients.push(crate::bytes::read_f64_le(&body, off)?);
            }
        }

        // Parse precursor reactions from the v66 body for dependent scans and for
        // non-dependent MS2+ scans (DIA mode). In DIA, MS2 scans are not flagged as
        // dependent but still carry one or more isolation window reactions in the body.
        //
        // Condition: parse reactions when ms_power >= 2 OR the scan is flagged dependent.
        // (MS1 primary scans with ms_power <= 1 and dependent=false are skipped.)
        let is_ms2_plus = preamble.bytes.get(6).copied().unwrap_or(0) >= 2;
        let mut reactions = if (!is_ms2_plus && !preamble.is_dependent()) || body_size < 8 {
            Vec::new()
        } else if is_tribrid_dep {
            // Tribrid dependent events (Eclipse, Fusion Lumos): n_reactions is stored at
            // body[0..4] and each 32-byte Reaction record begins at body[4].
            // The second reaction (if any) is typically a zero-mz supplemental step.
            let np = if body_size >= 4 {
                crate::bytes::read_u32_le(&body, 0)? as usize
            } else {
                0
            };
            let rxn_start = 4usize;
            let max_np = body_size.saturating_sub(rxn_start + 32) / 32;
            if np == 0 || np > max_np.max(1) {
                Vec::new()
            } else {
                let mut rxs = Vec::with_capacity(np);
                for i in 0..np {
                    let off = rxn_start + i * 32;
                    if off + 32 > body_size {
                        break;
                    }
                    let mz = crate::bytes::read_f64_le(&body, off)?;
                    let unk = crate::bytes::read_f64_le(&body, off + 8)?;
                    let energy = crate::bytes::read_f64_le(&body, off + 16)?;
                    let ul1 = crate::bytes::read_u32_le(&body, off + 24)?;
                    let ul2 = crate::bytes::read_u32_le(&body, off + 28)?;
                    if mz.is_finite() && mz >= 0.0 {
                        rxs.push(Reaction {
                            precursor_mz: mz,
                            unknown_double: unk,
                            energy,
                            unknown_long1: ul1,
                            unknown_long2: ul2,
                        });
                    }
                }
                rxs
            }
        } else {
            let np = crate::bytes::read_u32_le(&body, 4)? as usize;
            // Sanity check: np must fit within the body minus minimum fixed overhead.
            // Each reaction is 32 bytes; require at least 32 bytes of post-reaction
            // data (FC=16, nparam=4, minimum tail) for the body to be plausible.
            let max_np = body_size.saturating_sub(8 + 32) / 32;
            if np == 0 || np > max_np.max(1) {
                Vec::new()
            } else {
                let mut rxs = Vec::with_capacity(np);
                for i in 0..np {
                    let off = 8 + i * 32;
                    if off + 32 > body_size {
                        break;
                    }
                    let mz = crate::bytes::read_f64_le(&body, off)?;
                    let unk = crate::bytes::read_f64_le(&body, off + 8)?;
                    let energy = crate::bytes::read_f64_le(&body, off + 16)?;
                    let ul1 = crate::bytes::read_u32_le(&body, off + 24)?;
                    let ul2 = crate::bytes::read_u32_le(&body, off + 28)?;
                    // Accept only reactions with plausible m/z values (0 is valid for
                    // MS1 triggers; accept non-negative finite values).
                    if mz.is_finite() && mz >= 0.0 {
                        rxs.push(Reaction {
                            precursor_mz: mz,
                            unknown_double: unk,
                            energy,
                            unknown_long1: ul1,
                            unknown_long2: ul2,
                        });
                    }
                }
                rxs
            }
        };

        // Exploris v66 dependent scans store the reaction record starting at
        // body offset 4 (precursor m/z as an f64 at body[4..12]), with no
        // leading count word, so the offset-8 parse above finds nothing. Fall
        // back to that layout when an MS2+/dependent, non-tribrid scan yielded
        // no reaction.
        if reactions.is_empty()
            && (is_ms2_plus || preamble.is_dependent())
            && !is_tribrid_dep
            && body_size >= 36
        {
            let mz = crate::bytes::read_f64_le(&body, 4)?;
            if mz.is_finite() && mz > 0.0 && mz < 50_000.0 {
                reactions.push(Reaction {
                    precursor_mz: mz,
                    unknown_double: crate::bytes::read_f64_le(&body, 12)?,
                    energy: crate::bytes::read_f64_le(&body, 20)?,
                    unknown_long1: crate::bytes::read_u32_le(&body, 28)?,
                    unknown_long2: crate::bytes::read_u32_le(&body, 32)?,
                });
            }
        }

        Ok(Self {
            preamble,
            reactions,
            fraction_collectors,
            coefficients,
        })
    }

    fn read_pre_v66<R: Read + Seek>(
        r: &mut BinaryReader<R>,
        preamble: ScanEventPreamble,
    ) -> Result<Self> {
        let np = r.read_u32()?;
        let mut reactions = Vec::new();
        for _ in 0..np {
            reactions.push(Reaction::read(r)?);
        }
        let _unk1 = r.read_u32()?;
        let fc = FractionCollector::read(r)?;
        let nparam = r.read_u32()?;
        r.check_count(nparam as u64, 8)?;
        let mut coefficients = Vec::with_capacity(nparam as usize);
        for _ in 0..nparam {
            coefficients.push(r.read_f64()?);
        }
        let _unk2 = r.read_u32()?;
        let _unk3 = r.read_u32()?;

        Ok(Self {
            preamble,
            reactions,
            fraction_collectors: vec![fc],
            coefficients,
        })
    }
}

impl Reaction {
    fn read<R: Read + Seek>(r: &mut BinaryReader<R>) -> Result<Self> {
        let precursor_mz = r.read_f64()?;
        let unknown_double = r.read_f64()?;
        let energy = r.read_f64()?;
        let unknown_long1 = r.read_u32()?;
        let unknown_long2 = r.read_u32()?;
        Ok(Self {
            precursor_mz,
            unknown_double,
            energy,
            unknown_long1,
            unknown_long2,
        })
    }
}

impl FractionCollector {
    fn read<R: Read + Seek>(r: &mut BinaryReader<R>) -> Result<Self> {
        let low_mz = r.read_f64()?;
        let high_mz = r.read_f64()?;
        Ok(Self { low_mz, high_mz })
    }
}
