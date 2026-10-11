"""Builds mzlab/web/teoria/pratica/ei-dati.js: real 70 eV EI spectra for the Teoria (chapters 15-16) and the Pratica game
«Dallo spettro alla struttura». Run by the developer (needs internet), the result is committed; the site never calls MassBank.

    python3 tools/genera_ei.py            # downloads into .verifica/cache_massbank/ (kept between runs), writes ei-dati.js

Source: MassBank Europe (https://massbank.eu), REST API. The EI-B records are almost all CC BY-NC-SA (Univ. Tokyo, Koga M. /
UOEH and others): fine for this non-commercial teaching site, with attribution (accession, authors, licence) kept in every item
and shown by the game; the data file is therefore under CC BY-NC-SA 4.0, not MIT (see LICENZE-TERZI.md). No NIST spectra.

For every compound of LIST: search the EI-B records by each synonym (the API search is a substring search; a record is kept only
if one of its names is exactly a synonym AND its formula is the expected one), download them, merge peaks to integer m/z, keep
only the records that contain every key ion, and take the most representative one = highest mean cosine (square-root
intensities) to the other records of the same compound. The number of records and the cosine are stored: EI spectra are
reproducible, not identical. The "keys" (teaching notes: which ion, which formula, which mechanism) are written here by hand and
checked by tests/test_ei_dati.py with mzlab.chem.elements (nominal mass, odd/even electrons, sub-formula of the compound).
"""
from __future__ import annotations

import json
import math
import time
import urllib.parse
import urllib.request
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))      # ei_lista_extra.py and ei_percorso.py live next to this file
from ei_lista_extra import LIST_EXTRA  # noqa: E402
from ei_percorso import PATH, SKILL_OF_TYPE, SUBST  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "mzlab" / "web" / "teoria" / "pratica" / "ei-dati.js"
CACHE = ROOT / ".verifica" / "cache_massbank"
API = "https://massbank.eu/MassBank-api/records"
OK_LICENSES = {"CC BY-NC-SA", "CC BY-NC", "CC BY-SA", "CC BY", "CC0"}   # a record under any other licence is never used
MIN_REL = 2          # peaks kept in the file: >= 0.2 % of the base peak (relative intensities in per mille, base = 999)


def K(mz, ion, typ, text, delta=0):
    """A key ion. typ: M, alpha, i, sigma, mclafferty, rda, tropilio, orto, serie, perdita, riarr, isotopo (then delta = +1/+2/+4)."""
    return {"mz": mz, "ion": ion, "type": typ, "text": text, **({"delta": delta} if delta else {})}


