//! Attributio, untargeted side: the detection pipeline that reproduces mzmine 3.9.0 stage by stage (ADAP chromatogram builder first), each stage rewritten
//! from scratch in Rust with parity against the outputs of mzmine on synthetic data (`tests/data/untargeted/mzmine39/`). Behind the feature `untargeted`.

pub mod adap;
