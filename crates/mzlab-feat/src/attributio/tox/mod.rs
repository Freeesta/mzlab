// Ported from Attributio chem/registry.py, ecotox/ghs.py, ecotox/hazard.py, reglists/schema.py, vocab.py.
//! The data of the registry, regulatory-list and aquatic-hazard packages (RG-xx): the vocabulary words, the GHS/CLP categories and the typed readers of the
//! versioned snapshots (registry, list membership, ECOTOX values, ECOSAR endpoints). Nothing here touches the network: the snapshots are text the caller reads.

mod candidates;
mod ecosar;
pub mod ghs;
pub mod hazard;
pub mod registry;
pub mod snapshot;
pub mod vocab;
