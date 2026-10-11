"""Path of the Pratica EI (tools/genera_ei.py): compounds per level and the replacement report."""

# Path of simple compounds for the Pratica, per level (1-5) and ordered by difficulty inside the level. The level is about WHAT the
# student has to read in the spectrum: 1 molecular ion and isotopes, 2 simple cleavages, 3 rearrangements and ring/chain
# differentiation, 4 aromatics and series, 5 complex diagnosis (several classes together). The ids are the ids of LIST: the
# progress in localStorage qqq.pratica is keyed by the same ids, so entering the path changes nothing for existing users.
PATH = {
    1: ["pentano", "dodecano", "bromoetano", "1-bromopropano", "clorobenzene", "bromobenzene", "1-clorobutano"],
    2: ["2-butanone", "3-pentanone", "3-metil-2-butanone", "butanale", "1-butanolo", "2-butanolo", "2-metil-2-butanolo", "3-metil-3-pentanolo", "2-metil-2-pentanolo",
        "dietil-etere", "metil-butil-etere", "diisopropil-etere", "2-metileptano", "1-decene", "2-bromopentano", "2-clorobutano",
        "butilammina", "dietilammina", "sec-butilammina", "isobutilammina", "terz-butilammina", "dipropilammina", "dibutilammina", "trietilammina"],
    3: ["4-metil-2-pentanone", "3-metil-2-pentanone", "2-esanone", "2-ottanone", "2-dodecanone", "3-dodecanone", "ottanale", "2-etilesanale", "6-metil-5-epten-2-one",
        "acido-butanoico", "acido-esanoico", "acido-dodecanoico", "butanoato-di-etile", "acetato-di-sec-butile", "esanoato-di-metile", "undecanoato-di-metile", "dodecanoato-di-metile",
        "benzoato-di-metile", "anidride-maleica", "cicloesene", "cicloesanolo", "2-metilcicloesanolo", "butilcicloesano", "limonene", "alfa-ionone", "beta-ionone",
        "tetraidrofurano", "2-metiltetraidrofurano", "tetraidropirano", "diesil-etere", "di-sec-butil-etere", "isobutil-terz-butil-etere", "dibutilsolfuro",
        "1-dodecanolo", "1-bromododecano", "1-bromoottano", "esanonitrile", "decanonitrile", "n-metilacetammide", "dimetilacetammide", "o-cresolo", "2-nitrotoluene"],
    4: ["toluene", "etilbenzene", "propilbenzene", "butilbenzene", "o-xilene", "naftalene", "anisolo", "fenetolo", "benzaldeide", "acetofenone",
        "nitrobenzene", "4-nitrotoluene", "butanenitrile", "dietilsolfuro", "1-butantiolo", "anidride-ftalica", "m-cresolo", "p-cresolo", "2-etilfenolo", "4-etilfenolo",
        "1-esadecene", "3-etil-2,6,10-trimetilundecano", "metil-18-metilnonadecanoato", "4-bromoanisolo"],
    5: ["esadecano", "pentadecano", "farnesano", "diacetone-alcol", "levulinato-di-metile", "acido-levulinico", "vanillina", "4-metossiacetofenone", "4-cloroanilina",
        "acido-2-clorobenzoico", "acido-3-clorobenzoico", "acido-4-clorobenzoico", "acido-ftalico", "acido-isoftalico", "acido-tereftalico", "stearammide",
        "benzil-metil-solfuro", "tioanisolo"],
}
# What the task asked for and what was used instead (reported by main(); a candidate without an EI-B record in MassBank is replaced by an
# equivalent compound of the same class and the replacement is stated).
SUBST = {
    "1-bromobutano": ("1-bromopropano", "nessun record EI-B di 1-bromobutano: omologo con un carbonio in meno"),
    "3-metil-3-esanolo": ("3-metil-3-pentanolo; 2-metil-2-butanolo", "nessun record EI-B: due alcoli terziari della stessa classe"),
    "esanammide": ("N-metilacetammide", "nessuna ammide alifatica lunga in MassBank: ammide secondaria semplice"),
    "1-nitropropano": ("4-nitrotoluene (nitrobenzene già presente)", "nessun nitroalcano EI-B oltre il nitrometano già presente"),
    "2-metilpentadecano": ("2,6,10-trimetildodecano (farnesano); pentadecano", "nessun alcano monoramificato lungo: isoalcano ramificato di pari massa"),
    "cloroottano": ("4-cloroanilina", "nessun cloroalcano lungo EI-B: composto con cloro e un'altra classe"),
    "butirronitrile": ("butanenitrile (già presente)", "stesso composto"),
    "esilbenzene": ("(nessuno)", "unico record EI-B con picchi mancanti (manca m/z 91): scartato"),
}
SKILL_OF_TYPE = {"alpha": "alpha", "i": "induttiva", "sigma": "sigma", "mclafferty": "mclafferty", "rda": "rda", "tropilio": "benzilica", "orto": "orto",
                 "serie": "serie", "perdita": "perdita", "riarr": "riarr", "isotopo": "isotopi"}
