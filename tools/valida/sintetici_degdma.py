"""A hard family for the identity series: a molecule whose [M+H]+ is weak, strong Na/K/NH4 adducts, an in-source fragment that is the base peak of the MS1 (proved by
the MS2 of the parent) and the same fragment m/z coming from a product that elutes elsewhere (it must not be taken for a compound of the parent family).
Every formula and number is invented. Added by `sintetici_identita.costruisci` for the profile key `degdma`, with its own random generator (the draws of the
other profiles do not move).

Truth rows added: the molecule (ruolo `progenitore`), `addotto` Na, K and NH4, the `isf` base peak (`isf_in_ms2` = 1), and `isf_finto` of the product at its own
RT with the same m/z (`isf_in_ms2` = 0 for that parent: the product's MS2 does not contain it).
"""
from __future__ import annotations

import numpy as np

from mzlab.chem import elements as E

NA_DELTA, K_DELTA, NH4_DELTA = 21.98194, 37.95588, 17.02655
NEUTRA = {"C": 13, "H": 20, "O": 5}
FRAMMENTO = {"C": 9, "H": 9, "O": 2}


def aggiungi(c, somma, modello, Ione, frammenti_di) -> None:
    rng = np.random.default_rng(c.seme + 4242)
    mid = c.id_mol()
    c.formule.append(NEUTRA)
    kd = 0.04
    amp = c.traccia("progenitore", 4e6, kd=kd)           # weak protonated molecule
    rt = c.slot(1.0)
    m = c.ione(mid, "progenitore", mid, NEUTRA, rt, 0.06, amp, generazione=0, forma="progenitore", k_dec=kd, selezionabile=True)
    m.note = "molecola_debole;addotti_forti"
    for nome, delta, ad, r in (("Na", NA_DELTA, "[M+Na]+", 6.0), ("K", K_DELTA, "[M+K]+", 2.5), ("NH4", NH4_DELTA, "[M+NH4]+", 3.0)):
        c.ione(f"{mid}_{nome}", "addotto", mid, NEUTRA, rt + float(rng.uniform(-0.008, 0.008)), 0.06, amp * r, adduct=ad, progenitore=mid, forma="come_progenitore",
               note=f"ratio={r};delta_mz={delta}")
    # in-source fragment: the base peak of the MS1, and a fragment of the MS2 of the molecule
    frag = Ione(id=f"{mid}_ISF1", ruolo="isf", famiglia=mid, rt=rt + float(rng.uniform(-0.008, 0.008)), sg=0.06, progenitore=mid, forma="come_progenitore", isf_in_ms2=1,
                amp=amp * 12.0, note="ratio=12.0;picco_base")
    frag.ion = E.fmt(FRAMMENTO)
    frag.picchi, mz, _ = modello(frag.ion)
    frag.mz = float(mz[0])
    c.ioni.append(frag)
    m.frammenti = frammenti_di(rng, m.mz, extra=[(frag.mz, 0.9)])
    # a product of the molecule that eluates later and gives the same fragment in the source (its MS2 does not have it)
    tp, f = c.prodotto(m, NEUTRA, forma="monotona", rt=rt + 6.0)
    c.formule.append(f)
    c.rt_cursor = max(c.rt_cursor, tp.rt + 1.0)          # the run must reach the product
    twin = Ione(id=f"{tp.id}_ISFF1", ruolo="isf_finto", famiglia=tp.id, rt=tp.rt + float(rng.uniform(-0.008, 0.008)), sg=tp.sg, progenitore=tp.id, forma="come_progenitore",
                isf_in_ms2=0, amp=tp.amp * 0.8, mz=frag.mz, ion=frag.ion, picchi=list(frag.picchi), note="ratio=0.8;stesso_mz_di=" + frag.id)
    c.ioni.append(twin)
    tp.frammenti = frammenti_di(rng, tp.mz, evita=[frag.mz])
