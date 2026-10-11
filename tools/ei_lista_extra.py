"""Second part of the compound list of tools/genera_ei.py (the compounds added to path levels 1-5; their spectra go to ei-dati-l<N>.js,
loaded by the Pratica only when that level is chosen). Same entry format as LIST; see genera_ei.py for the meaning of the fields."""


def K(mz, ion, typ, text, delta=0):
    """A key ion (see genera_ei.K)."""
    return {"mz": mz, "ion": ion, "type": typ, "text": text, **({"delta": delta} if delta else {})}


LIST_EXTRA = [
    # ---------------------------------------------------------------- second part of the path (files ei-dati-l1..l5.js, loaded by level)
    dict(id="dodecano", it="dodecano", en=["DODECANE", "N-DODECANE"], f="C12H26", fam="alcani", lv=2, extra=True, keys=[
        K(170, "C12H26+.", "M", "M debole ma presente: l'ultimo gradino della «staccionata»."),
        K(57, "C4H9+", "serie", "Picco base della serie alchilica."),
        K(43, "C3H7+", "serie", "Secondo massimo della serie, a intervalli di 14."),
        K(71, "C5H11+", "serie", "La serie scende con regolarità fino a M−29."),
        K(85, "C6H13+", "serie", "Ancora il passo di 14: scheletro non ramificato.")]),
    dict(id="2-metil-2-pentanolo", it="2-metil-2-pentanolo", en=["2-METHYL-2-PENTANOL"], f="C6H14O", fam="alcoli", lv=3, extra=True, keys=[
        K(59, "C3H7O+", "alpha", "Scissione α con perdita del radicale più grande (propile): picco base."),
        K(87, "C5H11O+", "alpha", "Scissione α con perdita di un metile (M−15)."),
        K(31, "CH3O+", "riarr", "Ione ossonio più piccolo, da un riarrangiamento del 59.")]),
    dict(id="2-metileptano", it="2-metileptano", en=["2-METHYLHEPTANE", "ISOOCTANE"], f="C8H18", fam="alcani", lv=3, extra=True, keys=[
        K(114, "C8H18+.", "M", "Lo ione molecolare, debolissimo."),
        K(57, "C4H9+", "sigma", "Picco base, come negli alcani lineari: da solo non distingue."),
        K(56, "C4H8+.", "riarr", "Ione a elettroni dispari: due rotture, tipico di uno scheletro ramificato."),
        K(99, "C7H15+", "sigma", "M−15: perdita di un metile da un punto ramificato.")]),
    dict(id="1-decene", it="1-decene", en=["1-DECENE"], f="C10H20", fam="alcheni", lv=3, extra=True, keys=[
        K(140, "C10H20+.", "M", "Lo ione molecolare: più visibile che in un alcano."),
        K(41, "C3H5+", "alpha", "Catione allile da scissione allilica (picco base)."),
        K(55, "C4H7+", "serie", "Serie CnH2n−1+ spostata di 2 unità rispetto agli alcani."),
        K(56, "C4H8+.", "riarr", "Ione a elettroni dispari di un alchene."),
        K(70, "C5H10+.", "riarr", "Altro ione alchenico a elettroni dispari.")]),
    dict(id="1-esadecene", it="1-esadecene", en=["1-HEXADECENE"], f="C16H32", fam="alcheni", lv=4, extra=True, keys=[
        K(224, "C16H32+.", "M", "M debole ma presente."),
        K(41, "C3H5+", "alpha", "Catione allile (picco base)."),
        K(55, "C4H7+", "serie", "Serie CnH2n−1+."),
        K(83, "C6H11+", "serie", "Un'insaturazione abbassa ogni ione della serie di 2 unità."),
        K(56, "C4H8+.", "riarr", "Ione alchenico a elettroni dispari.")]),
    dict(id="butilcicloesano", it="butilcicloesano", en=["BUTYLCYCLOHEXANE", "N-BUTYLCYCLOHEXANE"], f="C10H20", fam="alcheni", lv=3, extra=True, keys=[
        K(140, "C10H20+.", "M", "Lo ione molecolare, ben visibile."),
        K(83, "C6H11+", "sigma", "Perdita della catena intera (M−57), picco base: la rottura avviene dove l'anello si lega alla catena."),
        K(81, "C6H9+", "perdita", "83 − H2."),
        K(41, "C3H5+", "serie", "Catione allile."),
        K(56, "C4H8+.", "riarr", "Ione a elettroni dispari.")]),
    dict(id="1-dodecanolo", it="1-dodecanolo", en=["DODECANOL", "1-DODECANOL", "LAURYL ALCOHOL"], f="C12H26O", fam="alcoli", lv=3, extra=True, keys=[
        K(168, "C12H24+.", "perdita", "M−18: perdita di acqua; lo ione molecolare (186) non si vede."),
        K(31, "CH3O+", "alpha", "CH2=OH+: scissione α di un alcol primario."),
        K(55, "C4H7+", "serie", "Serie alchenica."),
        K(69, "C5H9+", "serie", "Serie alchenica."),
        K(56, "C4H8+.", "riarr", "Ione alchenico a elettroni dispari."),
        K(70, "C5H10+.", "riarr", "Ione alchenico a elettroni dispari.")]),
    dict(id="diesil-etere", it="diesil etere", en=["DIHEXYL ETHER", "HEXYL ETHER"], f="C12H26O", fam="eteri", lv=3, extra=True, keys=[
        K(85, "C6H13+", "i", "Scissione induttiva del legame C-O: catione esile."),
        K(84, "C6H12+.", "riarr", "Ione a elettroni dispari: alchene che porta la carica."),
        K(56, "C4H8+.", "riarr", "Ione alchenico a elettroni dispari."),
        K(43, "C3H7+", "serie", "Picco base della serie alchilica.")]),
    dict(id="di-sec-butil-etere", it="di-sec-butil etere", en=["DI-SEC-BUTYL ETHER"], f="C8H18O", fam="eteri", lv=3, extra=True, keys=[
        K(57, "C4H9+", "i", "Scissione induttiva: catione sec-butile (picco base)."),
        K(59, "C3H7O+", "riarr", "Ione ossonio: scissione e trasferimento di un H."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
    dict(id="isobutil-terz-butil-etere", it="isobutil terz-butil etere", en=["ISOBUTYL TERT-BUTYL ETHER"], f="C8H18O", fam="eteri", lv=3, extra=True, keys=[
        K(57, "C4H9+", "i", "Scissione induttiva: catione terz-butile, il più stabile."),
        K(45, "C2H5O+", "riarr", "Ione ossonio (picco base) dopo un riarrangiamento."),
        K(59, "C3H7O+", "riarr", "Ione ossonio da scissione e trasferimento di H.")]),
    dict(id="diisopropil-etere", it="diisopropil etere", en=["DIISOPROPYL ETHER", "ISOPROPYL ETHER"], f="C6H14O", fam="eteri", lv=2, extra=True, keys=[
        K(102, "C6H14O+.", "M", "Lo ione molecolare, debole."),
        K(87, "C5H11O+", "alpha", "Scissione α con perdita di un metile."),
        K(45, "C2H5O+", "riarr", "87 perde propene (87 − 42): picco base."),
        K(43, "C3H7+", "i", "Scissione induttiva: catione isopropile.")]),
    dict(id="dibutilsolfuro", it="dibutilsolfuro", en=["DIBUTYL SULFIDE", "BUTYL SULFIDE"], f="C8H18S", fam="zolfo", lv=3, extra=True, keys=[
        K(146, "C8H18S+.", "M", "M intenso, con il M+2 dello zolfo (4-5%)."),
        K(90, "C4H10S+.", "riarr", "Perdita di butene (M−56) con trasferimento di H: ione a elettroni dispari."),
        K(89, "C4H9S+", "i", "Perdita di un butile (M−57)."),
        K(61, "C2H5S+", "riarr", "Ione tipico con zolfo e due carboni, da riarrangiamento."),
        K(56, "C4H8+.", "perdita", "Il butene porta la carica.")]),
    dict(id="1-bromododecano", it="1-bromododecano", en=["1-BROMODODECANE", "DODECYL BROMIDE"], f="C12H25Br", fam="alogenati", lv=3, extra=True, keys=[
        K(135, "C4H8Br+", "riarr", "Ione ciclico con il bromo (4 carboni): il doppietto 135/137 resta forte."),
        K(137, "C4H8Br+", "isotopo", "Il gemello 81Br del 135.", 2),
        K(43, "C3H7+", "serie", "Picco base della serie alchilica."),
        K(57, "C4H9+", "serie", "La catena lunga dà la serie alchilica senza bromo.")]),
    dict(id="1-bromoottano", it="1-bromoottano", en=["1-BROMOOCTANE", "OCTYL BROMIDE"], f="C8H17Br", fam="alogenati", lv=3, extra=True, keys=[
        K(135, "C4H8Br+", "riarr", "Ione ciclico con il bromo: doppietto 135/137."),
        K(137, "C4H8Br+", "isotopo", "Il gemello 81Br del 135.", 2),
        K(57, "C4H9+", "serie", "Picco base della serie alchilica."),
        K(71, "C5H11+", "serie", "Serie alchilica senza bromo.")]),
    dict(id="decanonitrile", it="decanonitrile", en=["N-NONYL CYANIDE", "DECANENITRILE"], f="C10H19N", fam="azotati", lv=3, extra=True, keys=[
        K(41, "C3H5+", "serie", "Picco base: serie alchilica/allilica."),
        K(43, "C3H7+", "serie", "Serie alchilica."),
        K(54, "C3H4N+", "riarr", "Ione con l'azoto tipico dei nitrili alifatici a catena lunga.")]),
    dict(id="esanonitrile", it="esanonitrile", en=["HEXANENITRILE"], f="C6H11N", fam="azotati", lv=3, extra=True, keys=[
        K(41, "C3H5+", "serie", "Picco base: serie alchilica/allilica."),
        K(54, "C3H4N+", "riarr", "Ione con l'azoto, da riarrangiamento."),
        K(96, "C6H10N+", "perdita", "M−1: perdita di un H."),
        K(82, "C5H8N+", "perdita", "M−15: perdita di un metile.")]),
    dict(id="2-bromopentano", it="2-bromopentano", en=["2-BROMOPENTANE"], f="C5H11Br", fam="alogenati", lv=2, extra=True, keys=[
        K(71, "C5H11+", "i", "Scissione induttiva: perdita di Br• (M−79), catione sec-pentile (picco base)."),
        K(43, "C3H7+", "sigma", "Rottura successiva del catione 71."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
    dict(id="2-clorobutano", it="2-clorobutano", en=["2-CHLOROBUTANE", "SEC-BUTYL CHLORIDE"], f="C4H9Cl", fam="alogenati", lv=2, extra=True, keys=[
        K(57, "C4H9+", "i", "Scissione induttiva: perdita di Cl•, catione sec-butile (picco base)."),
        K(56, "C4H8+.", "perdita", "M−36: perdita di HCl, ione a elettroni dispari."),
        K(63, "C2H4Cl+", "alpha", "Scissione α con perdita di un etile (M−29)."),
        K(65, "C2H4Cl+", "isotopo", "Il 63 con il 37Cl: 3 : 1.", 2)]),
    dict(id="dibutilammina", it="dibutilammina", en=["DIBUTYLAMINE"], f="C8H19N", fam="azotati", lv=2, extra=True, keys=[
        K(129, "C8H19N+.", "M", "M dispari: un azoto."),
        K(86, "C5H12N+", "alpha", "Scissione α con perdita di un propile: picco base."),
        K(44, "C2H6N+", "riarr", "Ione iminio piccolo, dopo perdita di alchene dallo ione 86.")]),
    dict(id="dipropilammina", it="dipropilammina", en=["DI-N-PROPYLAMINE", "DIPROPYLAMINE"], f="C6H15N", fam="azotati", lv=2, extra=True, keys=[
        K(101, "C6H15N+.", "M", "M dispari: un azoto."),
        K(72, "C4H10N+", "alpha", "Scissione α con perdita di un etile: picco base."),
        K(30, "CH4N+", "riarr", "CH2=NH2+: ione iminio più piccolo."),
        K(100, "C6H14N+", "perdita", "M−1.")]),
    dict(id="2-etilesanale", it="2-etilesanale", en=["2-ETHYLHEXANAL"], f="C8H16O", fam="carbonilici", lv=3, extra=True, keys=[
        K(128, "C8H16O+.", "M", "Lo ione molecolare, debole."),
        K(72, "C4H8O+.", "mclafferty", "McLafferty con perdita di butene: ione enolico a elettroni dispari (picco base)."),
        K(57, "C4H9+", "i", "Catione butile."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
    dict(id="2-ottanone", it="2-ottanone", en=["2-OCTANONE"], f="C8H16O", fam="carbonilici", lv=3, extra=True, keys=[
        K(128, "C8H16O+.", "M", "Lo ione molecolare."),
        K(58, "C3H6O+.", "mclafferty", "McLafferty con perdita di pentene: ione enolico a elettroni dispari (picco base)."),
        K(71, "C4H7O+", "sigma", "Rottura γ-δ della catena."),
        K(43, "C2H3O+", "alpha", "Acetile da scissione α.")]),
    dict(id="6-metil-5-epten-2-one", it="6-metil-5-epten-2-one", en=["6-METHYL-5-HEPTEN-2-ONE"], f="C8H14O", fam="carbonilici", lv=3, extra=True, keys=[
        K(43, "C2H3O+", "alpha", "Acetile da scissione α (picco base)."),
        K(108, "C8H12+.", "perdita", "M−18: perdita di acqua, ione a elettroni dispari."),
        K(69, "C5H9+", "i", "Catione allilico stabile dall'altra parte della catena."),
        K(111, "C7H11O+", "alpha", "M−15: perdita di un metile."),
        K(58, "C3H6O+.", "mclafferty", "McLafferty con un H vinilico: ione enolico.")]),
    dict(id="ottanale", it="ottanale", en=["OCTANAL"], f="C8H16O", fam="carbonilici", lv=3, extra=True, keys=[
        K(44, "C2H4O+.", "mclafferty", "McLafferty delle aldeidi: m/z 44 a elettroni dispari."),
        K(41, "C3H5+", "serie", "Picco base: serie alchilica/allilica."),
        K(84, "C6H12+.", "perdita", "M−44: perdita di etenolo, ione alchenico."),
        K(57, "C4H9+", "serie", "Serie alchilica.")]),
    dict(id="2-dodecanone", it="2-dodecanone", en=["2-DODECANONE"], f="C12H24O", fam="carbonilici", lv=3, extra=True, keys=[
        K(184, "C12H24O+.", "M", "Lo ione molecolare."),
        K(58, "C3H6O+.", "mclafferty", "McLafferty: ione enolico a elettroni dispari (picco base)."),
        K(43, "C2H3O+", "alpha", "Acetile da scissione α."),
        K(71, "C4H7O+", "sigma", "Rottura γ-δ."),
        K(85, "C5H9O+", "sigma", "Rottura δ-ε.")]),
    dict(id="3-dodecanone", it="3-dodecanone", en=["3-DODECANONE"], f="C12H24O", fam="carbonilici", lv=3, extra=True, keys=[
        K(184, "C12H24O+.", "M", "Lo ione molecolare."),
        K(72, "C4H8O+.", "mclafferty", "McLafferty: ione enolico a elettroni dispari (picco base)."),
        K(57, "C3H5O+", "alpha", "Acilio dal lato dell'etile."),
        K(155, "C10H19O+", "alpha", "M−29: perdita dell'etile."),
        K(29, "C2H5+", "i", "Catione etile.")]),
    dict(id="acido-dodecanoico", it="acido dodecanoico", en=["DODECANOIC ACID", "LAURIC ACID"], f="C12H24O2", fam="acidi", lv=3, extra=True, keys=[
        K(200, "C12H24O2+.", "M", "Lo ione molecolare."),
        K(60, "C2H4O2+.", "mclafferty", "McLafferty degli acidi: m/z 60 (picco base)."),
        K(73, "C3H5O2+", "sigma", "Rottura γ-δ."),
        K(129, "C7H13O2+", "sigma", "Rottura a metà catena."),
        K(45, "CHO2+", "alpha", "COOH+.")]),
    dict(id="undecanoato-di-metile", it="undecanoato di metile", en=["METHYL UNDECANOATE"], f="C12H24O2", fam="acidi", lv=3, extra=True, keys=[
        K(200, "C12H24O2+.", "M", "Lo ione molecolare."),
        K(74, "C3H6O2+.", "mclafferty", "McLafferty degli esteri metilici: m/z 74 (picco base)."),
        K(87, "C4H7O2+", "sigma", "Rottura γ-δ."),
        K(143, "C8H15O2+", "sigma", "Rottura a metà catena."),
        K(169, "C11H21O+", "alpha", "M−31: perdita del metossile.")]),
    dict(id="anidride-maleica", it="anidride maleica", en=["MALIC ANHYDRIDE", "MALEIC ANHYDRIDE"], f="C4H2O3", fam="acidi", lv=3, extra=True, keys=[
        K(98, "C4H2O3+.", "M", "Lo ione molecolare."),
        K(54, "C3H2O+.", "perdita", "M−44: perdita di CO2, la perdita tipica delle anidridi."),
        K(26, "C2H2+.", "perdita", "Acetilene (picco base) dopo la perdita di CO2 e CO.")]),
    dict(id="anidride-ftalica", it="anidride ftalica", en=["PHTHALIC ANHYDRIDE"], f="C8H4O3", fam="acidi", lv=4, extra=True, keys=[
        K(148, "C8H4O3+.", "M", "Lo ione molecolare, intenso."),
        K(104, "C7H4O+.", "perdita", "M−44: perdita di CO2 (picco base)."),
        K(76, "C6H4+.", "perdita", "104 − CO: benzino."),
        K(50, "C4H2+.", "perdita", "76 − acetilene.")]),
    dict(id="2-metilcicloesanolo", it="2-metilcicloesanolo", en=["2-METHYLCYCLOHEXANOL"], f="C7H14O", fam="alcoli", lv=3, extra=True, keys=[
        K(114, "C7H14O+.", "M", "Lo ione molecolare, debole."),
        K(57, "C3H5O+", "alpha", "Apertura dell'anello e seconda rottura (picco base)."),
        K(96, "C7H12+.", "perdita", "M−18: perdita di acqua."),
        K(81, "C6H9+", "perdita", "96 − CH3."),
        K(68, "C5H8+.", "riarr", "Ione a elettroni dispari dall'anello aperto.")]),
    dict(id="o-cresolo", it="o-cresolo", en=["O-CRESOL", "ORTHO-CRESOL"], f="C7H8O", fam="fenoli", lv=3, extra=True, keys=[
        K(108, "C7H8O+.", "M", "M picco base."),
        K(107, "C7H7O+", "perdita", "M−1."),
        K(90, "C7H6+.", "orto", "Effetto orto: M−18, perdita di acqua tra i due sostituenti adiacenti (nel meta e nel para è debole)."),
        K(79, "C6H7+", "perdita", "M−29: perdita di CHO.")]),
    dict(id="m-cresolo", it="m-cresolo", en=["M-CRESOL", "META-CRESOL"], f="C7H8O", fam="fenoli", lv=4, extra=True, keys=[
        K(108, "C7H8O+.", "M", "M picco base."),
        K(107, "C7H7O+", "perdita", "M−1."),
        K(79, "C6H7+", "perdita", "M−29: perdita di CHO. La perdita di acqua (M−18) è debole: nessun effetto orto.")]),
    dict(id="p-cresolo", it="p-cresolo", en=["P-CRESOL", "PARA-CRESOL"], f="C7H8O", fam="fenoli", lv=4, extra=True, keys=[
        K(108, "C7H8O+.", "M", "M picco base."),
        K(107, "C7H7O+", "perdita", "M−1: nel para molto intenso."),
        K(79, "C6H7+", "perdita", "M−29: perdita di CHO.")]),
    dict(id="2-etilfenolo", it="2-etilfenolo", en=["ORTHO-ETHYLPHENOL", "2-ETHYLPHENOL"], f="C8H10O", fam="fenoli", lv=4, extra=True, keys=[
        K(122, "C8H10O+.", "M", "Lo ione molecolare."),
        K(107, "C7H7O+", "tropilio", "Scissione benzilica: perdita di un metile (picco base)."),
        K(77, "C6H5+", "perdita", "Fenile."),
        K(79, "C6H7+", "perdita", "107 − CO.")]),
    dict(id="4-etilfenolo", it="4-etilfenolo", en=["4-ETHYLPHENOL"], f="C8H10O", fam="fenoli", lv=4, extra=True, keys=[
        K(122, "C8H10O+.", "M", "Lo ione molecolare, intenso."),
        K(107, "C7H7O+", "tropilio", "Scissione benzilica: perdita di un metile (picco base)."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="2-metiltetraidrofurano", it="2-metiltetraidrofurano", en=["2-METHYLTETRAHYDROFURAN"], f="C5H10O", fam="eteri", lv=3, extra=True, keys=[
        K(86, "C5H10O+.", "M", "Lo ione molecolare."),
        K(71, "C4H7O+", "alpha", "Scissione α con perdita del metile (picco base)."),
        K(43, "C2H3O+", "riarr", "Frammento dall'anello aperto."),
        K(42, "C3H6+.", "perdita", "Ione a elettroni dispari dall'anello aperto.")]),
    dict(id="tetraidropirano", it="tetraidropirano", en=["TETRAHYDROPYRAN"], f="C5H10O", fam="eteri", lv=3, extra=True, keys=[
        K(86, "C5H10O+.", "M", "Lo ione molecolare, debole."),
        K(85, "C5H9O+", "alpha", "M−1: perdita di un H in α all'ossigeno."),
        K(56, "C4H8+.", "perdita", "M−30: perdita di formaldeide, ione a elettroni dispari."),
        K(45, "C2H5O+", "riarr", "Ione ossonio dall'anello aperto."),
        K(41, "C3H5+", "serie", "Picco base: catione allile.")]),
    dict(id="tetraidrofurano", it="tetraidrofurano", en=["TETRAHYDROFURAN"], f="C4H8O", fam="eteri", lv=3, extra=True, keys=[
        K(72, "C4H8O+.", "M", "Lo ione molecolare."),
        K(71, "C4H7O+", "alpha", "M−1: perdita di un H in α all'ossigeno."),
        K(42, "C3H6+.", "perdita", "M−30: perdita di formaldeide (picco base)."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
    dict(id="metil-18-metilnonadecanoato", it="metil 18-metilnonadecanoato", en=["18-METHYLNONADECANOIC ACID METHYL ESTER", "METHYL 18-METHYLNONADECANOATE"], f="C21H42O2", fam="acidi", lv=4, extra=True, keys=[
        K(326, "C21H42O2+.", "M", "Lo ione molecolare."),
        K(74, "C3H6O2+.", "mclafferty", "McLafferty degli esteri metilici (picco base)."),
        K(87, "C4H7O2+", "sigma", "Rottura γ-δ."),
        K(143, "C8H15O2+", "sigma", "Rottura a metà catena."),
        K(283, "C18H35O2+", "sigma", "M−43: perdita di un propile terminale, sporge rispetto alla serie regolare.")]),
    dict(id="dodecanoato-di-metile", it="dodecanoato di metile", en=["METHYL DODECANOATE", "METHYL LAURATE"], f="C13H26O2", fam="acidi", lv=3, extra=True, keys=[
        K(214, "C13H26O2+.", "M", "Lo ione molecolare."),
        K(74, "C3H6O2+.", "mclafferty", "McLafferty degli esteri metilici (picco base)."),
        K(143, "C8H15O2+", "sigma", "Rottura a metà catena."),
        K(171, "C10H19O2+", "sigma", "M−43.")]),
    dict(id="acetato-di-sec-butile", it="acetato di sec-butile", en=["SEC-BUTYL ACETATE"], f="C6H12O2", fam="acidi", lv=3, extra=True, keys=[
        K(43, "C2H3O+", "alpha", "Acetile (picco base)."),
        K(87, "C4H7O2+", "alpha", "Scissione α dal lato del ramo: perdita di un etile (M−29)."),
        K(56, "C4H8+.", "mclafferty", "Perdita di acido acetico (M−60): ione alchenico a elettroni dispari."),
        K(61, "C2H5O2+", "riarr", "Acido acetico protonato (doppio trasferimento di H).")]),
    dict(id="2-nitrotoluene", it="2-nitrotoluene", en=["ORTHO NITROTOLUENE", "2-NITROTOLUENE"], f="C7H7NO2", fam="azotati", lv=3, extra=True, keys=[
        K(65, "C5H5+", "perdita", "Picco base: tropilio − acetilene."),
        K(120, "C7H6NO+", "perdita", "M−17: perdita di OH•, effetto orto (nel para è assente)."),
        K(91, "C7H7+", "tropilio", "M−46: perdita di NO2."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="isobutilbenzene", it="isobutilbenzene", en=["ISOBUTYLBENZENE"], f="C10H14", fam="aromatici", lv=0, play=False, extra=True, keys=[]),
    dict(id="sec-butilbenzene", it="sec-butilbenzene", en=["SEC-BUTYLBENZENE"], f="C10H14", fam="aromatici", lv=0, play=False, extra=True, keys=[]),
    dict(id="3-etil-2,6,10-trimetilundecano", it="3-etil-2,6,10-trimetilundecano", en=["3-ETHYL-2,6,10-TRIMETHYLUNDECANE"], f="C16H34", fam="alcani", lv=4, extra=True, keys=[
        K(57, "C4H9+", "serie", "Picco base, come nell'alcano lineare."),
        K(71, "C5H11+", "serie", "Serie alchilica più intensa del normale: segnala punti di rottura preferiti."),
        K(85, "C6H13+", "serie", "Serie alchilica."),
        K(141, "C10H21+", "sigma", "Ione che sporge sopra la serie regolare: rottura a un punto ramificato.")]),
    dict(id="4-bromoanisolo", it="4-bromoanisolo", en=["PARA-BROMOANISOLE", "4-BROMOANISOLE"], f="C7H7BrO", fam="eteri", lv=4, extra=True, keys=[
        K(186, "C7H7BrO+.", "M", "M picco base, con il doppietto del bromo."),
        K(188, "C7H7BrO+.", "isotopo", "M+2 quasi uguale a M: un bromo.", 2),
        K(171, "C6H4BrO+", "perdita", "M−15: perdita del metile."),
        K(143, "C5H4Br+", "perdita", "171 − CO."),
        K(63, "C5H3+", "perdita", "Dopo la perdita del bromo.")]),
    dict(id="fenetolo", it="fenetolo", en=["PHENETOLE", "ETHOXYBENZENE"], f="C8H10O", fam="eteri", lv=4, extra=True, keys=[
        K(122, "C8H10O+.", "M", "Lo ione molecolare."),
        K(94, "C6H6O+.", "riarr", "M−28: perdita di etene con trasferimento di H (picco base)."),
        K(66, "C5H6+.", "perdita", "94 − CO."),
        K(65, "C5H5+", "perdita", "66 − H.")]),
    dict(id="acido-4-clorobenzoico", it="acido 4-clorobenzoico", en=["4-CHLOROBENZOIC ACID"], f="C7H5ClO2", fam="acidi", lv=4, extra=True, keys=[
        K(156, "C7H5ClO2+.", "M", "Lo ione molecolare."),
        K(158, "C7H5ClO2+.", "isotopo", "M+2 circa un terzo di M: un cloro.", 2),
        K(139, "C7H4ClO+", "alpha", "M−17: perdita di OH, acilio (picco base)."),
        K(141, "C7H4ClO+", "isotopo", "Il 139 con il 37Cl.", 2),
        K(111, "C6H4Cl+", "perdita", "139 − CO.")]),
    dict(id="acido-2-clorobenzoico", it="acido 2-clorobenzoico", en=["2-CHLOROBENZOIC ACID", "ORTHO-CHLOROBENZOIC ACID"], f="C7H5ClO2", fam="acidi", lv=4, extra=True, keys=[
        K(156, "C7H5ClO2+.", "M", "Lo ione molecolare."),
        K(158, "C7H5ClO2+.", "isotopo", "M+2 circa un terzo di M.", 2),
        K(139, "C7H4ClO+", "alpha", "M−17: perdita di OH (picco base)."),
        K(111, "C6H4Cl+", "perdita", "139 − CO."),
        K(75, "C6H3+", "perdita", "111 − HCl, più intenso che nell'isomero para.")]),
    dict(id="acido-3-clorobenzoico", it="acido 3-clorobenzoico", en=["3-CHLOROBENZOIC ACID", "META-CHLOROBENZOIC ACID"], f="C7H5ClO2", fam="acidi", lv=4, extra=True, keys=[
        K(156, "C7H5ClO2+.", "M", "Lo ione molecolare."),
        K(158, "C7H5ClO2+.", "isotopo", "M+2 circa un terzo di M.", 2),
        K(139, "C7H4ClO+", "alpha", "M−17: perdita di OH (picco base)."),
        K(111, "C6H4Cl+", "perdita", "139 − CO (più intenso che negli altri due isomeri).")]),
    dict(id="acido-tereftalico", it="acido tereftalico", en=["TEREPHTHALIC ACID"], f="C8H6O4", fam="acidi", lv=4, extra=True, keys=[
        K(166, "C8H6O4+.", "M", "Lo ione molecolare, intenso."),
        K(149, "C8H5O3+", "alpha", "M−17: perdita di OH (picco base)."),
        K(121, "C7H5O2+", "perdita", "149 − CO."),
        K(65, "C5H5+", "perdita", "Dopo perdite ripetute di CO e CO2.")]),
    dict(id="acido-isoftalico", it="acido isoftalico", en=["ISOPHTHALIC ACID"], f="C8H6O4", fam="acidi", lv=4, extra=True, keys=[
        K(166, "C8H6O4+.", "M", "Lo ione molecolare, intenso."),
        K(149, "C8H5O3+", "alpha", "M−17: perdita di OH (picco base)."),
        K(121, "C7H5O2+", "perdita", "149 − CO."),
        K(65, "C5H5+", "perdita", "Dopo perdite ripetute.")]),
    dict(id="acido-ftalico", it="acido ftalico", en=["PHTHALIC ACID"], f="C8H6O4", fam="acidi", lv=4, extra=True, keys=[
        K(149, "C8H5O3+", "alpha", "Anidride ftalica protonata: effetto orto."),
        K(122, "C7H6O2+.", "perdita", "Perdita di CO2 (dopo disidratazione termica nella sorgente)."),
        K(105, "C7H5O+", "alpha", "Benzoile."),
        K(104, "C7H4O+.", "orto", "Effetto orto: perdita sequenziale di acqua e CO2.")]),
    dict(id="stearammide", it="stearammide", en=["STEARYLAMIDE", "STEARAMIDE", "OCTADECANAMIDE"], f="C18H37NO", fam="azotati", lv=4, extra=True, keys=[
        K(283, "C18H37NO+.", "M", "M dispari: un azoto."),
        K(59, "C2H5NO+.", "mclafferty", "McLafferty delle ammidi primarie: m/z 59 (picco base)."),
        K(72, "C3H6NO+", "sigma", "Rottura γ-δ."),
        K(44, "CH2NO+", "alpha", "O=C=NH2+.")]),
    dict(id="benzil-metil-solfuro", it="benzil metil solfuro", en=["BENZYL METHYL SULFIDE"], f="C8H10S", fam="zolfo", lv=4, extra=True, keys=[
        K(138, "C8H10S+.", "M", "M ben visibile, con il M+2 dello zolfo."),
        K(91, "C7H7+", "tropilio", "M−47: perdita di CH3S• e tropilio (picco base)."),
        K(123, "C7H7S+", "perdita", "M−15."),
        K(65, "C5H5+", "perdita", "Il tropilio perde acetilene."),
        K(47, "CH3S+", "i", "CH3S+: scissione induttiva.")]),
    dict(id="tioanisolo", it="tioanisolo", en=["THIOANISOLE", "METHYL PHENYL SULFIDE"], f="C7H8S", fam="zolfo", lv=4, extra=True, keys=[
        K(124, "C7H8S+.", "M", "M picco base, con il M+2 dello zolfo."),
        K(109, "C6H5S+", "perdita", "M−15: perdita del metile."),
        K(78, "C6H6+.", "perdita", "M−46: perdita di CH2S."),
        K(91, "C7H7+", "perdita", "M−33: perdita di SH•."),
        K(45, "CHS+", "perdita", "CHS+.")]),
]