# id, Italian name, English synonyms (MassBank style), formula, family, level 1-4, keys. play=False: reference spectrum only
# (an isomer the game can show when a student proposes it). Families = sections of chapter 16.
LIST = [
    # ---------------------------------------------------------------- alkanes
    dict(id="decano", it="decano", en=["DECANE", "N-DECANE"], f="C10H22", fam="alcani", lv=1, keys=[
        K(142, "C10H22+.", "M", "Lo ione molecolare c'è ma è debole: gli alcani lineari si rompono facilmente."),
        K(43, "C3H7+", "serie", "Serie alchilica CnH2n+1+ (29, 43, 57, 71, 85…): la «staccionata» con massimi a 43 e 57."),
        K(57, "C4H9+", "serie", "Secondo massimo della serie: C3 e C4 sono i cationi più stabili prodotti dalle rotture σ e dalle isomerizzazioni."),
        K(71, "C5H11+", "serie", "La serie scende regolarmente verso la massa molecolare, senza ioni preferiti: catena lineare.")]),
    dict(id="esadecano", it="esadecano", en=["HEXADECANE", "N-HEXADECANE"], f="C16H34", fam="alcani", lv=2, keys=[
        K(226, "C16H34+.", "M", "M debole ma visibile: il gruppo più alto della staccionata."),
        K(57, "C4H9+", "serie", "Picco base nella serie alchilica: tipico degli alcani lineari lunghi."),
        K(85, "C6H13+", "serie", "La serie prosegue a intervalli di 14 (CH2) fino a M−29.")]),
    dict(id="esano", it="esano", en=["HEXANE", "N-HEXANE"], f="C6H14", fam="alcani", lv=2, keys=[
        K(86, "C6H14+.", "M", "Lo ione molecolare: massa pari, nessun azoto."),
        K(57, "C4H9+", "sigma", "Perdita di un etile (M−29) per rottura di un legame σ C-C."),
        K(43, "C3H7+", "sigma", "Perdita di un propile: rottura al centro della catena.")]),
    dict(id="2-metilpentano", it="2-metilpentano", en=["2-METHYLPENTANE", "ISOHEXANE"], f="C6H14", fam="alcani", lv=3, keys=[
        K(43, "C3H7+", "sigma", "Rottura al carbonio ramificato: si forma il catione isopropile, secondario e stabile (picco base)."),
        K(71, "C5H11+", "sigma", "Perdita di un metile dal carbonio ramificato (M−15): segnala un ramo metilico.")]),
    dict(id="2,2-dimetilbutano", it="2,2-dimetilbutano", en=["2,2-DIMETHYLBUTANE", "NEOHEXANE"], f="C6H14", fam="alcani", lv=3, keys=[
        K(57, "C4H9+", "sigma", "Catione terz-butile, terziario: picco base. M (86) quasi assente: le ramificazioni indeboliscono lo ione molecolare."),
        K(71, "C5H11+", "sigma", "M−15: perdita di un metile dal carbonio quaternario.")]),
    dict(id="isoottano", it="2,2,4-trimetilpentano (isoottano)", en=["2,2,4-TRIMETHYLPENTANE", "ISOOCTANE"], f="C8H18", fam="alcani", lv=3, keys=[
        K(57, "C4H9+", "sigma", "Catione terz-butile dal carbonio quaternario: picco base; lo ione molecolare (114) praticamente non si vede.")]),
    # ---------------------------------------------------------------- alkenes and cycloalkanes
    dict(id="1-esene", it="1-esene", en=["1-HEXENE", "HEXENE-1"], f="C6H12", fam="alcheni", lv=2, keys=[
        K(84, "C6H12+.", "M", "M ben visibile: il doppio legame stabilizza lo ione molecolare più che in un alcano."),
        K(41, "C3H5+", "alpha", "Catione allile da scissione allilica: stabilizzato per risonanza."),
        K(56, "C4H8+.", "mclafferty", "Ione a elettroni dispari (massa pari, senza N): trasferimento di γ-H al doppio legame e perdita di etene.")]),
    dict(id="cicloesano", it="cicloesano", en=["CYCLOHEXANE"], f="C6H12", fam="alcheni", lv=2, keys=[
        K(84, "C6H12+.", "M", "M intenso: un anello deve rompere due legami per perdere un frammento."),
        K(56, "C4H8+.", "perdita", "Perdita di etene (M−28) dopo l'apertura dell'anello: ione a elettroni dispari."),
        K(69, "C5H9+", "perdita", "M−15, perdita di un metile dopo l'apertura dell'anello.")]),
    dict(id="cicloesene", it="cicloesene", en=["CYCLOHEXENE"], f="C6H10", fam="alcheni", lv=2, keys=[
        K(82, "C6H10+.", "M", "Lo ione molecolare: RDB 2 (anello + doppio legame)."),
        K(67, "C5H7+", "perdita", "M−15: perdita di un metile (picco base)."),
        K(54, "C4H6+.", "rda", "Retro-Diels-Alder: l'anello si apre in butadiene (carica) ed etene (neutro).")]),
    dict(id="limonene", it="limonene", en=["LIMONENE", "D-LIMONENE", "DL-LIMONENE", "DIPENTENE"], f="C10H16", fam="alcheni", lv=3, keys=[
        K(136, "C10H16+.", "M", "Lo ione molecolare di un monoterpene, C10H16 (RDB 3)."),
        K(68, "C5H8+.", "rda", "Retro-Diels-Alder del cicloesene: isoprene radicale catione, picco base."),
        K(93, "C7H9+", "perdita", "M−43: perdita del gruppo isopropenile/propile."),
        K(121, "C9H13+", "perdita", "M−15, perdita di un metile.")]),
    # ---------------------------------------------------------------- aromatics
    dict(id="benzene", it="benzene", en=["BENZENE"], f="C6H6", fam="aromatici", lv=1, keys=[
        K(78, "C6H6+.", "M", "M picco base: l'anello aromatico stabilizza lo ione molecolare."),
        K(52, "C4H4+.", "perdita", "Perdita di acetilene (M−26), tipica degli aromatici."),
        K(51, "C4H3+", "serie", "Serie aromatica 39, 50-52, 63-65, 76-78.")]),
    dict(id="toluene", it="toluene", en=["TOLUENE", "METHYLBENZENE"], f="C7H8", fam="aromatici", lv=1, keys=[
        K(92, "C7H8+.", "M", "Lo ione molecolare, intenso."),
        K(91, "C7H7+", "tropilio", "M−1: lo ione benzile si riarrangia nel tropilio, catione aromatico a sette atomi (picco base)."),
        K(65, "C5H5+", "perdita", "Il tropilio perde acetilene (91 − 26): la coppia 91/65 è la firma dei composti benzilici.")]),
    dict(id="etilbenzene", it="etilbenzene", en=["ETHYLBENZENE"], f="C8H10", fam="aromatici", lv=2, keys=[
        K(106, "C8H10+.", "M", "Lo ione molecolare."),
        K(91, "C7H7+", "tropilio", "Scissione benzilica: perdita di un metile e tropilio (picco base)."),
        K(65, "C5H5+", "perdita", "91 − C2H2, come nel toluene.")]),
    dict(id="propilbenzene", it="propilbenzene", en=["PROPYLBENZENE", "N-PROPYLBENZENE"], f="C9H12", fam="aromatici", lv=3, keys=[
        K(120, "C9H12+.", "M", "Lo ione molecolare."),
        K(91, "C7H7+", "tropilio", "Scissione benzilica: perdita di un etile (picco base)."),
        K(92, "C7H8+.", "mclafferty", "Ione a elettroni dispari a massa pari: McLafferty con l'anello come accettore (perdita di etene); serve una catena di almeno tre carboni.")]),
    dict(id="butilbenzene", it="butilbenzene", en=["BUTYLBENZENE", "N-BUTYLBENZENE"], f="C10H14", fam="aromatici", lv=3, keys=[
        K(134, "C10H14+.", "M", "Lo ione molecolare."),
        K(91, "C7H7+", "tropilio", "Tropilio dalla scissione benzilica (perdita di un propile)."),
        K(92, "C7H8+.", "mclafferty", "McLafferty: perdita di propene, ione a elettroni dispari.")]),
    dict(id="naftalene", it="naftalene", en=["NAPHTHALENE"], f="C10H8", fam="aromatici", lv=1, keys=[
        K(128, "C10H8+.", "M", "M picco base e pochi frammenti: molecola aromatica molto stabile (RDB 7)."),
        K(102, "C8H6+.", "perdita", "Perdita di acetilene, debole.")]),
    dict(id="fenantrene", it="fenantrene", en=["PHENANTHRENE"], f="C14H10", fam="aromatici", lv=4, keys=[
        K(178, "C14H10+.", "M", "IPA a tre anelli: M picco base, pochissimi frammenti."),
        K(176, "C14H8+.", "perdita", "M−2: perdita di H2, tipica degli IPA.")]),
    # ---------------------------------------------------------------- alcohols
    dict(id="1-propanolo", it="1-propanolo", en=["1-PROPANOL", "N-PROPYL ALCOHOL", "PROPAN-1-OL"], f="C3H8O", fam="alcoli", lv=2, keys=[
        K(31, "CH3O+", "alpha", "Scissione α: CH2=OH+ (m/z 31), la firma degli alcoli primari (picco base)."),
        K(59, "C3H7O+", "alpha", "M−1: scissione α con perdita di un H."),
        K(60, "C3H8O+.", "M", "Lo ione molecolare, debole come in tutti gli alcoli.")]),
    dict(id="2-propanolo", it="2-propanolo", en=["2-PROPANOL", "ISOPROPYL ALCOHOL", "ISOPROPANOL"], f="C3H8O", fam="alcoli", lv=2, keys=[
        K(45, "C2H5O+", "alpha", "Scissione α con perdita di un metile: CH3-CH=OH+ (m/z 45), la firma di un alcol secondario (picco base).")]),
    dict(id="1-butanolo", it="1-butanolo", en=["1-BUTANOL", "N-BUTANOL", "BUTYL ALCOHOL", "BUTAN-1-OL"], f="C4H10O", fam="alcoli", lv=1, keys=[
        K(31, "CH3O+", "alpha", "CH2=OH+: alcol primario."),
        K(56, "C4H8+.", "perdita", "M−18: perdita di acqua (eliminazione 1,4); il picco a 56 è a elettroni dispari."),
        K(41, "C3H5+", "serie", "Catione allile dalla catena.")]),
    dict(id="2-butanolo", it="2-butanolo", en=["2-BUTANOL", "SEC-BUTYL ALCOHOL", "BUTAN-2-OL"], f="C4H10O", fam="alcoli", lv=3, keys=[
        K(45, "C2H5O+", "alpha", "Scissione α con perdita dell'etile, il radicale più grande (picco base)."),
        K(59, "C3H7O+", "alpha", "Scissione α con perdita del metile: meno favorita (regola del radicale più grande).")]),
    dict(id="terz-butanolo", it="2-metil-2-propanolo (terz-butanolo)", en=["2-METHYL-2-PROPANOL", "TERT-BUTYL ALCOHOL", "T-BUTANOL", "TERT-BUTANOL"], f="C4H10O", fam="alcoli", lv=3, keys=[
        K(59, "C3H7O+", "alpha", "Scissione α: perdita di un metile da un alcol terziario; M (74) non si vede.")]),
    dict(id="alcol-benzilico", it="alcol benzilico", en=["BENZYL ALCOHOL", "BENZENEMETHANOL"], f="C7H8O", fam="alcoli", lv=3, keys=[
        K(108, "C7H8O+.", "M", "Lo ione molecolare, intenso perché c'è l'anello."),
        K(79, "C6H7+", "riarr", "Riarrangiamento con perdita di CHO: tipico degli alcoli benzilici."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    # ---------------------------------------------------------------- ethers
    dict(id="dietil-etere", it="dietil etere", en=["DIETHYL ETHER", "ETHYL ETHER", "ETHOXYETHANE"], f="C4H10O", fam="eteri", lv=2, keys=[
        K(74, "C4H10O+.", "M", "Lo ione molecolare."),
        K(59, "C3H7O+", "alpha", "Scissione α: perdita di un metile e ione ossonio."),
        K(31, "CH3O+", "riarr", "CH2=OH+ dopo un riarrangiamento con perdita di etene dallo ione 59."),
        K(29, "C2H5+", "i", "Scissione induttiva: la carica va sul gruppo etile.")]),
    dict(id="dibutil-etere", it="dibutil etere", en=["DIBUTYL ETHER", "BUTYL ETHER", "N-BUTYL ETHER"], f="C8H18O", fam="eteri", lv=3, keys=[
        K(57, "C4H9+", "i", "Scissione induttiva del legame C-O: catione butile (picco base)."),
        K(87, "C5H11O+", "alpha", "Scissione α con perdita di un propile.")]),
    dict(id="anisolo", it="anisolo", en=["ANISOLE", "METHOXYBENZENE"], f="C7H8O", fam="eteri", lv=2, keys=[
        K(108, "C7H8O+.", "M", "M picco base: etere aromatico."),
        K(78, "C6H6+.", "perdita", "Perdita di formaldeide (M−30), tipica degli anisoli."),
        K(65, "C5H5+", "perdita", "M−43: perdita di CH3 e CO in successione.")]),
    # ---------------------------------------------------------------- aldehydes and ketones
    dict(id="butanale", it="butanale", en=["BUTANAL", "BUTYRALDEHYDE", "N-BUTYRALDEHYDE", "BUTYRIC ALDEHYDE"], f="C4H8O", fam="carbonilici", lv=3, keys=[
        K(72, "C4H8O+.", "M", "Lo ione molecolare."),
        K(44, "C2H4O+.", "mclafferty", "McLafferty delle aldeidi: ione enolico a m/z 44 (elettroni dispari), perdita di etene.")]),
    dict(id="benzaldeide", it="benzaldeide", en=["BENZALDEHYDE"], f="C7H6O", fam="carbonilici", lv=1, keys=[
        K(106, "C7H6O+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "M−1: scissione α con perdita dell'H aldeidico, catione benzoile."),
        K(77, "C6H5+", "perdita", "Il benzoile perde CO: catione fenile."),
        K(51, "C4H3+", "perdita", "Il fenile perde acetilene.")]),
    dict(id="3-pentanone", it="3-pentanone", en=["3-PENTANONE", "DIETHYL KETONE", "PENTAN-3-ONE"], f="C5H10O", fam="carbonilici", lv=2, keys=[
        K(86, "C5H10O+.", "M", "Lo ione molecolare."),
        K(57, "C3H5O+", "alpha", "Scissione α: perdita di un etile, catione acilio C2H5CO+ (picco base)."),
        K(29, "C2H5+", "i", "Scissione induttiva, oppure l'acilio 57 che perde CO.")]),
    dict(id="3-metil-2-butanone", it="3-metil-2-butanone", en=["3-METHYL-2-BUTANONE", "METHYL ISOPROPYL KETONE"], f="C5H10O", fam="carbonilici", lv=3, keys=[
        K(43, "C2H3O+", "alpha", "Scissione α con perdita dell'isopropile (il radicale più grande): acetile CH3CO+ (picco base)."),
        K(71, "C4H7O+", "alpha", "Scissione α dall'altra parte: perdita del metile."),
        K(86, "C5H10O+.", "M", "Stessa massa del 3-pentanone: lo spettro distingue i due isomeri.")]),
    dict(id="2-esanone", it="2-esanone", en=["2-HEXANONE", "METHYL BUTYL KETONE", "HEXAN-2-ONE"], f="C6H12O", fam="carbonilici", lv=2, keys=[
        K(100, "C6H12O+.", "M", "Lo ione molecolare."),
        K(43, "C2H3O+", "alpha", "Scissione α con perdita del butile: acetile (picco base), firma dei metilchetoni."),
        K(58, "C3H6O+.", "mclafferty", "McLafferty: trasferimento di γ-H e perdita di propene; ione enolico a elettroni dispari a m/z 58."),
        K(85, "C5H9O+", "alpha", "Scissione α con perdita del metile (meno favorita).")]),
    dict(id="4-eptanone", it="4-eptanone", en=["4-HEPTANONE", "DIPROPYL KETONE", "HEPTAN-4-ONE"], f="C7H14O", fam="carbonilici", lv=2, keys=[
        K(114, "C7H14O+.", "M", "Lo ione molecolare."),
        K(71, "C4H7O+", "alpha", "Scissione α: perdita di un propile, acilio C3H7CO+ (picco base)."),
        K(43, "C3H7+", "perdita", "L'acilio 71 perde CO."),
        K(86, "C5H10O+.", "mclafferty", "McLafferty: perdita di etene, ione a elettroni dispari.")]),
    dict(id="acetofenone", it="acetofenone", en=["ACETOPHENONE"], f="C8H8O", fam="carbonilici", lv=1, keys=[
        K(120, "C8H8O+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "Scissione α: perdita del metile, benzoile (picco base)."),
        K(77, "C6H5+", "perdita", "Benzoile − CO: fenile.")]),
    dict(id="propiofenone", it="propiofenone", en=["PROPIOPHENONE", "ETHYL PHENYL KETONE"], f="C9H10O", fam="carbonilici", lv=2, keys=[
        K(134, "C9H10O+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "Scissione α: perdita dell'etile, benzoile."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="butirrofenone", it="butirrofenone", en=["BUTYROPHENONE", "PHENYL PROPYL KETONE"], f="C10H12O", fam="carbonilici", lv=3, keys=[
        K(148, "C10H12O+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "Scissione α: perdita del propile, benzoile."),
        K(120, "C8H8O+.", "mclafferty", "McLafferty: perdita di etene; ione a elettroni dispari a massa pari, che il propiofenone non dà."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    # ---------------------------------------------------------------- acids and esters
    dict(id="acido-acetico", it="acido acetico", en=["ACETIC ACID", "ETHANOIC ACID"], f="C2H4O2", fam="acidi", lv=1, keys=[
        K(60, "C2H4O2+.", "M", "Lo ione molecolare."),
        K(45, "CHO2+", "alpha", "Perdita del metile: COOH+."),
        K(43, "C2H3O+", "alpha", "Perdita di OH: acetile.")]),
    dict(id="acido-pentanoico", it="acido pentanoico", en=["PENTANOIC ACID", "VALERIC ACID", "N-VALERIC ACID"], f="C5H10O2", fam="acidi", lv=2, keys=[
        K(60, "C2H4O2+.", "mclafferty", "McLafferty degli acidi: m/z 60 (elettroni dispari), picco base."),
        K(73, "C3H5O2+", "sigma", "Rottura del legame γ-δ: CH2CH2COOH+."),
        K(45, "CHO2+", "alpha", "COOH+.")]),
    dict(id="acido-esanoico", it="acido esanoico", en=["HEXANOIC ACID", "CAPROIC ACID", "N-CAPROIC ACID"], f="C6H12O2", fam="acidi", lv=3, keys=[
        K(60, "C2H4O2+.", "mclafferty", "McLafferty: m/z 60, la firma degli acidi carbossilici con una catena di almeno 4 C."),
        K(73, "C3H5O2+", "sigma", "Rottura γ-δ, come nell'acido pentanoico.")]),
    dict(id="acido-benzoico", it="acido benzoico", en=["BENZOIC ACID"], f="C7H6O2", fam="acidi", lv=2, keys=[
        K(122, "C7H6O2+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "Perdita di OH: benzoile."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="acetato-di-etile", it="acetato di etile", en=["ETHYL ACETATE"], f="C4H8O2", fam="acidi", lv=3, keys=[
        K(88, "C4H8O2+.", "M", "Lo ione molecolare, debole."),
        K(43, "C2H3O+", "alpha", "Acetile: picco base, firma degli acetati."),
        K(61, "C2H5O2+", "riarr", "Doppio trasferimento di idrogeno: acido acetico protonato, tipico degli acetati di alcoli con almeno due C.")]),
    dict(id="acetato-di-butile", it="acetato di butile", en=["BUTYL ACETATE", "N-BUTYL ACETATE"], f="C6H12O2", fam="acidi", lv=3, keys=[
        K(43, "C2H3O+", "alpha", "Acetile (picco base)."),
        K(56, "C4H8+.", "mclafferty", "McLafferty con la carica sull'alchene: perdita di acido acetico (M−60)."),
        K(61, "C2H5O2+", "riarr", "Acido acetico protonato (doppio trasferimento di H).")]),
    dict(id="esanoato-di-metile", it="esanoato di metile", en=["METHYL HEXANOATE", "METHYL CAPROATE"], f="C7H14O2", fam="acidi", lv=3, keys=[
        K(74, "C3H6O2+.", "mclafferty", "McLafferty degli esteri metilici: m/z 74 (picco base)."),
        K(87, "C4H7O2+", "sigma", "Rottura γ-δ: CH2CH2COOCH3+."),
        K(99, "C6H11O+", "alpha", "Perdita del metossile (M−31): acilio.")]),
    dict(id="benzoato-di-metile", it="benzoato di metile", en=["METHYL BENZOATE"], f="C8H8O2", fam="acidi", lv=2, keys=[
        K(136, "C8H8O2+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "M−31: perdita del metossile, benzoile (picco base)."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="benzoato-di-etile", it="benzoato di etile", en=["ETHYL BENZOATE"], f="C9H10O2", fam="acidi", lv=3, keys=[
        K(150, "C9H10O2+.", "M", "Lo ione molecolare."),
        K(105, "C7H5O+", "alpha", "M−45: perdita dell'etossile, benzoile."),
        K(122, "C7H6O2+.", "mclafferty", "Perdita di etene (M−28) per trasferimento di H: acido benzoico radicale catione.")]),
    dict(id="salicilato-di-metile", it="salicilato di metile", en=["METHYL SALICYLATE", "METHYL 2-HYDROXYBENZOATE"], f="C8H8O3", fam="acidi", lv=3, keys=[
        K(152, "C8H8O3+.", "M", "Lo ione molecolare."),
        K(120, "C7H4O2+.", "orto", "Effetto orto: l'OH vicino all'estero cede un H e si perde metanolo (M−32); il para non lo fa."),
        K(92, "C6H4O+.", "perdita", "120 − CO.")]),
    dict(id="dibutilftalato", it="dibutilftalato", en=["DIBUTYL PHTHALATE", "DI-N-BUTYL PHTHALATE"], f="C16H22O4", fam="acidi", lv=4, keys=[
        K(149, "C8H5O3+", "riarr", "Anidride ftalica protonata: m/z 149, firma degli ftalati (e contaminante frequente dei laboratori)."),
        K(205, "C12H13O3+", "alpha", "Perdita di un butossile (M−73).")]),
    # ---------------------------------------------------------------- amines and amides
    dict(id="butilammina", it="butilammina", en=["BUTYLAMINE", "N-BUTYLAMINE", "1-BUTANAMINE"], f="C4H11N", fam="azotati", lv=2, keys=[
        K(30, "CH4N+", "alpha", "Scissione α: CH2=NH2+ (m/z 30), firma delle ammine primarie con il gruppo su un CH2 (picco base)."),
        K(73, "C4H11N+.", "M", "M dispari: un azoto (regola dell'azoto).")]),
    dict(id="sec-butilammina", it="sec-butilammina", en=["SEC-BUTYLAMINE", "2-BUTANAMINE", "2-AMINOBUTANE"], f="C4H11N", fam="azotati", lv=3, keys=[
        K(44, "C2H6N+", "alpha", "Scissione α con perdita dell'etile (il più grande): picco base."),
        K(58, "C3H8N+", "alpha", "Scissione α con perdita del metile.")]),
    dict(id="terz-butilammina", it="terz-butilammina", en=["TERT-BUTYLAMINE", "T-BUTYLAMINE", "2-METHYL-2-PROPANAMINE"], f="C4H11N", fam="azotati", lv=3, keys=[
        K(58, "C3H8N+", "alpha", "Scissione α: perdita di un metile, ione iminio a massa pari (un N, elettroni pari).")]),
    dict(id="isobutilammina", it="isobutilammina", en=["ISOBUTYLAMINE", "2-METHYLPROPYLAMINE", "2-METHYL-1-PROPANAMINE"], f="C4H11N", fam="azotati", lv=3, keys=[
        K(30, "CH4N+", "alpha", "CH2=NH2+: come la butilammina; le due si distinguono solo dai dettagli della parte alta.")]),
    dict(id="dietilammina", it="dietilammina", en=["DIETHYLAMINE"], f="C4H11N", fam="azotati", lv=2, keys=[
        K(58, "C3H8N+", "alpha", "Scissione α: perdita di un metile (picco base)."),
        K(73, "C4H11N+.", "M", "M dispari, un azoto."),
        K(30, "CH4N+", "riarr", "Lo ione 58 perde etene: CH2=NH2+.")]),
    dict(id="trietilammina", it="trietilammina", en=["TRIETHYLAMINE"], f="C6H15N", fam="azotati", lv=1, keys=[
        K(86, "C5H12N+", "alpha", "Scissione α: perdita di un metile, ione iminio (picco base). Le ammine danno la scissione α più forte di tutte."),
        K(101, "C6H15N+.", "M", "M dispari: un azoto."),
        K(58, "C3H8N+", "riarr", "86 − etene.")]),
    dict(id="anilina", it="anilina", en=["ANILINE", "BENZENAMINE", "AMINOBENZENE"], f="C6H7N", fam="azotati", lv=2, keys=[
        K(93, "C6H7N+.", "M", "M dispari (un N), picco base."),
        K(66, "C5H6+.", "perdita", "Perdita di HCN (M−27): tipica delle ammine aromatiche e degli eterocicli azotati."),
        K(65, "C5H5+", "perdita", "66 − H.")]),
    dict(id="piridina", it="piridina", en=["PYRIDINE"], f="C5H5N", fam="azotati", lv=2, keys=[
        K(79, "C5H5N+.", "M", "M dispari, picco base: eterociclo aromatico."),
        K(52, "C4H4+.", "perdita", "Perdita di HCN.")]),
    dict(id="chinolina", it="chinolina", en=["QUINOLINE"], f="C9H7N", fam="azotati", lv=4, keys=[
        K(129, "C9H7N+.", "M", "M dispari e intenso."),
        K(102, "C8H6+.", "perdita", "Perdita di HCN.")]),
    dict(id="dimetilacetammide", it="N,N-dimetilacetammide", en=["N,N-DIMETHYLACETAMIDE", "DIMETHYLACETAMIDE"], f="C4H9NO", fam="azotati", lv=3, keys=[
        K(87, "C4H9NO+.", "M", "M dispari: un azoto."),
        K(44, "C2H6N+", "i", "(CH3)2N+ e isomeri: la carica resta sulla parte azotata."),
        K(43, "C2H3O+", "alpha", "Acetile.")]),
    dict(id="benzammide", it="benzammide", en=["BENZAMIDE"], f="C7H7NO", fam="azotati", lv=2, keys=[
        K(121, "C7H7NO+.", "M", "M dispari (un azoto) e intenso: ammide aromatica."),
        K(105, "C7H5O+", "alpha", "Scissione α: perdita di NH2•, benzoile (picco base)."),
        K(77, "C6H5+", "perdita", "Benzoile − CO: fenile.")]),
        dict(id="nitrobenzene", it="nitrobenzene", en=["NITROBENZENE"], f="C6H5NO2", fam="azotati", lv=2, keys=[
        K(123, "C6H5NO2+.", "M", "M dispari: un azoto."),
        K(77, "C6H5+", "perdita", "M−46: perdita di NO2, fenile (picco base)."),
        K(93, "C6H5O+", "perdita", "M−30: perdita di NO dopo isomerizzazione nitro → nitrito."),
        K(51, "C4H3+", "perdita", "Il fenile perde acetilene.")]),
    dict(id="nitrometano", it="nitrometano", en=["NITROMETHANE"], f="CH3NO2", fam="azotati", lv=3, keys=[
        K(61, "CH3NO2+.", "M", "Lo ione molecolare, dispari."),
        K(30, "NO+", "i", "NO+: firma dei nitrocomposti."),
        K(46, "NO2+", "i", "NO2+.")]),
    dict(id="butanenitrile", it="butanenitrile", en=["BUTANENITRILE", "BUTYRONITRILE", "PROPYL CYANIDE"], f="C4H7N", fam="azotati", lv=3, keys=[
        K(41, "C2H3N+.", "mclafferty", "McLafferty dei nitrili: CH2=C=NH+• a m/z 41 (picco base), perdita di etene.")]),
    # ---------------------------------------------------------------- sulfur
    dict(id="2-etiltiofene", it="2-etiltiofene", en=["2-ETHYLTHIOPHENE"], f="C6H8S", fam="zolfo", lv=2, keys=[
        K(112, "C6H8S+.", "M", "M intenso: eterociclo aromatico con lo zolfo."),
        K(114, "C6H8S+.", "isotopo", "M+2 di circa il 4,5%: un atomo di zolfo (34S).", 2),
        K(97, "C5H5S+", "alpha", "Scissione «benzilica» del gruppo etile: perdita di un metile e catione tienilmetile/tiopirilio (picco base), come il tropilio degli alchilbenzeni.")]),
        dict(id="dimetildisolfuro", it="dimetildisolfuro", en=["DIMETHYL DISULFIDE", "METHYL DISULFIDE"], f="C2H6S2", fam="zolfo", lv=3, keys=[
        K(94, "C2H6S2+.", "M", "M picco base."),
        K(96, "C2H6S2+.", "isotopo", "M+2 di circa il 9%: due atomi di zolfo.", 2),
        K(79, "CH3S2+", "perdita", "M−15."),
        K(45, "CHS+", "perdita", "CHS+.")]),
    dict(id="dietilsolfuro", it="dietilsolfuro", en=["DIETHYL SULFIDE", "ETHYL SULFIDE"], f="C4H10S", fam="zolfo", lv=3, keys=[
        K(90, "C4H10S+.", "M", "M intenso: lo zolfo si ionizza facilmente (IE più bassa dell'ossigeno)."),
        K(75, "C3H7S+", "alpha", "Scissione α: perdita di un metile, ione solfonio.")]),
    dict(id="1-butantiolo", it="1-butantiolo", en=["1-BUTANETHIOL", "BUTYL MERCAPTAN", "BUTANETHIOL", "N-BUTYL MERCAPTAN"], f="C4H10S", fam="zolfo", lv=3, keys=[
        K(90, "C4H10S+.", "M", "Lo ione molecolare, con il 34S a M+2."),
        K(56, "C4H8+.", "perdita", "Perdita di H2S (M−34)."),
        K(47, "CH3S+", "alpha", "CH2=SH+: l'analogo di CH2=OH+ degli alcoli.")]),
    # ---------------------------------------------------------------- halogenated
    dict(id="clorobenzene", it="clorobenzene", en=["CHLOROBENZENE"], f="C6H5Cl", fam="alogenati", lv=1, keys=[
        K(112, "C6H5Cl+.", "M", "M picco base."),
        K(114, "C6H5Cl+.", "isotopo", "M+2 al 32%: un atomo di cloro.", 2),
        K(77, "C6H5+", "i", "Perdita del cloro: fenile.")]),
    dict(id="bromobenzene", it="bromobenzene", en=["BROMOBENZENE"], f="C6H5Br", fam="alogenati", lv=1, keys=[
        K(156, "C6H5Br+.", "M", "Lo ione molecolare."),
        K(158, "C6H5Br+.", "isotopo", "M+2 quasi uguale a M: un atomo di bromo.", 2),
        K(77, "C6H5+", "i", "Perdita del bromo: fenile.")]),
    dict(id="1,2-diclorobenzene", it="1,2-diclorobenzene", en=["1,2-DICHLOROBENZENE", "O-DICHLOROBENZENE"], f="C6H4Cl2", fam="alogenati", lv=2, keys=[
        K(146, "C6H4Cl2+.", "M", "M picco base."),
        K(148, "C6H4Cl2+.", "isotopo", "100 : 64 : 10 a M, M+2, M+4: due cloro.", 2),
        K(111, "C6H4Cl+", "i", "Perdita di un cloro."),
        K(75, "C6H3+", "perdita", "111 − HCl.")]),
    dict(id="1,3-diclorobenzene", it="1,3-diclorobenzene", en=["1,3-DICHLOROBENZENE", "M-DICHLOROBENZENE"], f="C6H4Cl2", fam="alogenati", lv=3, keys=[
        K(146, "C6H4Cl2+.", "M", "Stesso spettro, quasi, dell'1,2 e dell'1,4: l'EI non distingue bene gli isomeri di posizione."),
        K(148, "C6H4Cl2+.", "isotopo", "Due cloro.", 2),
        K(111, "C6H4Cl+", "i", "Perdita di un cloro.")]),
    dict(id="1,4-diclorobenzene", it="1,4-diclorobenzene", en=["1,4-DICHLOROBENZENE", "P-DICHLOROBENZENE"], f="C6H4Cl2", fam="alogenati", lv=3, keys=[
        K(146, "C6H4Cl2+.", "M", "Isomero dei precedenti: serve la cromatografia (indice di ritenzione) per distinguerli."),
        K(148, "C6H4Cl2+.", "isotopo", "Due cloro.", 2),
        K(111, "C6H4Cl+", "i", "Perdita di un cloro.")]),
    dict(id="1-clorobutano", it="1-clorobutano", en=["1-CHLOROBUTANE", "BUTYL CHLORIDE", "N-BUTYL CHLORIDE"], f="C4H9Cl", fam="alogenati", lv=3, keys=[
        K(56, "C4H8+.", "perdita", "Perdita di HCl (M−36): ione a elettroni dispari (picco base)."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
    dict(id="2-bromobutano", it="2-bromobutano", en=["SEC-BUTYL BROMIDE", "2-BROMOBUTANE"], f="C4H9Br", fam="alogenati", lv=3, keys=[
        K(57, "C4H9+", "i", "Scissione induttiva: il bromo se ne va con la coppia di elettroni e resta il catione sec-butile (picco base)."),
        K(41, "C3H5+", "serie", "Catione allile.")]),
        dict(id="diclorometano", it="diclorometano", en=["DICHLOROMETHANE", "METHYLENE CHLORIDE"], f="CH2Cl2", fam="alogenati", lv=2, keys=[
        K(84, "CH2Cl2+.", "M", "Lo ione molecolare, con il gruppo 84/86/88 di due cloro."),
        K(49, "CH2Cl+", "i", "Perdita di un cloro (picco base)."),
        K(51, "CH2Cl+", "isotopo", "Il 49 con il 37Cl: 3 : 1.", 2)]),
    dict(id="cloroformio", it="cloroformio", en=["CHLOROFORM", "TRICHLOROMETHANE"], f="CHCl3", fam="alogenati", lv=2, keys=[
        K(83, "CHCl2+", "i", "Perdita di un cloro (picco base); M (118) è debolissimo."),
        K(85, "CHCl2+", "isotopo", "Gruppo 83/85/87 = due cloro (100 : 64 : 10).", 2),
        K(47, "CCl+", "perdita", "CCl+.")]),
    dict(id="tetracloruro-di-carbonio", it="tetracloruro di carbonio", en=["CARBON TETRACHLORIDE", "TETRACHLOROMETHANE"], f="CCl4", fam="alogenati", lv=3, keys=[
        K(117, "CCl3+", "i", "CCl3+: picco base. M (152) non si vede: il gruppo più alto non è lo ione molecolare!"),
        K(119, "CCl3+", "isotopo", "Gruppo 117/119/121 = tre cloro (100 : 96 : 31).", 2),
        K(82, "CCl2+.", "perdita", "CCl2+•.")]),
    dict(id="tricloroetilene", it="tricloroetilene", en=["TRICHLOROETHYLENE", "TRICHLOROETHENE"], f="C2HCl3", fam="alogenati", lv=2, keys=[
        K(130, "C2HCl3+.", "M", "M picco base, con tre cloro: 130/132/134."),
        K(132, "C2HCl3+.", "isotopo", "Tre cloro: 100 : 96 : 31.", 2),
        K(95, "C2HCl2+", "i", "Perdita di un cloro.")]),
    dict(id="tetracloroetilene", it="tetracloroetilene", en=["TETRACHLOROETHYLENE", "TETRACHLOROETHENE", "PERCHLOROETHYLENE"], f="C2Cl4", fam="alogenati", lv=3, keys=[
        K(164, "C2Cl4+.", "M", "Lo ione molecolare con quattro 35Cl: non è il picco più alto del gruppo."),
        K(166, "C2Cl4+.", "isotopo", "Con quattro cloro il picco più alto è M+2 (78 : 100 : 48 : 10).", 2),
        K(129, "C2Cl3+", "i", "Perdita di un cloro."),
        K(94, "C2Cl2+.", "perdita", "Perdita di Cl2.")]),
    # ---------------------------------------------------------------- phenols
    dict(id="fenolo", it="fenolo", en=["PHENOL"], f="C6H6O", fam="fenoli", lv=2, keys=[
        K(94, "C6H6O+.", "M", "M picco base."),
        K(66, "C5H6+.", "perdita", "Perdita di CO (M−28): tipica di fenoli e chinoni."),
        K(65, "C5H5+", "perdita", "Perdita di CHO (M−29).")]),
    dict(id="2,4-diclorofenolo", it="2,4-diclorofenolo", en=["2,4-DICHLOROPHENOL"], f="C6H4Cl2O", fam="fenoli", lv=2, keys=[
        K(162, "C6H4Cl2O+.", "M", "M picco base, gruppo di due cloro."),
        K(164, "C6H4Cl2O+.", "isotopo", "100 : 64 : 10: due cloro.", 2),
        K(98, "C5H3Cl+.", "perdita", "Perdita di HCl e di CO.")]),
    dict(id="pentaclorofenolo", it="pentaclorofenolo", en=["PENTACHLOROPHENOL"], f="C6HCl5O", fam="fenoli", lv=4, keys=[
        K(264, "C6HCl5O+.", "M", "Lo ione con cinque 35Cl: il gruppo dello ione molecolare ha il massimo a M+2."),
        K(266, "C6HCl5O+.", "isotopo", "Cinque cloro: 61 : 100 : 64 : 20 : 3 circa.", 2)]),
    dict(id="furano", it="furano", en=["FURAN"], f="C4H4O", fam="fenoli", lv=2, keys=[
        K(68, "C4H4O+.", "M", "M intenso: eterociclo aromatico."),
        K(39, "C3H3+", "perdita", "Perdita di CHO (M−29): ciclopropenile.")]),
    # ---------------------------------------------------------------- environmental and bridge
    dict(id="caffeina", it="caffeina", en=["CAFFEINE"], f="C8H10N4O2", fam="ambientali", lv=4, keys=[
        K(194, "C8H10N4O2+.", "M", "M pari con quattro azoti (numero pari): picco base. Tracciante delle acque reflue.")]),
    dict(id="beta-hch", it="β-esaclorocicloesano (β-HCH, isomero del lindano)", en=["BETA-1,2,3,4,5,6-HEXACHLOROCYCLOHEXANE", "BETA-HCH", "BETA-BHC"], f="C6H6Cl6", fam="ambientali", lv=4, keys=[
        K(181, "C6H4Cl3+", "perdita", "Gruppo 181/183/185 (tre cloro): lo ione molecolare (288) quasi non si vede, la molecola perde HCl e Cl."),
        K(183, "C6H4Cl3+", "isotopo", "Tre cloro.", 2)]),
        dict(id="atrazina", it="atrazina", en=["ATRAZINE"], f="C8H14ClN5", fam="ambientali", lv=4, keys=[
        K(215, "C8H14ClN5+.", "M", "M dispari (cinque azoti) con il cloro a M+2."),
        K(200, "C7H11ClN5+", "alpha", "Scissione α nel gruppo isopropilamminico: perdita di un metile (picco base)."),
        K(217, "C8H14ClN5+.", "isotopo", "Un cloro: M+2 al 32%.", 2),
        K(173, "C5H8ClN5+.", "riarr", "Perdita di propene (M−42) dal gruppo isopropile.")]),
    # ---------------------------------------------------------------- path of simple compounds (see PATH): new entries
    dict(id="butano", it="butano", en=["BUTANE", "N-BUTANE"], f="C4H10", fam="alcani", lv=1, play=False, keys=[
        K(58, "C4H10+.", "M", "Lo ione molecolare: massa pari, nessun azoto; debole ma ben visibile."),
        K(43, "C3H7+", "sigma", "Perdita di un metile (M−15): picco base."),
        K(29, "C2H5+", "sigma", "Rottura al centro della catena: due frammenti da due carboni."),
        K(41, "C3H5+", "serie", "Il catione propile perde H2: serie di ioni a intervalli di 14 e di 2."),
        K(28, "C2H4+.", "riarr", "Ione a elettroni dispari a massa pari: due rotture, perdita di etano (M−30).")]),
    dict(id="pentano", it="pentano", en=["PENTANE", "N-PENTANE"], f="C5H12", fam="alcani", lv=1, keys=[
        K(72, "C5H12+.", "M", "Lo ione molecolare, debole (circa un settimo del picco base)."),
        K(57, "C4H9+", "sigma", "M−15: perdita di un metile."),
        K(43, "C3H7+", "sigma", "Perdita di un etile (M−29): picco base."),
        K(41, "C3H5+", "serie", "Catione allile, 43 − H2.")]),
    dict(id="pentadecano", it="pentadecano", en=["PENTADECANE", "N-PENTADECANE"], f="C15H32", fam="alcani", lv=2, keys=[
        K(212, "C15H32+.", "M", "M debole ma presente: il gruppo più alto della «staccionata»."),
        K(57, "C4H9+", "serie", "Picco base della serie alchilica CnH2n+1+."),
        K(43, "C3H7+", "serie", "Secondo massimo della serie, intervalli di 14 (CH2)."),
        K(71, "C5H11+", "serie", "La serie scende con regolarità: nessuno ione preferito."),
        K(85, "C6H13+", "serie", "Ancora il passo di 14: catena non ramificata.")]),
    dict(id="farnesano", it="2,6,10-trimetildodecano (farnesano)", en=["2,6,10-TRIMETHYLDODECANE", "FARNESANE"], f="C15H32", fam="alcani", lv=4, keys=[
        K(212, "C15H32+.", "M", "M debolissimo: le ramificazioni indeboliscono lo ione molecolare."),
        K(57, "C4H9+", "sigma", "Picco base, come negli alcani lineari: da solo non distingue."),
        K(197, "C14H29+", "sigma", "M−15: perdita di un metile da un punto ramificato (nel lineare M−15 è quasi assente)."),
        K(169, "C12H25+", "sigma", "M−43: perdita di un propile/isopropile alla ramificazione."),
        K(112, "C8H16+.", "riarr", "Ione a elettroni dispari (alchene): lungo la catena ramificata si perdono frammenti interi con due rotture.")]),
    dict(id="bromometano", it="bromometano", en=["BROMOMETHANE", "METHYL BROMIDE"], f="CH3Br", fam="alogenati", lv=1, play=False, keys=[
        K(94, "CH3Br+.", "M", "Lo ione molecolare è il picco base; accanto ha un gemello due unità più pesante."),
        K(96, "CH3Br+.", "isotopo", "M+2 quasi uguale a M: il bromo ha due isotopi (79 e 81) in rapporto circa 1 : 1.", 2),
        K(93, "CH2Br+", "perdita", "M−1: perdita di un H; anche questo picco ha il suo gemello (93/95)."),
        K(79, "Br+", "i", "Scissione induttiva: la carica resta sull'alogeno (coppia 79/81).")]),
    dict(id="iodometano", it="iodometano", en=["IODOMETHANE", "METHYL IODIDE"], f="CH3I", fam="alogenati", lv=1, play=False, keys=[
        K(142, "CH3I+.", "M", "Lo ione molecolare è il picco base; nessun M+2: lo iodio ha un solo isotopo stabile."),
        K(127, "I+", "i", "Scissione induttiva: la carica resta sullo iodio; perdita di un metile (M−15)."),
        K(141, "CH2I+", "perdita", "M−1: perdita di un H.")]),
    dict(id="bromoetano", it="bromoetano", en=["BROMOETHANE", "ETHYL BROMIDE"], f="C2H5Br", fam="alogenati", lv=1, keys=[
        K(108, "C2H5Br+.", "M", "Lo ione molecolare, intenso."),
        K(110, "C2H5Br+.", "isotopo", "M+2 quasi uguale a M: un bromo.", 2),
        K(29, "C2H5+", "i", "Scissione induttiva: il bromo se ne va con la coppia di elettroni, resta il catione etile (picco base)."),
        K(27, "C2H3+", "perdita", "Il catione etile perde H2 (29 − 2).")]),
    dict(id="1-bromopropano", it="1-bromopropano", en=["1-BROMOPROPANE", "PROPYL BROMIDE", "N-PROPYL BROMIDE"], f="C3H7Br", fam="alogenati", lv=1, keys=[
        K(122, "C3H7Br+.", "M", "Lo ione molecolare, con la coppia M/M+2 del bromo."),
        K(124, "C3H7Br+.", "isotopo", "Coppia M/M+2 ≈ 1 : 1: un bromo.", 2),
        K(43, "C3H7+", "i", "Scissione induttiva: perdita di Br• (M−79), catione propile (picco base)."),
        K(41, "C3H5+", "serie", "Catione allile: 43 − H2.")]),
    dict(id="2-metil-2-butanolo", it="2-metil-2-butanolo", en=["2-METHYL-2-BUTANOL", "TERT-AMYL ALCOHOL", "TERT-PENTYL ALCOHOL"], f="C5H12O", fam="alcoli", lv=2, keys=[
        K(88, "C5H12O+.", "M", "Negli alcoli terziari lo ione molecolare è debolissimo o assente."),
        K(59, "C3H7O+", "alpha", "Scissione α con perdita di un etile, il radicale più grande (picco base)."),
        K(73, "C4H9O+", "alpha", "Scissione α con perdita di un metile (M−15)."),
        K(55, "C4H7+", "perdita", "Perdita di acqua dallo ione 73 (73 − 18)."),
        K(31, "CH3O+", "riarr", "Ione ossonio più piccolo, da un riarrangiamento dello ione 59.")]),
    dict(id="3-metil-3-pentanolo", it="3-metil-3-pentanolo", en=["3-METHYL-3-PENTANOL"], f="C6H14O", fam="alcoli", lv=3, keys=[
        K(102, "C6H14O+.", "M", "Lo ione molecolare di un alcol terziario: debolissimo o assente."),
        K(73, "C4H9O+", "alpha", "Scissione α con perdita di un etile (M−29): picco base."),
        K(87, "C5H11O+", "alpha", "Scissione α con perdita di un metile (M−15)."),
        K(55, "C4H7+", "perdita", "Perdita di acqua dallo ione 73.")]),
    dict(id="metil-butil-etere", it="metil butil etere", en=["BUTYL METHYL ETHER", "METHYL BUTYL ETHER", "1-METHOXYBUTANE", "METHYL N-BUTYL ETHER"], f="C5H12O", fam="eteri", lv=2, keys=[
        K(88, "C5H12O+.", "M", "Lo ione molecolare, debole (circa il 3% del picco base)."),
        K(45, "C2H5O+", "alpha", "Scissione α dal lato del butile: perdita di un propile (picco base)."),
        K(56, "C4H8+.", "perdita", "M−32: perdita di metanolo, ione a elettroni dispari.")]),
    dict(id="2-butanone", it="2-butanone", en=["2-BUTANONE", "METHYL ETHYL KETONE", "BUTAN-2-ONE"], f="C4H8O", fam="carbonilici", lv=2, keys=[
        K(72, "C4H8O+.", "M", "Lo ione molecolare. Non c'è il picco di McLafferty: nessun idrogeno in γ."),
        K(43, "C2H3O+", "alpha", "Scissione α con perdita dell'etile, il radicale più grande: acetile (picco base)."),
        K(57, "C3H5O+", "alpha", "Scissione α dall'altra parte: perdita del metile (M−15)."),
        K(29, "C2H5+", "i", "Scissione induttiva: la carica va sul gruppo etile.")]),
    dict(id="3-metil-2-pentanone", it="3-metil-2-pentanone", en=["3-METHYL-2-PENTANONE", "3-METHYLPENTAN-2-ONE"], f="C6H12O", fam="carbonilici", lv=3, keys=[
        K(100, "C6H12O+.", "M", "Lo ione molecolare."),
        K(43, "C2H3O+", "alpha", "Scissione α con perdita del radicale più grande: acetile (picco base)."),
        K(72, "C4H8O+.", "mclafferty", "McLafferty con perdita di etene: ione enolico a elettroni dispari."),
        K(57, "C4H9+", "i", "Scissione induttiva: la carica sul catione sec-butile.")]),
    dict(id="4-metil-2-pentanone", it="4-metil-2-pentanone", en=["4-METHYL-2-PENTANONE", "METHYL ISOBUTYL KETONE", "4-METHYLPENTAN-2-ONE"], f="C6H12O", fam="carbonilici", lv=3, keys=[
        K(100, "C6H12O+.", "M", "Lo ione molecolare."),
        K(43, "C2H3O+", "alpha", "Scissione α con perdita dell'isobutile: acetile (picco base)."),
        K(58, "C3H6O+.", "mclafferty", "McLafferty con perdita di propene: ione enolico a elettroni dispari."),
        K(85, "C5H9O+", "alpha", "Scissione α con perdita del metile (M−15)."),
        K(57, "C4H9+", "i", "Scissione induttiva: catione isobutile/terz-butile.")]),
    dict(id="butanoato-di-etile", it="butanoato di etile", en=["ETHYL BUTANOATE", "ETHYL BUTYRATE"], f="C6H12O2", fam="acidi", lv=3, keys=[
        K(116, "C6H12O2+.", "M", "Lo ione molecolare, debole."),
        K(71, "C4H7O+", "alpha", "M−45: perdita dell'etossile, catione acilio (picco base)."),
        K(88, "C4H8O2+.", "mclafferty", "McLafferty sul lato dell'acido, con perdita di etene."),
        K(43, "C3H7+", "i", "Catione propile."),
        K(60, "C2H4O2+.", "riarr", "Dallo ione 88 per una seconda perdita di etene (88 − 28).")]),
    dict(id="alfa-ionone", it="α-ionone", en=["ALPHA-IONONE", "A-IONONE", "(E)-ALPHA-IONONE", "TRANS-ALPHA-IONONE"], f="C13H20O", fam="carbonilici", lv=4, keys=[
        K(192, "C13H20O+.", "M", "Lo ione molecolare, RDB 4 (anello, due doppi legami, C=O)."),
        K(136, "C9H12O+.", "rda", "Retro-Diels-Alder dell'anello cicloesenico: perdita di un'olefina da quattro carboni (M−56)."),
        K(121, "C8H9O+", "perdita", "136 − CH3."),
        K(93, "C7H9+", "perdita", "121 − CO."),
        K(43, "C2H3O+", "alpha", "Acetile da scissione α del metilchetone.")]),
    dict(id="beta-ionone", it="β-ionone", en=["BETA-IONONE", "B-IONONE", "(E)-BETA-IONONE", "TRANS-BETA-IONONE"], f="C13H20O", fam="carbonilici", lv=4, keys=[
        K(192, "C13H20O+.", "M", "Lo ione molecolare, isomero dell'α-ionone."),
        K(177, "C12H17O+", "perdita", "M−15: perdita di un metile, picco base (nell'isomero con il doppio legame fuori anello il picco base è un altro)."),
        K(135, "C10H15+", "perdita", "177 − chetene (C2H2O)."),
        K(43, "C2H3O+", "alpha", "Acetile da scissione α del metilchetone.")]),
    dict(id="acetammide", it="acetammide", en=["ACETAMIDE"], f="C2H5NO", fam="azotati", lv=3, play=False, keys=[
        K(59, "C2H5NO+.", "M", "M dispari: un azoto. Picco base."),
        K(44, "CH2NO+", "alpha", "M−15: scissione α con perdita del metile; resta O=C=NH2+."),
        K(43, "C2H3O+", "alpha", "M−16: perdita di NH2•, acetile."),
        K(41, "C2H3N+.", "perdita", "M−18: perdita di acqua, ione nitrilico a elettroni dispari.")]),
    dict(id="n-metilacetammide", it="N-metilacetammide", en=["N-METHYLACETAMIDE"], f="C3H7NO", fam="azotati", lv=3, keys=[
        K(73, "C3H7NO+.", "M", "M dispari: un azoto."),
        K(43, "C2H3O+", "alpha", "Scissione α con perdita del radicale metilamminico: acetile (picco base)."),
        K(58, "C2H4NO+", "alpha", "M−15: perdita di un metile."),
        K(30, "CH4N+", "riarr", "CH2=NH2+: tipica delle ammine e delle ammidi con un gruppo CH3 sull'azoto.")]),
    dict(id="benzonitrile", it="benzonitrile", en=["BENZONITRILE"], f="C7H5N", fam="azotati", lv=2, play=False, keys=[
        K(103, "C7H5N+.", "M", "M dispari (un azoto), picco base: nitrile aromatico (RDB 6)."),
        K(76, "C6H4+.", "perdita", "M−27: perdita di HCN, tipica di nitrili e ammine aromatiche."),
        K(50, "C4H2+.", "perdita", "76 − acetilene."),
        K(51, "C4H3+", "serie", "Serie aromatica 50-52, 63-65, 74-78.")]),
    dict(id="4-nitrotoluene", it="4-nitrotoluene", en=["4-NITROTOLUENE", "P-NITROTOLUENE"], f="C7H7NO2", fam="azotati", lv=3, keys=[
        K(137, "C7H7NO2+.", "M", "M dispari: un azoto; intenso per l'anello."),
        K(91, "C7H7+", "tropilio", "M−46: perdita di NO2 e tropilio (picco base)."),
        K(107, "C7H7O+", "perdita", "M−30: perdita di NO dopo isomerizzazione nitro → nitrito."),
        K(65, "C5H5+", "perdita", "Il tropilio perde acetilene (91 − 26).")]),
    dict(id="diacetone-alcol", it="4-idrossi-4-metil-2-pentanone", en=["DIACETONE ALCOHOL", "4-HYDROXY-4-METHYL-2-PENTANONE"], f="C6H12O2", fam="carbonilici", lv=4, keys=[
        K(43, "C2H3O+", "alpha", "Acetile da scissione α del chetone (picco base)."),
        K(59, "C3H7O+", "alpha", "Scissione α sull'alcol terziario con perdita del resto chetonico: (CH3)2C=OH+."),
        K(101, "C5H9O2+", "alpha", "M−15: perdita di un metile dall'alcol terziario."),
        K(58, "C3H6O+.", "riarr", "Ione a elettroni dispari: una molecola di acetone ionizzata, da un riarrangiamento di tipo retro-aldolico."),
        K(98, "C6H10O+.", "perdita", "M−18: perdita di acqua.")]),
    dict(id="levulinato-di-metile", it="levulinato di metile", en=["METHYL LEVULINATE"], f="C6H10O3", fam="acidi", lv=4, keys=[
        K(43, "C2H3O+", "alpha", "Acetile da scissione α del metilchetone (picco base)."),
        K(99, "C5H7O2+", "alpha", "M−31: perdita del metossile dall'estere."),
        K(115, "C5H7O3+", "alpha", "M−15: perdita del metile del chetone."),
        K(59, "C2H3O2+", "alpha", "Gruppo metossicarbonile COOCH3+.")]),
    dict(id="acido-levulinico", it="acido levulinico", en=["LEVULINIC ACID", "4-OXOPENTANOIC ACID"], f="C5H8O3", fam="acidi", lv=4, keys=[
        K(116, "C5H8O3+.", "M", "Lo ione molecolare, debolissimo."),
        K(43, "C2H3O+", "alpha", "Acetile da scissione α del metilchetone (picco base)."),
        K(45, "CHO2+", "alpha", "COOH+ dal gruppo carbossilico."),
        K(56, "C3H4O+.", "perdita", "M−60: ione a elettroni dispari, perdita di una molecola neutra con due rotture."),
        K(99, "C5H7O2+", "perdita", "M−17: perdita di OH.")]),
    dict(id="vanillina", it="vanillina", en=["VANILLIN"], f="C8H8O3", fam="fenoli", lv=4, keys=[
        K(152, "C8H8O3+.", "M", "M intenso: anello aromatico con tre sostituenti."),
        K(151, "C8H7O3+", "alpha", "M−1: perdita dell'H aldeidico."),
        K(123, "C7H7O2+", "perdita", "M−29: perdita di CHO."),
        K(109, "C6H5O2+", "perdita", "M−43: perdita di CH3 e CO in successione."),
        K(81, "C5H5O+", "perdita", "109 − CO.")]),
    dict(id="4-metossiacetofenone", it="4-metossiacetofenone", en=["4-METHOXYACETOPHENONE", "4'-METHOXYACETOPHENONE", "P-METHOXYACETOPHENONE"], f="C9H10O2", fam="carbonilici", lv=4, keys=[
        K(150, "C9H10O2+.", "M", "Lo ione molecolare."),
        K(135, "C8H7O2+", "alpha", "Scissione α: perdita del metile dell'acetile, catione acilio sostituito (picco base)."),
        K(107, "C7H7O+", "perdita", "135 − CO."),
        K(92, "C6H4O+.", "perdita", "107 − CH3•: ione a elettroni dispari."),
        K(77, "C6H5+", "perdita", "Fenile.")]),
    dict(id="4-cloroanilina", it="4-cloroanilina", en=["4-CHLOROANILINE", "P-CHLOROANILINE"], f="C6H6ClN", fam="azotati", lv=4, keys=[
        K(127, "C6H6ClN+.", "M", "M dispari: un azoto; picco base."),
        K(129, "C6H6ClN+.", "isotopo", "M+2 circa un terzo di M: un cloro.", 2),
        K(92, "C6H6N+", "i", "M−35: perdita di Cl•."),
        K(100, "C5H5Cl+.", "perdita", "M−27: perdita di HCN dall'anello."),
        K(65, "C5H5+", "perdita", "92 − HCN.")]),
    dict(id="cicloesanolo", it="cicloesanolo", en=["CYCLOHEXANOL"], f="C6H12O", fam="alcoli", lv=3, keys=[
        K(100, "C6H12O+.", "M", "Lo ione molecolare è debole: gli alcoli ciclici si aprono con facilità."),
        K(57, "C3H5O+", "alpha", "Scissione α con apertura dell'anello e seconda rottura: lo ione tipico degli alcoli ciclici (picco base)."),
        K(82, "C6H10+.", "perdita", "M−18: perdita di acqua, ione a elettroni dispari."),
        K(67, "C5H7+", "perdita", "82 − CH3.")]),
    dict(id="acido-butanoico", it="acido butanoico", en=["BUTANOIC ACID", "BUTYRIC ACID", "N-BUTYRIC ACID"], f="C4H8O2", fam="acidi", lv=2, keys=[
        K(88, "C4H8O2+.", "M", "Lo ione molecolare, debole."),
        K(60, "C2H4O2+.", "mclafferty", "McLafferty degli acidi: m/z 60 (elettroni dispari), picco base."),
        K(73, "C3H5O2+", "sigma", "M−15: rottura dell'estremità della catena."),
        K(45, "CHO2+", "alpha", "COOH+.")]),
    dict(id="o-xilene", it="o-xilene", en=["ORTHO XYLENE", "ORTHO-XYLENE", "O-XYLENE", "1,2-DIMETHYLBENZENE"], f="C8H10", fam="aromatici", lv=2, keys=[
        K(106, "C8H10+.", "M", "Lo ione molecolare, intenso."),
        K(91, "C7H7+", "tropilio", "M−15: perdita di un metile e tropilio (picco base)."),
        K(105, "C8H9+", "perdita", "M−1: perdita di un H."),
        K(65, "C5H5+", "perdita", "Il tropilio perde acetilene (91 − 26).")]),
    # ---------------------------------------------------------------- reference spectra only (isomers of the problems)
        dict(id="2-pentanone", it="2-pentanone", en=["2-PENTANONE", "METHYL PROPYL KETONE"], f="C5H10O", fam="carbonilici", lv=0, play=False, keys=[]),
    dict(id="3-esanone", it="3-esanone", en=["3-HEXANONE", "ETHYL PROPYL KETONE"], f="C6H12O", fam="carbonilici", lv=0, play=False, keys=[]),
    dict(id="esanale", it="esanale", en=["HEXANAL", "CAPROALDEHYDE", "HEXALDEHYDE"], f="C6H12O", fam="carbonilici", lv=0, play=False, keys=[]),
    dict(id="p-xilene", it="p-xilene", en=["PARA XYLENE", "PARA-XYLENE", "P-XYLENE", "1,4-DIMETHYLBENZENE"], f="C8H10", fam="aromatici", lv=0, play=False, keys=[]),
    dict(id="propanoato-di-metile", it="propanoato di metile", en=["METHYL PROPANOATE", "METHYL PROPIONATE"], f="C4H8O2", fam="acidi", lv=0, play=False, keys=[]),
    dict(id="2-metil-1-propanolo", it="2-metil-1-propanolo", en=["2-METHYL-1-PROPANOL", "ISOBUTYL ALCOHOL", "ISOBUTANOL"], f="C4H10O", fam="alcoli", lv=0, play=False, keys=[]),
    dict(id="metil-propil-etere", it="metil propil etere", en=["METHYL PROPYL ETHER", "1-METHOXYPROPANE"], f="C4H10O", fam="eteri", lv=0, play=False, keys=[]),
]
LIST += LIST_EXTRA


def skills_of(item, peaks) -> list:
    """Skill tags derived from the (test-validated) keys and the formula: they are the vocabulary of the hints of the Pratica."""
    f = canon(item["f"])
    tags = {"M"}
    tags |= {SKILL_OF_TYPE[k["type"]] for k in item["keys"] if k["type"] in SKILL_OF_TYPE}
    if any(e in f for e in ("Cl", "Br", "S")):
        tags.add("isotopi")
    if "N" in f:
        tags.add("azoto")
    rdb = f.get("C", 0) - (f.get("H", 0) + f.get("Cl", 0) + f.get("Br", 0) + f.get("I", 0) + f.get("F", 0)) / 2 + f.get("N", 0) / 2 + 1
    if rdb >= 4:
        tags.add("rdb")
    if not any(m == round(mass_of(item["f"])) and v >= MIN_REL for m, v in peaks):
        tags.add("m-assente")
    return sorted(tags)


def fetch(url: str):
    CACHE.mkdir(parents=True, exist_ok=True)
    f = CACHE / (urllib.parse.quote(url, safe="")[-180:] + ".json")
    if f.exists():
        return json.loads(f.read_text(encoding="utf-8"))
    for k in range(4):
        try:
            d = json.load(urllib.request.urlopen(url, timeout=90))
            f.write_text(json.dumps(d), encoding="utf-8")
            time.sleep(0.15)
            return d
        except Exception as e:  # network hiccups: retry with backoff
            if k == 3:
                raise
            time.sleep(2 ** k)


def canon(formula: str) -> dict:
    from mzlab.chem.elements import parse_formula
    return dict(parse_formula(formula))


def peaks_of(rec) -> dict[int, float]:
    out: dict[int, float] = {}
    for p in rec["peak"]["peak"]["values"]:
        m = int(round(p["mz"]))
        out[m] = out.get(m, 0.0) + float(p["intensity"])
    top = max(out.values()) or 1.0
    return {m: v / top * 999 for m, v in out.items()}


def cosine(a: dict, b: dict) -> float:
    ks = set(a) | set(b)
    x = [math.sqrt(a.get(k, 0)) for k in ks]
    y = [math.sqrt(b.get(k, 0)) for k in ks]
    n = math.sqrt(sum(v * v for v in x)) * math.sqrt(sum(v * v for v in y))
    return sum(p * q for p, q in zip(x, y)) / n if n else 0.0


def records_for(item) -> list:
    want = canon(item["f"])
    syn = {s.upper() for s in item["en"]}
    seen, recs = set(), []
    for s in item["en"]:
        d = fetch(f"{API}/search?compound_name={urllib.parse.quote(s)}&instrument_type=EI-B&limit=500")
        for a in (d or {}).get("data", []):
            acc = a["accession"]
            if acc in seen:
                continue
            seen.add(acc)
            r = fetch(f"{API}/{acc}")
            names = {n.upper().strip() for n in r["compound"]["names"]}
            if not names & syn:
                continue
            try:
                if canon(r["compound"]["formula"]) != want:
                    continue
            except Exception:
                continue
            if r.get("license") not in OK_LICENSES or "TMS" in r.get("title", "") or "DERIVATIVE" in r.get("title", "").upper():
                continue
            from mzlab.chem.elements import mass, parse_formula
            if max(p["mz"] for p in r["peak"]["peak"]["values"]) > round(mass(parse_formula(item["f"]))) + 12:
                continue                                   # peaks far above the molecular ion: impurity or wrong record
            recs.append(r)
    return recs


def mass_of(formula: str) -> float:
    from mzlab.chem.elements import mass, parse_formula
    return mass(parse_formula(formula))


def iso_expected(formula: str) -> tuple[float, float]:
    """Expected M+1 and M+2 (% of M) at unit resolution, first-order sums of the IUPAC abundances (enough to rank spectra)."""
    f = canon(formula)
    a1 = {"C": 1.082, "H": 0.012, "N": 0.365, "O": 0.038, "S": 0.79, "Si": 5.08}
    a2 = {"O": 0.205, "S": 4.47, "Si": 3.35, "Cl": 31.99, "Br": 97.28}
    m1 = sum(a1.get(e, 0) * n for e, n in f.items())
    m2 = sum(a2.get(e, 0) * n for e, n in f.items()) + m1 * m1 / 200
    return m1, m2


def need(k) -> int:
    """Minimum relative intensity (per mille) a key ion must have in the chosen spectrum."""
    return 2 if k["type"] == "M" else 10


def build(item) -> dict | None:
    recs = records_for(item)
    if not recs:
        print(f"  !! {item['id']}: nessun record EI-B trovato")
        return None
    sp = [peaks_of(r) for r in recs]
    ok = [i for i, s in enumerate(sp) if all(s.get(k["mz"], 0) >= need(k) for k in item["keys"])]
    if not ok:
        miss = sorted({k["mz"] for k in item["keys"] for s in sp if s.get(k["mz"], 0) < need(k)})
        print(f"  !! {item['id']}: nessuno dei {len(recs)} spettri contiene tutte le chiavi (mancano {miss}); tolgo quelle chiavi")
        item["keys"] = [k for k in item["keys"] if any(s.get(k["mz"], 0) >= need(k) for s in sp)]
        ok = [i for i, s in enumerate(sp) if all(s.get(k["mz"], 0) >= need(k) for k in item["keys"])] or list(range(len(sp)))
    cos = {i: (sum(cosine(sp[i], sp[j]) for j in range(len(sp)) if j != i) / (len(sp) - 1) if len(sp) > 1 else 1.0) for i in ok}
    # isotopes: students estimate the carbons from M+1 and Cl/Br/S from M+2, so prefer spectra whose cluster agrees with the formula
    # (some old records have a large M+1 from self-protonation in the source, or a cut cluster)
    exp1, exp2 = iso_expected(item["f"])
    def iso_bad(s):
        m0 = s.get(round(mass_of(item["f"])), 0)
        if m0 < 30:
            return 0.5                                     # weak or missing M: cannot be judged, ranks after a good cluster
        o1, o2 = s.get(round(mass_of(item["f"])) + 1, 0) / m0 * 100, s.get(round(mass_of(item["f"])) + 2, 0) / m0 * 100
        return abs(o1 - exp1) / max(exp1, 2) + (abs(o2 - exp2) / max(exp2, 2) if exp2 > 2 else 0)
    good = [i for i in ok if iso_bad(sp[i]) < 0.45] or ok
    best = max(good, key=lambda i: cos[i] - 0.3 * iso_bad(sp[i]))
    if iso_bad(sp[best]) >= 0.45:
        print(f"  ~ {item['id']}: rapporti isotopici di M lontani dall'atteso in tutti gli spettri (M+1 atteso {exp1:.1f}%)")
    r, s = recs[best], sp[best]
    link = {l["database"]: l["identifier"] for l in r["compound"].get("link", [])}
    M0 = round(mass_of(item["f"])); m0 = s.get(M0, 0)
    iso = {"m1": round(s.get(M0 + 1, 0) / m0 * 100, 1) if m0 else None, "m2": round(s.get(M0 + 2, 0) / m0 * 100, 1) if m0 else None,
           "m1_exp": round(exp1, 1), "m2_exp": round(exp2, 1)}
    iso["ok"] = bool(m0 >= 30 and iso_bad(s) < 0.45)     # False: M too weak or cluster distorted (e.g. [M+H]+ from self-protonation)
    from mzlab.chem.elements import mass, parse_formula
    out = {
        "id": item["id"], "name": item["it"], "name_en": item["en"][0].capitalize(), "formula": item["f"],
        "M": round(mass(parse_formula(item["f"]))), "smiles": r["compound"].get("smiles", ""), "inchikey": link.get("INCHIKEY", ""),
        "family": item["fam"], "level": item["lv"], "play": item.get("play", True), "keys": item["keys"], "iso": iso,
        **PATHINFO.get(item["id"], {}),
        "peaks": [[m, round(v)] for m, v in sorted(s.items()) if v >= MIN_REL],
        "src": {"accession": r["accession"], "authors": ", ".join(a["name"].strip() for a in r.get("authors", []))[:160],
                "license": r.get("license", ""), "instrument": r["acquisition"].get("instrument", ""),
                "n_spectra": len(recs), "cos_mean": round(cos[best], 3)},
    }
    out["skills"] = skills_of(item, out["peaks"])
    print(f"  {item['id']}: {len(recs)} spettri, scelto {r['accession']} (coseno medio {cos[best]:.2f}), {len(out['peaks'])} picchi, {r.get('license')}")
    return out


def report(items) -> None:
    """«trovato / non trovato / sostituito» for every compound of the path and every candidate that has no EI-B record."""
    by = {i["id"]: i for i in items}
    rows = ["| livello | composto | stato | accessione MassBank | licenza | spettri |", "|---|---|---|---|---|---|"]
    for lv, ids in PATH.items():
        for i in ids:
            b = by.get(i)
            rows.append(f"| {lv} | {i} | " + (f"trovato | {b['src']['accession']} | {b['src']['license']} | {b['src']['n_spectra']} |" if b else "NON TROVATO | – | – | 0 |"))
    for want, (used, why) in SUBST.items():
        rows.append(f"| – | {want} | sostituito con {used} | – | – | {why} |")
    txt = "\n".join(rows)
    (ROOT / ".verifica").mkdir(exist_ok=True)
    (ROOT / ".verifica" / "rapporto_ei.md").write_text(txt + "\n", encoding="utf-8")
    print(txt)


PATHINFO = {i: {"path": lv, "order": k} for lv, ids in PATH.items() for k, i in enumerate(ids)}


def main() -> None:
    assert len(PATHINFO) == sum(len(v) for v in PATH.values()), "a compound is twice in PATH"
    assert set(PATHINFO) <= {i["id"] for i in LIST}, sorted(set(PATHINFO) - {i["id"] for i in LIST})
    items = []
    for it in LIST:
        b = build(dict(it))
        if b:
            items.append(b)
    lic = sorted({i["src"]["license"] for i in items})
    head = ("// Generated by tools/genera_ei.py from MassBank Europe (https://massbank.eu): do not edit by hand.\n"
            "// LICENCE OF THIS FILE: the spectra are MassBank records under " + ", ".join(lic) + " (attribution in every item: accession,\n"
            "// authors, licence). The whole file is distributed under CC BY-NC-SA 4.0 (non-commercial teaching use), not under the MIT\n"
            "// licence of the program. The 'keys' (teaching notes) are by the mzLab authors, same licence.\n")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    extra_ids = {i["id"] for i in LIST if i.get("extra")}
    base = [i for i in items if i["id"] not in extra_ids]
    levels: dict[int, list] = {}
    for i in items:
        if i["id"] in extra_ids:
            levels.setdefault(i.get("path", 4), []).append(i)          # reference spectra without a path level travel with level 4
    index = {i["id"]: lv for lv, grp in levels.items() for i in grp}       # id -> level file that holds it
    dump = lambda o: json.dumps(o, ensure_ascii=False, separators=(",", ":"))
    OUT.write_text(head + "const EI_DATA = " + dump({"version": 3, "source": "MassBank Europe", "items": base, "extra": index}) + ";\n", encoding="utf-8")
    for old in OUT.parent.glob("ei-dati-l*.js"):
        old.unlink()
    for lv, grp in sorted(levels.items()):
        (OUT.parent / f"ei-dati-l{lv}.js").write_text(head + "EI_DATA.items.push(..." + dump(grp) + ");\n", encoding="utf-8")
    items = base + [i for g in levels.values() for i in g]
    report(items)
    print(f"{len(items)} composti ({sum(i['play'] for i in items)} giocabili) -> {OUT.relative_to(ROOT)} ({OUT.stat().st_size // 1024} kB)")


if __name__ == "__main__":
    import sys
    sys.path.insert(0, str(ROOT))
    main()
