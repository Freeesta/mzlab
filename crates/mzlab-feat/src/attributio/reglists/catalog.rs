// Ported from Attributio reglists/catalog.py.
//! Which regulatory and priority lists exist, in which group, built by which source reader. The catalogue is the same for every campaign; what changes with each
//! version of a list (act, version, dates, URL, licence) is data that lives next to the list's primary source.

/// Groups of lists, in the order of every table, with their English names.
pub const GROUPS: [(&str, &str); 6] = [
    ("acque_UE", "EU water legislation"),
    ("chimici_UE", "EU chemicals legislation"),
    ("pesticidi_HHP", "Pesticides and HHPs"),
    ("internazionali", "International conventions and lists"),
    ("thailandia", "Thailand"),
    ("italia", "Italy"),
];

/// One list of the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSpec {
    pub id: &'static str,
    pub group: &'static str,
    /// Short English label (tables, figures).
    pub label: &'static str,
    /// What the list is.
    pub title: &'static str,
    /// Source reader that builds it (`None`: not buildable here).
    pub reader: Option<&'static str>,
    /// The question of the papers it answers.
    pub question: &'static str,
    /// Why it is not built, or a caveat.
    pub note: &'static str,
}

pub const LISTS: [ListSpec; 28] = [
    ListSpec { id: "EU_PS_2013_39", group: "acque_UE", label: "EU priority substances (Dir. 2013/39/EU)", title: "Priority and priority hazardous substances, Directive 2008/105/EC Annex I as amended by Directive 2013/39/EU (in force until transposition of 2026/805)", reader: Some("eu_water"), question: "chemical status of surface waters", note: "" },
    ListSpec { id: "EU_PS_2026_805", group: "acque_UE", label: "EU priority substances (Dir. (EU) 2026/805)", title: "Priority substances after revision: Directive (EU) 2026/805 (amending 2000/60/EC, 2006/118/EC and 2008/105/EC)", reader: Some("eu_water"), question: "chemical status of surface waters", note: "" },
    ListSpec { id: "EU_WL_2022_1307", group: "acque_UE", label: "EU surface water Watch List 2022", title: "Fourth Watch List, Implementing Decision (EU) 2022/1307", reader: Some("eu_water"), question: "", note: "" },
    ListSpec { id: "EU_WL_2025_439", group: "acque_UE", label: "EU surface water Watch List 2025", title: "Fifth Watch List, Implementing Decision (EU) 2025/439", reader: Some("eu_water"), question: "", note: "" },
    ListSpec { id: "UWWTD_2024_3019", group: "acque_UE", label: "UWWTD indicators (Dir. (EU) 2024/3019)", title: "Quaternary treatment indicator substances, Directive (EU) 2024/3019, Annex I, Tab. 3, Note 1", reader: Some("eu_water"), question: "urban wastewater tracers", note: "" },
    ListSpec { id: "EU_DWD_2020_2184", group: "acque_UE", label: "EU Drinking Water Directive parameters", title: "Chemical parameters of Directive (EU) 2020/2184, Annex I part B and PFAS of Annex III", reader: Some("voci"), question: "", note: "" },
    ListSpec { id: "EU_DWD_WL", group: "acque_UE", label: "EU drinking water Watch List", title: "Drinking water Watch List (art. 13(8) of Directive 2020/2184)", reader: Some("eu_water"), question: "", note: "" },
    ListSpec { id: "EU_GWD", group: "acque_UE", label: "EU Groundwater Directive", title: "Directive 2006/118/EC, Annex I and II, as amended by Directive (EU) 2026/805", reader: Some("eu_water"), question: "", note: "" },
    ListSpec { id: "SVHC", group: "chimici_UE", label: "REACH SVHC Candidate List", title: "ECHA Candidate List of SVHCs (REACH art. 59), mono-constituents only", reader: Some("svhc"), question: "plastic additives and SVHCs", note: "" },
    ListSpec { id: "REACH_XIV", group: "chimici_UE", label: "REACH Annex XIV (authorisation)", title: "Annex XIV of Regulation (EC) No 1907/2006 (substances subject to authorisation)", reader: Some("eu_chem"), question: "", note: "" },
    ListSpec { id: "REACH_XVII", group: "chimici_UE", label: "REACH Annex XVII (restrictions)", title: "Annex XVII of Regulation (EC) No 1907/2006 (restrictions), entries with CAS number", reader: Some("eu_chem"), question: "", note: "" },
    ListSpec { id: "CLP_ANNEX_VI", group: "chimici_UE", label: "CLP Annex VI (harmonised: Aquatic 1, CMR, new classes)", title: "Harmonised classification, Reg. (EC) No 1272/2008 Annex VI Tab. 3: Aquatic Acute 1 / Chronic 1, CMR 1A/1B, new classes of Delegated Reg. (EU) 2023/707", reader: Some("eu_chem"), question: "", note: "" },
    ListSpec { id: "UBAPMT", group: "chimici_UE", label: "PMT/vPvM (UBA)", title: "PMT/vPvM candidates of the German Environment Agency (Arp & Hale), NORMAN-SLE S36", reader: Some("uba_pmt"), question: "PMT/vPvM among contaminants", note: "" },
    ListSpec { id: "EDLISTS", group: "chimici_UE", label: "Endocrine disruptor lists (edlists.org)", title: "Endocrine disruptor lists (edlists.org): I identified at EU level, II under evaluation at EU level, III identified at national level", reader: Some("international"), question: "", note: "" },
    ListSpec { id: "PlastChem", group: "chimici_UE", label: "PlastChem Red List", title: "PlastChem, only Red List (chemicals of concern), Wagner et al. 2024", reader: Some("plastchem"), question: "plastic additives", note: "" },
    ListSpec { id: "HHP_PAN", group: "pesticidi_HHP", label: "HHP (PAN International)", title: "PAN International List of Highly Hazardous Pesticides (+ documented TPs)", reader: Some("hhp_pan"), question: "HHPs: TH and EU status", note: "" },
    ListSpec { id: "EU_PPP", group: "pesticidi_HHP", label: "EU pesticides: approval status", title: "Active substances of plant protection products, EU Pesticides Database (Reg. (EC) No 1107/2009): approved, not approved, candidates for substitution", reader: Some("eu_ppp"), question: "banned in the EU but used elsewhere", note: "" },
    ListSpec { id: "WHO_PEST_HAZARD", group: "pesticidi_HHP", label: "WHO pesticide hazard classes Ia/Ib/II", title: "WHO Recommended Classification of Pesticides by Hazard (classes Ia, Ib, II)", reader: Some("international"), question: "", note: "" },
    ListSpec { id: "STOCKHOLM_POPS", group: "internazionali", label: "Stockholm Convention POPs", title: "Stockholm Convention, Annexes A, B and C", reader: Some("voci"), question: "", note: "" },
    ListSpec { id: "ROTTERDAM_PIC", group: "internazionali", label: "Rotterdam Convention Annex III (PIC)", title: "Rotterdam Convention, Annex III (PIC procedure), in the EU text of Reg. (EU) No 649/2012, Annex I part 3", reader: Some("eu_chem"), question: "", note: "" },
    ListSpec { id: "WHO_MIA", group: "internazionali", label: "WHO MIA / AWaRe", title: "WHO Medically Important Antimicrobials List + AWaRe classification", reader: Some("who_mia"), question: "antimicrobials (aquaculture)", note: "" },
    ListSpec { id: "VET_BANNED", group: "internazionali", label: "Vet drugs banned in food animals (EU/TH)", title: "Substances banned in food-producing animals: Reg. (EU) 37/2010 Tab. 2 (EU) and MOPH Notif. 268/2003 as amended by 299/2006 (TH)", reader: Some("voci"), question: "banned veterinary drugs", note: "" },
    ListSpec { id: "EPA_CCL5", group: "internazionali", label: "US EPA Contaminant Candidate List 5", title: "US EPA Drinking Water Contaminant Candidate List 5 (CCL 5), chemical substances", reader: Some("international"), question: "", note: "" },
    ListSpec { id: "EPA_CCL6_DRAFT", group: "internazionali", label: "US EPA draft Contaminant Candidate List 6", title: "US EPA Draft Sixth Contaminant Candidate List (CCL 6), chemical substances (draft, FR 6.4.2026)", reader: Some("international"), question: "", note: "draft: final CCL 6 is expected for November 2026" },
    ListSpec { id: "TH_HAZ_BANNED", group: "thailandia", label: "Thai banned agricultural hazardous substances", title: "Agricultural hazardous substances banned in Thailand (Hazardous Substance Act B.E. 2535, Type 4), DOA compilation of the Ministry of Industry lists", reader: Some("voci"), question: "", note: "includes MOI List No. 8 B.E. 2568; List No. 7 B.E. 2565 adds no DOA Type 4 substance" },
    ListSpec { id: "TH_PCD_SW", group: "thailandia", label: "Thai surface water quality standards", title: "Surface water quality standards (NEB Notif. No. 8, B.E. 2537), PCD", reader: Some("voci"), question: "", note: "" },
    ListSpec { id: "TH_DOF_AQUA", group: "thailandia", label: "Thai substances banned in animal and aquatic feed", title: "Substances prohibited as ingredients of animal feed, aquatic feed included (MOAC Notif. B.E. 2559, Animal Feed Quality Control Act B.E. 2558)", reader: Some("voci"), question: "", note: "historical ID: no readable DOF list of drugs banned in aquaculture exists online" },
    ListSpec { id: "IT_DLGS172_TAB1B", group: "italia", label: "Italy: specific pollutants (D.Lgs. 172/2015, Tab. 1/B)", title: "Specific pollutants (Tab. 1/B), Annex 1 to Part III of D.Lgs. 152/2006 as amended by D.Lgs. 172/2015", reader: Some("voci"), question: "", note: "official GU text still to be saved in fonte/ (Tab. 1/A = EU_PS_2013_39)" },
];

