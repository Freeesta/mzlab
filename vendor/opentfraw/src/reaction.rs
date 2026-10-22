//! Per-reaction view of an MSn scan: precursor, activation and energy of every step of the chain, built from the scan event and the final reaction's
//! trailer values. Used by `mzml::PrecursorInfo::reactions` and by the mzLab reader.

use crate::scan_event::{Reaction, ScanEvent};
use crate::types::{Activation, Analyzer, MsPower};

/// One step of the precursor chain (MS2 has one, MS3 two, ... MS8 seven). What is not understood stays `raw_*`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReactionInfo {
    pub precursor_mz: f64,
    /// Isolation width of the final step (from the trailer); earlier steps keep it in `raw_double` until the meaning of that field is confirmed.
    pub isolation_width: Option<f64>,
    pub activation: Option<Activation>,
    /// Supplemental activation (EThcD: HCD; ETciD: CID).
    pub supplemental: Option<Activation>,
    pub energy: Option<f64>,
    /// True when the trailer says the energy is an NCE (dimensionless %); false means the unit is not known (not that it is eV).
    pub energy_is_nce: bool,
    /// Stepped NCE (`20,35,60`): every step, `energy` keeps the single value the trailer gives.
    pub energy_steps: Vec<f64>,
    pub supplemental_energy: Option<f64>,
    pub raw_double: f64,
    pub raw_flags: u32,
    pub raw_long2: u32,
}

/// What the trailer says about the final reaction.
#[derive(Debug, Clone, Default)]
pub struct FinalReaction {
    pub isolation_width: Option<f64>,
    pub energy: Option<f64>,
    pub energy_is_nce: bool,
    pub energy_steps: Vec<f64>,
    pub supplemental_energy: Option<f64>,
}

impl Reaction {
    /// Activation coded in `unknown_long1`: bit 0 says the energy is set, the rest is the Thermo activation type (0 CID, 1 MPD, 2 ECD, 3 PQD, 4 ETD,
    /// 5 HCD, 11 UVPD). Only CID (1) and HCD (11) are seen in the corpus, the others come from the public Thermo enumeration.
    pub fn activation(&self) -> Option<Activation> {
        if self.unknown_long1 & 1 == 0 {
            return None;
        }
        Some(match self.unknown_long1 >> 1 {
            0 => Activation::CID,
            1 => Activation::MPID,
            2 => Activation::ECD,
            3 => Activation::PQD,
            4 => Activation::ETD,
            5 => Activation::HCD,
            11 => Activation::UVPD,
            _ => return None,
        })
    }
}

