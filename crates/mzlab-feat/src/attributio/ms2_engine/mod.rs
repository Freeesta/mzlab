// Ported from Attributio `ms2_engine/` (matcher.py, compiler.py, generate.py) and `readers/ms2_engine.py`.
//! The experimental MS2 engine: a spectral library (GNPS JSON / MSP, any size mzLab's parsers read) is compiled into per-polarity entries and every feature
//! spectrum is searched by entropy similarity within a precursor tolerance. The hits are EVIDENCE shown next to the cascade: they never make an identity and never
//! reach a level (the chemist decides 2b). Off by default (`use_ms2_engine`), visible only in administrator mode. The deep-learning embedding search of the Python
//! original (MS2DeepScore) is not ported: its score is not an identity and it needs a neural network that does not run here.

pub mod compile;
pub mod hits;
pub mod insilico;
pub mod matcher;
pub mod rearrange;

pub use compile::{clean_library_name, compile_library, EngineEntry};
pub use hits::{engine_hits_csv, load_engine_hits, read_engine_hits, EngineHitRow, ENGINE_COLUMNS};
pub use matcher::{Ms2Library, Query, SearchHit, SearchOpts};
