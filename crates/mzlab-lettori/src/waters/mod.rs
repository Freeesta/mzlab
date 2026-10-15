//! Waters MassLynx `.raw` folders (`X.raw/` with `_FUNCTNS.INF`, `_FUNCnnn.IDX/DAT`, `_HEADER.TXT`, `_extern.inf`, `_INLET.INF`, `_CHROMS.INF`): the `Lettore` of the family `waters`.
//! The layout was worked out from real folders (Xevo TQ in MRM/SIR, Xevo G2-XS, Synapt and Vion in MS, MSe and DDA) with the free `openwraw` reader (Apache-2.0) as a guide for the
//! TOF record encodings; the code is written here from scratch. Format notes: `docs/agenti/waters.md`. Ion mobility is not decoded and the lock mass correction is not applied.

pub mod lettore;

mod cartella;
mod cromi;
mod dat;
mod doc;
mod funzioni;
mod idx;
mod inf;
mod lati;
mod mrm;

pub use lettore::VOCE;
