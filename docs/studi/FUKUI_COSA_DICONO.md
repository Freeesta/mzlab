# Fukui: cosa dicono e cosa no

Validazione degli indici di Fukui condensati già calcolati da `mzlab-chem` (`reatt/1`, GFN2-xTB con ALPB acqua, cariche di Mulliken; `docs/agenti/reattivita.md`), per lo studio sul timololo (`TABUI_TP_TIMOLOLO.md` §8b). Nessun codice di prodotto toccato. Riproduzione: `python3 tools/fukui_valida/valida.py` (costruisce da solo il piccolo eseguibile `tools/fukui_valida`, fuori dal workspace; stampa le tabelle qui sotto). I TP e i siti di letteratura citati sono ipotesi, non verità.

## 0. Verdetto
- **Sì, come ordinamento, solo dentro un anello π e per la sostituzione elettrofila.** Su 17 molecole semplici f(−) mette un sito atteso al primo posto in 16 e nei primi tre in 17; con un margine decisivo (≥ 0,005) in 12.
- **No, come prova di posizione**, e **no sul timololo**: nessun C–H insaturo (`ch_sites` è vuoto); sui C sp3 α a N/O f(−) del solo carbonio è negativo o nullo, e l'ordine cambia se si sommano gli H (scelta arbitraria); gli equivalenti per simmetria differiscono quanto i siti da distinguere. Per il timololo f(−) indica gli eteroatomi (S, N), non i C–H.
- **Raccomandazione**: usarli per il timololo solo come informazione secondaria «Prev.» sugli eteroatomi (S, N), mai per riordinare le posizioni C–H alifatiche né per cambiare confidenza (come già in `posiz/1`). Per i C–H α a N/O serve un altro indicatore (BDE C–H, già provata e scartata una volta: `reattivita-studi.md`; si riprova solo con un insieme di riferimento dedicato).

## 1. Cosa esiste nel repository
| Cosa | Dove |
|---|---|
| Motore GFN2-xTB + ALPB, Fukui f(+), f(−), f(0), duale; siti C–H (`ch_sites`) | `crates/mzlab-chem/src/fukui.rs`, `reatt.rs`; `.wasm` `chem` |
| Parità con tblite 0.7.0 (stessa geometria): |Δq| ≤ 2e-3, |Δf| ≤ 3e-3 | `crates/mzlab-chem/tests/parita_gfn2.rs`, `cc_ref.json` |
| Riferimento xtb 6.7.1 `--vfukui` (gas e acqua), 20 molecole | `tests/data/xtb/` (`*_fukui.csv`, `*_xtb.out`, `LEGGIMI.md`) |
| Siti sperimentali (·OH, ozono, metabolismo), 23 molecole, 57 righe con DOI | `tests/data/reattivita/validazione.json`, `crates/mzlab-chem/tests/validazione.rs` (misure in `docs/agenti/reattivita-studi.md`) |
| Prior f(−) in `posiz/1`, mostrato come «Prev.», mai usato per riordinare | `crates/mzlab-feat/src/chem/posizioni/prior.rs` |

Dati di partenza già misurati (`cargo test --release -p mzlab-chem --test validazione`, rieseguito qui): siti C–H, ·OH, letteratura (n = 20) top-1/3/5 = 8/17/18; bozza CC-05 (n = 9) 9/9/9; **tutti gli atomi pesanti** (n = 39) 0/4/9, ozono 0/0/1, metabolismo 0/0/1. Il motore non pubblica una classifica per i siti non C–H insaturi.

## 2. Coerenza interna (17 molecole, acqua)
Tutte e 17 convergono (N, N+1, N−1). Σq tra −2e-4 e +2e-4; Σf(+) e Σf(−) tra 0,9997 e 1,0002 (somme su tutti gli atomi, H compresi, arrotondamento a 4 decimali); f(0) = (f(+) + f(−))/2 entro 1e-4. Valori negativi di f(−) su atomi pesanti: 0–1 per molecola (anisolo, toluene, acetanilide, acetofenone: sempre il carbonio del metile), cioè la perdita di un elettrone riduce la popolazione di Mulliken di quell'atomo: succede e va letto così, non come errore. Una proprietà di conservazione, non di accuratezza chimica.

## 3. Sostituzione elettrofila su molecole semplici (f(−), primo sito C–H, 4 semi di embedding)
Margine = miglior f(−) fra i C attesi meno miglior f(−) fra i C sfavoriti; «tie» se |margine| < 0,005 (soglia `TIE`).

| Molecola | Atteso (C, SMILES 1-based) | Primo sito (f−) | Margine | Esito |
|---|---|---|---|---|
| fenolo | o/p 3,5,7 | C5 = para (+0,080) | +0,046 | giusto, 4/4 semi |
| anisolo | o/p 4,6,8 | C6 = para (+0,065) | +0,035 | giusto |
| anilina | o/p 3,5,7 | C5 = para (+0,069) | +0,041 | giusto |
| toluene | o/p 3,5,7 | C5 = para (+0,068) | +0,038 | giusto |
| clorobenzene | o/p 3,5,7 | C5 = para (+0,070) | +0,036 | giusto |
| acetanilide | o/p 6,8,10 | C8 = para (+0,040) | +0,013 | giusto |
| nitrobenzene | meta 6,8 | meta (+0,039) | +0,006 | giusto, margine piccolo |
| acido benzoico | meta 6,8 | meta (+0,045) | +0,002 | pari (tie) con orto |
| benzaldeide | meta 5,7 | meta (+0,033) | +0,002 | tie |
| acetofenone | meta 6,8 | meta (+0,030) | +0,003 | tie |
| benzonitrile | meta 5,7 | meta (+0,047) | +0,004 | tie |
| naftalene | α 3,5,8,10 | α (+0,054) | +0,019 | giusto |
| pirrolo | α 3,5 | α (+0,104) | +0,050 | giusto |
| furano | α 3,5 | α (+0,104) | +0,044 | giusto |
| tiofene | α 3,5 | **β (C1,C2, +0,053)** | −0,028 | **sbagliato**, 0/4 semi |
| indolo | C3 (7) | C7 = C3 (+0,071) | +0,034 | giusto |
| piridina | β 3,5 | β (+0,039) | +0,017 | giusto |

