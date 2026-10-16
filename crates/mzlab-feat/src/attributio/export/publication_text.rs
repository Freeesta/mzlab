// Ported from Attributio diagnostics/publication_table.py.
//! The fixed texts of the README of the publication table: the dictionary of the columns that does not depend on the campaign and the definitions of the flags.

pub const DICT_HEAD: [(&str, &str); 36] = [
    ("ID", "Progressive number: anthropogenic contaminants first, then by confidence level, then by name."),
    ("Compound", "Curated common name of the identity (one row per compound; synonyms, brand names and library codes of the same compound merged)."),
    ("Confidence level", "Identification confidence after Schymanski et al. (Environ. Sci. Technol. 2014, 48, 2097): 1, 2a or 3 (criteria below). The best level among the features of the compound."),
    ("Use category", "anthropogenic contaminant (curated list of contaminants of emerging concern and industrial chemicals) or not classified as anthropogenic (not in that list; this is not a claim of natural origin)."),
    ("Contaminant class", "Class of the anthropogenic contaminant (curated)."),
    ("Acute aquatic hazard", "Acute aquatic hazard category of the structure, CLP Annex I Table 4.1.0 with the GHS (UN) thresholds on the lowest acute L(E)C50 among fish (96 h LC50), crustaceans (48 h EC50) and algae or aquatic plants (72-96 h ErC50 / EC50): Acute 1 (<= 1 mg/L), Acute 2 (> 1 to 10 mg/L), Acute 3 (> 10 to 100 mg/L), Not classified (> 100 mg/L or no effects at saturation); Not assessable = no structure or no data. Acute 2 and 3 exist in the UN GHS, not in the EU CLP. A hazard class, not a risk: the areas are signals, not concentrations."),
    ("Acute hazard tier", "Source tier of the acute category; the first tier with a category decides: regulatory (CLP harmonised classification), measured (L(E)C50 from the US EPA ECOTOX Knowledgebase, laboratory tests, geometric mean per species, or curated literature values), notified classification (H-statements of PubChem's GHS section: ECHA C&L notifications and other classifiers), predicted (curated read-across, else US EPA ECOSAR 2.2 class-based QSAR; baseline narcosis only = lower bound), not assessable."),
    ("Acute hazard basis", "The data that decided the acute category: taxon, species, endpoint, duration, value (mg/L) and number of results, the H-statements, the ECOSAR classes and predicted values, or the curated basis."),
    ("Chronic aquatic hazard", "Chronic (long-term) aquatic hazard category, CLP Annex I Table 4.1.0: from chronic NOEC/EC10 (Chronic 1 <= 0.1, Chronic 2 <= 1 mg/L), else from acute L(E)C50 (Chronic 1 <= 1, 2 <= 10, 3 <= 100 mg/L), the more stringent when chronic data miss a trophic level (CLP 4.1.3.3); Chronic 4 = safety net or notified H413. The substance is assumed NOT rapidly degradable (no degradability data used): a worst case for degradable compounds."),
    ("Chronic hazard tier", "Source tier of the chronic category (as for the acute one; predicted chronic values are ECOSAR chronic values, ChV)."),
    ("Chronic hazard basis", "The data that decided the chronic category, with the route (chronic data, acute data, or both)."),
    ("Hazard caveat", "Limits of the hazard record: Level 3 (hazard of the CSI:FingerID candidate, an isomer may differ), RT isomer (hazard of the named structure), a predicted category contradicted by a notified classification."),
    ("Hazard cross-check", "What the lower tiers say when they disagree with the category given (tier and category)."),
    ("Neutral molecular formula", "Neutral molecular formula of the structure (ASCII)."),
    ("Monoisotopic mass (Da)", "Monoisotopic mass of the molecular formula (Da)."),
    ("InChIKey", "InChIKey of the structure given by the source that named the compound; when that source gives only the connectivity block (14 characters), the full key of the PubChem representative structure. LC-HRMS does not determine stereochemistry: compare on the first block."),
    ("SMILES", "SMILES of the same structure."),
    ("Structure source", "Source of the structure: the source that named the compound (reference standard, CSI:FingerID, SIRIUS spectral, GNPS2, mzMine library) or PubChem name (resolution of the name, weaker)."),
    ("Polarity", "Ionisation mode of the representative ion (the feature with the best evidence at the compound's level)."),
    ("Adduct", "Ion type of the representative ion, as given by the evidence source (inferred from m/z when the source gives none: see Flags)."),
    ("m/z", "Measured m/z of the representative ion."),
    ("RT (min)", "Retention time of the representative ion (min)."),
    ("Reference RT (min)", "Retention time of the best-evidence feature of the identity (min): one identity is one compound at one retention time."),
    ("RT range (min)", "Lowest and highest retention time of the features collapsed in the row (min)."),
    ("Mass error (ppm)", "(measured m/z - theoretical m/z of the adduct of the molecular formula) / theoretical m/z x 10^6 (ppm)."),
    ("Evidence source", "Sources whose evidence passed the criteria of the compound's level, over all its features at that level."),
    ("Library", "Library of the best spectral match (Level 2a, and Level 3 resting on a library match), the reference standard (Level 1) or CSI:FingerID (Level 3)."),
    ("Reference spectrum", "Identifier of the matched library spectrum (GNPS2 CCMSLIB ID or USI), or the name of the reference standard."),
    ("Cosine", "Cosine similarity of the best spectral-library match of the representative ion (Level 2a, and Level 3 resting on a library match)."),
    ("Matched ions", "Fragment ions shared with the library spectrum (for MZmine, its own count of matching signals)."),
    ("COSMIC confidence", "COSMIC confidence score of the CSI:FingerID candidate of the representative ion (Level 3)."),
    ("Std mass error (ppm)", "Level 1: m/z error against the reference standard (ppm)."),
    ("Std delta RT (min)", "Level 1: retention-time difference from the reference standard on the same instrument (min)."),
    ("Std MS2 cosine", "Level 1: cosine of the MS/MS spectrum against the reference standard's spectrum of the same ion."),
    ("Std matched ions", "Level 1: fragment ions matched with the reference standard's spectrum."),
    ("Flags", "Caveats, separated by '; ' (list below)."),
];

