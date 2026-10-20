//! The shared records that the identification steps pass to each other (`candidates` and `annot` of the data types of `contract.schema.json`): their
//! fields with the struct they are read from. Plain data like the step descriptors; `docs/schemi/records-1.json` is the export.

/// One field of a record.
pub struct Field {
    pub name: &'static str,
    pub ty: &'static str,
    pub doc: &'static str,
}

/// A shared record and the code that owns it.
pub struct Record {
    pub data_type: &'static str,
    pub source: &'static str,
    pub doc: &'static str,
    pub fields: Vec<Field>,
}

fn f(name: &'static str, ty: &'static str, doc: &'static str) -> Field {
    Field { name, ty, doc }
}

/// `candidates`: a ranked identity candidate of a feature (library hit, in silico structure or suspect), with scores and evidence.
fn candidates() -> Record {
    Record {
        data_type: "candidates",
        source: "attributio/ms2_engine/insilico.rs (Candidate, Ranked), mzlab-lib (search hit)",
        doc:
            "A hypothesis with its proofs, never an identity; the level is decided by annot.level.",
        fields: vec![
            f("id", "string", "Identifier in the list or library"),
            f("name", "string", "Compound name"),
            f(
                "smiles",
                "string",
                "Structure; rows without one are left out of the in silico step",
            ),
            f("formula", "string", "Molecular formula"),
            f("mass", "number", "Monoisotopic neutral mass (Da)"),
            f(
                "score",
                "number",
                "Score of the source: entropy similarity, cosine or fragmenter score",
            ),
            f(
                "explained",
                "number[]",
                "m/z of the peaks the candidate explains",
            ),
            f(
                "mass_ppm",
                "number",
                "Mass error against the precursor (ppm)",
            ),
            f(
                "source",
                "string",
                "Library, in silico list or suspect list",
            ),
        ],
    }
}

/// `annot`: the chosen annotation per row with its level.
fn annot() -> Record {
    Record {
        data_type: "annot",
        source: "attributio/annotate/types.rs (Feature, Decision, RtInfo)",
        doc: "The level a feature reaches with the basis and the reasons, and whether the programme or the chemist chose it.",
        fields: vec![
            f("pol", "string", "Polarity of the feature"),
            f("id", "integer", "Feature id"),
            f("level", "integer", "Level 1 to 5; 0 = detected, not annotated"),
            f("level_sub", "string", "Sub-level: 2a, 2b, 3a or 3b"),
            f("level_basis", "string", "Source the level rests on"),
            f("level_label", "string", "Label shown for the level"),
            f("level_flag", "string", "Caveat attached to the level"),
            f("level_origin", "string", "auto when the programme chose, user when a decision of the chemist changed it (see the audit log)"),
            f("smiles", "string", "Structure of the identity, when it has one"),
            f("inchikey_skeleton", "string", "First block of the InChIKey"),
            f("rt_group", "integer", "Group of features of one identity separated in RT"),
            f("feature_decision", "string", "Reason the chemist wrote, when a decision moved the feature"),
            f("regulatory", "object[]", "List memberships (annot.regulatory)"),
            f("hazard", "object", "Acute and chronic aquatic hazard record (annot.hazard)"),
        ],
    }
}

/// The shared records, in the order of the chain.
pub fn records() -> Vec<Record> {
    vec![candidates(), annot()]
}

/// The JSON of the records, one record per line.
pub fn records_json(records: &[Record]) -> String {
    let mut out = String::from("[\n");
    for (i, r) in records.iter().enumerate() {
        let fields: Vec<String> = r
            .fields
            .iter()
            .map(|x| {
                format!(
                    "{{\"name\":{:?},\"type\":{:?},\"doc\":{:?}}}",
                    x.name, x.ty, x.doc
                )
            })
            .collect();
        out.push_str(&format!(
            "{{\"dataType\":{:?},\"source\":{:?},\"doc\":{:?},\"fields\":[{}]}}{}\n",
            r.data_type,
            r.source,
            r.doc,
            fields.join(","),
            if i + 1 < records.len() { "," } else { "" }
        ));
    }
    out.push_str("]\n");
    out
}