/// The chain of `event`: one entry per reaction with a precursor, in order. Empty without reactions. The reactions are read as `scan_filter` reads
/// them, so the two always agree on which steps exist and which one is the final.
pub fn chain(event: &ScanEvent, fin: &FinalReaction) -> Vec<ReactionInfo> {
    let rs = &event.reactions;
    let act = event.activation();
    let info = |rx: &Reaction| ReactionInfo {
        precursor_mz: rx.precursor_mz,
        energy: Some(rx.energy).filter(|&e| e > 0.0),
        raw_double: rx.unknown_double,
        raw_flags: rx.unknown_long1,
        raw_long2: rx.unknown_long2,
        ..Default::default()
    };
    let valid = rs.iter().filter(|r| r.precursor_mz > 0.0).count();
    let tribrid_ethcd = event.preamble.ms_power() == Some(MsPower::Ms2)
        && rs.len() >= 2
        && valid == 1
        && event.preamble.analyzer() == Some(Analyzer::FTMS)
        && matches!(act, Some(Activation::CID | Activation::HCD));
    if tribrid_ethcd {
        let Some(rx) = rs.iter().find(|r| r.precursor_mz > 0.0) else {
            return Vec::new();
        };
        let second = rs
            .iter()
            .find(|r| r.precursor_mz <= 0.0)
            .and_then(Reaction::activation);
        return vec![ReactionInfo {
            activation: Some(Activation::ETD),
            supplemental: second.or(act),
            supplemental_energy: fin.energy,
            isolation_width: fin.isolation_width,
            ..info(rx)
        }];
    }
    let last = rs.len().saturating_sub(1);
    rs.iter()
        .enumerate()
        .filter(|(_, r)| r.precursor_mz > 0.0)
        .map(|(i, rx)| {
            let mut out = info(rx);
            if i != last {
                out.activation = rx.activation().or(act);
                out.energy_is_nce = fin.energy_is_nce;
                return out;
            }
            out.isolation_width = fin.isolation_width;
            out.energy = fin.energy.or(out.energy);
            out.energy_is_nce = fin.energy.is_some() && fin.energy_is_nce;
            out.energy_steps = fin.energy_steps.clone();
            if act == Some(Activation::EThcD) {
                out.activation = Some(Activation::ETD);
                out.supplemental = Some(Activation::HCD);
                out.supplemental_energy = fin.supplemental_energy;
            } else {
                // On FTMS the event's own activation comes from the first reaction; the final one says CID or HCD for itself.
                let ftms = event.preamble.analyzer() == Some(Analyzer::FTMS);
                out.activation = match (act, rx.activation()) {
                    (
                        Some(Activation::CID | Activation::HCD),
                        Some(a @ (Activation::CID | Activation::HCD)),
                    ) if ftms => Some(a),
                    _ => act,
                };
            }
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan_event::{FractionCollector, ScanEventPreamble};

    fn rx(mz: f64, energy: f64, flags: u32) -> Reaction {
        Reaction {
            precursor_mz: mz,
            unknown_double: 2.0,
            energy,
            unknown_long1: flags,
            unknown_long2: 0,
        }
    }

    fn event(power: u8, analyzer: u8, code: u8, reactions: Vec<Reaction>) -> ScanEvent {
        let mut bytes = vec![0u8; 136];
        bytes[6] = power;
        bytes[10] = 1;
        bytes[24] = code;
        bytes[40] = analyzer;
        ScanEvent {
            preamble: ScanEventPreamble { bytes },
            reactions,
            fraction_collectors: vec![FractionCollector {
                low_mz: 50.0,
                high_mz: 500.0,
            }],
            coefficients: Vec::new(),
        }
    }

    fn fin(energy: f64, nce: bool) -> FinalReaction {
        FinalReaction {
            isolation_width: Some(1.2),
            energy: Some(energy),
            energy_is_nce: nce,
            ..Default::default()
        }
    }

    #[test]
    fn ms2_hcd_has_one_step_with_the_trailer_values() {
        let c = chain(&event(2, 4, 1, vec![rx(300.5, 0.0, 11)]), &fin(30.0, true));
        assert_eq!(c.len(), 1);
        let r = &c[0];
        assert_eq!(
            (r.precursor_mz, r.activation),
            (300.5, Some(Activation::HCD))
        );
        assert_eq!(
            (r.energy, r.energy_is_nce, r.isolation_width),
            (Some(30.0), true, Some(1.2))
        );
        assert_eq!(r.raw_flags, 11);
    }

    #[test]
    fn ms2_cid_on_ftms_is_cid_in_ev() {
        let c = chain(&event(2, 4, 4, vec![rx(300.5, 0.0, 1)]), &fin(35.0, false));
        assert_eq!(
            (c[0].activation, c[0].energy, c[0].energy_is_nce),
            (Some(Activation::CID), Some(35.0), false)
        );
    }

    #[test]
    fn ms3_cid_then_hcd_keeps_each_activation() {
        let e = event(3, 4, 4, vec![rx(500.0, 35.0, 1), rx(300.0, 0.0, 11)]);
        let c = chain(&e, &fin(40.0, true));
        let got: Vec<_> = c
            .iter()
            .map(|r| (r.precursor_mz, r.activation, r.energy))
            .collect();
        assert_eq!(
            got,
            vec![
                (500.0, Some(Activation::CID), Some(35.0)),
                (300.0, Some(Activation::HCD), Some(40.0))
            ]
        );
        assert_eq!(
            (c[0].isolation_width, c[1].isolation_width),
            (None, Some(1.2))
        );
        assert_eq!(c[0].raw_double, 2.0);
    }

    #[test]
    fn ms8_chain_has_seven_steps() {
        let mz = [317.0, 261.0, 244.0, 200.0, 172.0, 145.0, 112.0];
        let e = event(8, 4, 4, mz.iter().map(|&m| rx(m, 25.0, 1)).collect());
        let c = chain(&e, &fin(25.0, true));
        assert_eq!(c.iter().map(|r| r.precursor_mz).collect::<Vec<_>>(), mz);
        assert!(c.iter().all(|r| r.activation == Some(Activation::CID)));
    }

    #[test]
    fn ethcd_is_etd_plus_supplemental_hcd() {
        let fin = FinalReaction {
            supplemental_energy: Some(28.0),
            ..fin(35.0, true)
        };
        let c = chain(&event(2, 4, 12, vec![rx(649.1, 0.0, 0)]), &fin);
        assert_eq!(
            (c[0].activation, c[0].supplemental, c[0].supplemental_energy),
            (Some(Activation::ETD), Some(Activation::HCD), Some(28.0))
        );
        assert_eq!(c[0].energy, Some(35.0));
    }

    #[test]
    fn tribrid_ethcd_and_etcid_read_the_second_reaction() {
        let hcd = chain(
            &event(2, 4, 1, vec![rx(649.1, 0.0, 0), rx(0.0, 0.0, 11)]),
            &fin(28.0, true),
        );
        let cid = chain(
            &event(2, 4, 4, vec![rx(649.1, 0.0, 0), rx(0.0, 0.0, 1)]),
            &fin(28.0, true),
        );
        assert_eq!(
            (
                hcd[0].activation,
                hcd[0].supplemental,
                hcd[0].supplemental_energy
            ),
            (Some(Activation::ETD), Some(Activation::HCD), Some(28.0))
        );
        assert_eq!(cid[0].supplemental, Some(Activation::CID));
    }

    #[test]
    fn stepped_nce_stays_a_list() {
        let fin = FinalReaction {
            energy_steps: vec![20.0, 35.0, 60.0],
            ..fin(35.0, true)
        };
        let c = chain(&event(2, 4, 1, vec![rx(300.5, 0.0, 11)]), &fin);
        assert_eq!(c[0].energy_steps, vec![20.0, 35.0, 60.0]);
    }

    #[test]
    fn no_reactions_no_chain() {
        assert!(chain(&event(2, 4, 1, vec![]), &FinalReaction::default()).is_empty());
    }

    /// The bytes of a counted v66 MS3 event (CID then HCD flags) read back into the chain.
    #[test]
    fn a_counted_event_from_bytes_gives_the_chain() {
        use crate::scan_event::{counted_tests::counted_event, COUNTED_BODY};
        let mut bytes = counted_event(&[500.0, 300.0]);
        bytes[24] = 1; // preamble: HCD family code, FTMS below
        bytes[40] = 4;
        for (i, flags) in [1u32, 11].into_iter().enumerate() {
            let at = 136 + 4 + i * 56 + 24;
            bytes[at..at + 4].copy_from_slice(&flags.to_le_bytes());
        }
        let mut r = crate::reader::BinaryReader::new(std::io::Cursor::new(bytes));
        let e = ScanEvent::read(&mut r, 66, [COUNTED_BODY; 3]).unwrap();
        let c = chain(&e, &fin(40.0, true));
        let got: Vec<_> = c
            .iter()
            .map(|r| (r.precursor_mz, r.activation, r.energy))
            .collect();
        assert_eq!(
            got,
            vec![
                (500.0, Some(Activation::CID), Some(25.0)),
                (300.0, Some(Activation::HCD), Some(40.0))
            ]
        );
    }
}