/// The catalogue entry of a list; an unknown id gets an entry with no group whose label is the id.
pub fn spec(list_id: &str) -> Option<&'static ListSpec> {
    LISTS.iter().find(|s| s.id == list_id)
}

/// The label of a list (its id when the list is not in the catalogue).
pub fn label(list_id: &str) -> String {
    spec(list_id).map_or_else(|| list_id.to_string(), |s| s.label.to_string())
}

/// The question of a list (empty when unknown).
pub fn question(list_id: &str) -> &'static str {
    spec(list_id).map_or("", |s| s.question)
}

/// The group of a list (empty when unknown).
pub fn group_of(list_id: &str) -> &'static str {
    spec(list_id).map_or("", |s| s.group)
}

/// Position of a list in the catalogue order.
pub fn order_of(list_id: &str) -> Option<usize> {
    LISTS.iter().position(|s| s.id == list_id)
}

/// The ids without repeats: catalogue lists first in catalogue order, then the others sorted.
pub fn ordered(ids: &[&str]) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    for i in ids {
        if !seen.contains(i) {
            seen.push(i);
        }
    }
    let mut out: Vec<String> = LISTS
        .iter()
        .filter(|s| seen.contains(&s.id))
        .map(|s| s.id.to_string())
        .collect();
    let mut rest: Vec<&str> = seen.into_iter().filter(|i| order_of(i).is_none()).collect();
    rest.sort();
    out.extend(rest.into_iter().map(str::to_string));
    out
}
