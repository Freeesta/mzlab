//! The family `andi`: the netCDF-3 files of the AIA/ANDI standard (ASTM E2077 mass spectra, E1947 chromatograms) that every maker exports (docs/agenti/lettori-andi.md). Its module is loaded only when a `.cdf` arrives.

mod cromato;
pub mod ms;
mod netcdf;
mod testata;