pub const DICT_MID: [(&str, &str); 2] = [
    ("N procedural blanks with signal", "Largest number of procedural blanks in which a feature of the compound was found (MZmine blank subtraction)."),
    ("Min ratio to procedural blank", "Smallest, over the sites where the compound is DETECTED (blanks ignored), of (largest DETECTED area at the site / largest abundance of its features in the procedural blanks). Empty when the compound is absent from every blank."),
];

pub const DICT_TAIL: [(&str, &str); 2] = [
    ("N features", "Number of features (ions, adducts, polarities, in-source fragments) collapsed into the compound."),
    ("Feature IDs (polarity:MZmine id)", "The features of the compound: polarity and MZmine row ID in the aligned feature tables."),
];

pub const FLAG_DEFS: [(&str, &str); 12] = [
    ("Level 2a from MZmine library only", "Level 2a feature(s) whose only library match above threshold is MZmine (no second library confirms it); '(n of m Level 2a features)' when the compound has other Level 2a features. Information for the reader."),
    ("Level 3: library matches of different structures, not told apart by the spectrum", "Level 2a downgraded to 3: two or more spectral libraries pass every gate but name structures with different connectivity, and the best match exceeds the best match of another structure by less than 0.05 in cosine (the cosines of the three tools are different functions; a smaller difference does not tell the structures apart). The name shown is the one of the best match."),
    ("libraries name different structures: the best match names the compound", "Two or more spectral libraries pass every gate but name structures with different connectivity: the libraries are treated as one library and the best match (highest cosine, then more shared fragment ions) names the compound. It stays Level 2a when it exceeds the best match of another structure by at least 0.05 in cosine, otherwise Level 3 (see the previous flag). The competing matches and the margin are listed in brackets."),
    ("Level 3: library match judged insufficient by the analyst", "Level 2a downgraded to 3 by the analyst after review of the spectra."),
    ("isomers not discriminated", "the name covers isomers that MS/MS does not tell apart (a curated union named after the formula class, e.g. 'disaccharide (sucrose/palatinose)'), an oligomer series grouped by molecular formula (PEG/polyether), the features of the compound point to structures with different connectivity (different InChIKey first blocks), or the same library compound matched peaks at different retention times (Level 3). Stereoisomers are never distinguished (see InChIKey)."),
    ("Level 3: library match at a retention time other than the reference standard's (another isomer)", "Level 2a downgraded to 3: the reference standard (Level 1) elutes at another retention time, so this peak is another isomer."),
    ("the same name at another retention time is a separate identity", "features with the same name elute at other retention times: they are reported as separate identities ('<name> isomer, RT x.x min')."),
    ("@RTLOGP@", "Level 3: the retention time is inconsistent with the predicted logP of the candidate relative to the other candidates of the batch (isotonic fit, robust z-score): a warning, not a rejection."),
    ("RT-logP watchlist (checked by hand)", "Level 3 candidate reviewed by hand against the lower-ranked CSI:FingerID candidates and RT."),
    ("Level 1 confirmed by the analyst on the MS/MS mirror plot", "Level 1 whose MS/MS match was borderline and was confirmed by the analyst on the mirror plot."),
    ("adduct inferred from m/z", "the evidence source gives no ion type: the common adduct closest to the measured m/z is reported."),
    ("DETECTED within Nx procedural blank at: ...", "DETECTED at those sites, but no feature above N x the procedural blank: not counted as present there."),
];

pub const FLAG_MZLIB_ONLY: &str = "Level 2a from MZmine library only";
pub const FLAG_NAME_CONFLICT: &str =
    "Level 3: library matches of different structures, not told apart by the spectrum";
pub const FLAG_LIBRARY_CHOICE: &str =
    "libraries name different structures: the best match names the compound";
pub const FLAG_EXPERT: &str = "Level 3: library match judged insufficient by the analyst";
pub const FLAG_2B: &str = "Level 2b: structure supported by diagnostic evidence, judged by the analyst on the MS/MS spectrum";
pub const FLAG_ISOMERS: &str = "isomers not discriminated";
pub const FLAG_RT_ISOMER: &str = "Level 3: library match at a retention time other than the reference standard's (another isomer)";
pub const FLAG_RT_SPLIT: &str = "the same name at another retention time is a separate identity";
pub const FLAG_RT_LOGP: &str = "RT-logP outlier";
pub const FLAG_WATCHLIST: &str = "RT-logP watchlist (checked by hand)";
pub const FLAG_L1_MANUAL: &str = "Level 1 confirmed by the analyst on the MS/MS mirror plot";
pub const FLAG_ADDUCT: &str = "adduct inferred from m/z";
