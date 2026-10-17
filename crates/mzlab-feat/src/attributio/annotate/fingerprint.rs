// Ported from Attributio annotate.py.
//! The fingerprint of an annotation run: what the cache of the annotations is keyed on. It changes when a source file changes (its path, size and content),
//! when the campaign file or the chemist's decisions change, or when a setting of the cascade changes. No clock and no disk: the files are read through `Io`.

use super::settings::AnnotationSettings;
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::io::{join, Io};
use crate::attributio::readers::ms2::feature_mgf;
use crate::attributio::readers::mzmine::annotation_file;
use crate::attributio::readers::sirius::sirius_output_dir;
use crate::attributio::readers::standards::StandardsSettings;
use crate::attributio::sources::merged_table_path;
use crate::sortc::SortC;

/// Every file whose change must invalidate the cache that exists: the merged tables, the SIRIUS and GNPS2 exports, the mzMine annotations and the FBMN MGF, the
/// standards library and its spectra, the decisions, the curation tables, the sorbent tables and the campaign file.
pub fn source_files(c: &Campaign, io: &Io) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for pol in POLARITIES {
        files.extend(merged_table_path(c, io, pol).ok());
        if let Some(d) = sirius_output_dir(c, io, pol) {
            for t in [
                "canopus_structure_summary",
                "structure_identifications",
                "spectral_matches",
            ] {
                files.push(join(&d, &format!("{t}.tsv")));
            }
        }
        files.extend(annotation_file(c, io, pol));
        files.extend(feature_mgf(c, io, pol));
        let g = join(&c.gnps_output_dir(pol), "nf_output");
        files.push(join(&g, "library/merged_results_with_gnps.tsv"));
        files.push(join(&g, "networking/clustersummary_with_network.tsv"));
    }
    if let Some(set) = StandardsSettings::from_campaign(c) {
        files.extend(c.library_csv.clone());
        files.extend(set.decisions);
        files.extend(set.ms2_library.into_iter().flatten());
    }
    if c.has_sorbent_tables() {
        for pol in POLARITIES {
            files.extend(
                c.sorbents
                    .iter()
                    .filter_map(|s| c.sorbent_table_path(pol, s)),
            );
        }
    }
    files.push(c.file_name.clone());
    files.extend(c.feature_decisions_csv.clone());
    files.extend(c.curation_csv.iter().flatten().cloned());
    files.retain(|f| io.read(f).is_some());
    files
}

fn fnv(seed: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(seed, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// 24 hexadecimal digits that identify the run.
pub fn annotations_fingerprint(
    c: &Campaign,
    io: &Io,
    sites: &[String],
    s: &AnnotationSettings,
) -> String {
    let mut sorted: Vec<&str> = sites.iter().map(String::as_str).collect();
    sorted.sortc();
    let mut parts = vec![
        c.id.clone(),
        sorted.join(","),
        format!("{s:?}"),
        c.count_2b_as_2.to_string(),
    ];
    for p in source_files(c, io) {
        let data = io.read(&p).unwrap_or_default();
        parts.push(format!(
            "{p}:{}:{:016x}",
            data.len(),
            fnv(0xcbf2_9ce4_8422_2325, &data)
        ));
    }
    let text = parts.join("|");
    let (a, b) = (
        fnv(0xcbf2_9ce4_8422_2325, text.as_bytes()),
        fnv(0x8422_2325_cbf2_9ce4, text.as_bytes()),
    );
    format!("{a:016x}{b:016x}")[..24].to_string()
}