Letture: (1) para è sempre sopra orto negli attivanti (è l'ordine osservato); (2) per i disattivanti meta primo, ma quasi in parità con orto: l'ordinamento non è informativo; (3) tiofene è un errore netto e stabile (un primo posto sbagliato non si vede dalle somme del §2); (4) f(−) descrive la perdita di un elettrone π (HOMO), non la reattività verso un radicale ·OH né lo stato di transizione.

## 4. Confronto con xtb 6.7.1 (20 molecole, acqua)
mzLab usa la propria geometria ETKDG, xtb la geometria MMFF dei file: le differenze sono quindi di geometria, non di metodo (a geometria uguale la parità con tblite è entro 3e-3). Correlazione di Spearman su f(−) degli atomi pesanti ≥ 0,83 in 19 molecole su 20, ≥ 0,95 in 12 (la mediana è 0,95); unica sotto: sulfametossazolo (0,56, max |Δf(−)| 0,20, molecola flessibile). Spearman su f(+) 0,32 (paracetamolo) – 0,98. Primo sito C–H uguale (o a pari merito) in 18 su 19 molecole con C–H insaturi; unica differenza il diclofenac, il cui primo sito cambia anche tra gas e acqua in `tests/data/xtb/LEGGIMI.md` (siti quasi pari). Dunque anche una scelta di conformero può cambiare l'ordine di siti vicini.

## 5. Timololo (`CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1`)
Atomi pesanti 0-based nell'ordine dello SMILES; morfolina: N15, C16/C20 (α a N), C17/C19 (α a O), O18; catena: N4, C5 (NCH2), C6 (CHOH), C8 (OCH2), O9.

| Gruppo | f(−) del solo atomo | note |
|---|---|---|
| S12 | +0,129 (1° di 21) | S-ossidazione plausibile |
| N4 (NH tBu) | +0,123 (2°) | N-ossidazione/dealchilazione |
| N15 (N morfolina) | +0,112 (3°) | coniugato col tiadiazolo |
| O9 (etere) | +0,033 (4°) | |
| C α a N (5, 16, 20) | −0,025, −0,019, −0,023 (19°–21°) | negativi |
| C α a O (8, 17, 19) | −0,013, −0,004, −0,009 | negativi o nulli |

- `ch_sites` è vuoto (nessun C insaturo con H): il motore non propone siti C–H.
- Sommando gli H al C (f(−) del gruppo CH₂), i C α a N salgono in testa (C20 primo in 3 semi su 4, poi C5 della catena, poi C16): ma (a) il risultato è prodotto dagli H, non dal carbonio: cambiare schema di condensazione cambia l'ordine; (b) C5, che non è nella morfolina, resta sempre fra i primi tre e non è separabile dalla morfolina; (c) i due C α a N della morfolina, equivalenti per simmetria, differiscono di 0,008–0,028 (stesso ordine di grandezza delle differenze fra siti); (d) i C α a O (17, 19) cadono nelle ultime posizioni (6°–10° su 10) mentre le ipotesi di letteratura hanno anche aperture ed ossidazioni sul lato dell'O; (e) lo spread di f(−) sui conformeri è 0,026.
- In gas i primi quattro sono S12 (+0,198), N15 e N4 (+0,101), O9: stesse classi.
- Letteratura (ipotesi, `docs/letteratura_cc/tp_noti/tp_noti.csv`): taglio della morfolina, apertura dell'anello, +O −H₂, S-ossidazione a diossido (PD5). I Fukui sono coerenti solo con la parte S/N; sulle posizioni C–H dicono troppo poco.

## 6. Errori e limiti
- f(−) è un descrittore di donazione elettronica (HOMO) per attacchi elettrofili su π: non copre l'astrazione di H (·OH su C sp3), le scissioni, né i siti non aromatici (0/39 top-1 su tutti gli atomi pesanti nel set ·OH).
- Mulliken: i valori dipendono dalla base e dal modo di assegnare gli H; negativi possibili; sensibilità alla geometria (§4) e al conformero (§5).
- Disattivanti: meta e orto quasi pari (±0,005): l'ordine non è informativo.
- Insieme di prova piccolo (17 + 20 + 23 molecole), non indipendente dalla taratura; manca il confronto con DFT/standard sul timololo; nessuna statistica di significatività.

## 7. Se serve altro
`tools/fukui_valida/` (runner Rust fuori workspace + `valida.py`) si estende aggiungendo righe a `CASES`. Per un'ordinamento dei C–H alifatici del timololo l'indicatore da provare è la BDE C–H (GFN2 a guscio aperto) con un insieme di riferimento di ammine/eteri, come in `TABUI_TP_TIMOLOLO.md` §8b punto 3; non è una decisione di questa nota.
