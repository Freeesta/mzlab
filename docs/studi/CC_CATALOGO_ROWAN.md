# Catalogo Rowan: cosa serve per i prodotti di trasformazione

Complemento di `CC_MOTORE_NEL_BROWSER.md` (§2F). Per ogni calcolo del catalogo Rowan (labs.rowansci.com) che riguarda i TP: valore per chi cerca TP a risoluzione unitaria, metodo realistico in Rust nel browser, costo, rischio, priorità. Tutto resta «Prev.», senza livello di Schymanski, e non esclude mai classi di posizione. I costi sono giorni-chat stimati, non misurati.

## Valutazione
| Calcolo | Valore per i TP | Metodo in Rust nel browser | Costo | Rischio | Priorità |
|---|---|---|---|---|---|
| Fukui f(−)/f(0)/f(+) | alto: ordina i siti di attacco di •OH (elettrofilo) sul parent | GFN2 + ALPB, differenze finite (CC-B2) | in CC-B2 | medio (convergenza degli ioni) | **1** |
| Ricerca di conformeri | alto: serve al 3D e rende onesto f(−) (dispersione sui 3 più bassi) | ETKDG + MMFF94 da sci-form, raggruppamento per RMSD (CC-B1) | in CC-B1 | basso | **1** |
| HOMO/LUMO, gap | medio: confronto tra parent e TP proposti; il colore per atomo viene già dalle Fukui | autovalori GFN2 (CC-B2, scheda «Info» in CC-B6); niente isosuperfici (griglie e marching cubes: dopo, su richiesta) | < 0,5 | basso; GFN2 sottostima il gap | **2** |
| BDE C–H | medio: siti di astrazione di H da parte di •OH sulle catene alifatiche, dove le Fukui π dicono poco | GFN2: E(R•) + E(H•) − E(RH) con radicale a geometria rilassata (open-shell da scrivere) | 1,5-2 (CC-B6) | medio-alto: va validata su BDE sperimentali prima di mostrarla | **2** |
| Ricerca di tautomeri | medio: stesso m/z, strutture diverse (cheto/enolo, ossime, idrossipiridine): il TP disegnato può non essere la forma presente | enumerazione per regole sul grafo di `mzlab_feat::chem::grafo` (spostamenti 1,3 e 1,5 di H), poi ordine per energia GFN2 + ALPB | 2-3 (CC-B7) | medio: le differenze di energia sotto ~2 kcal/mol non sono affidabili a questo livello | **3** |
| logP e descrittori di polarità | alto in principio: ordine di eluizione in RP-LC degli isomeri TP A/B | Crippen logP per contributi atomici (Wildman, Crippen 1999), momento di dipolo GFN2, legame H intramolecolare, ΔG di solvatazione ALPB | ~2 (CC-B8) | medio-alto: i contributi atomici danno quasi lo stesso logP a isomeri di posizione; si mostra solo dopo la prova su coppie di isomeri con RT misurato (criterio: ordine giusto ≥ 8/10 coppie) | **3**, solo con la validazione |
| pKa microscopico | medio: ione dominante, carica attesa, addotti in ESI+ | **non fattibile** come numero affidabile a questo costo; fattibile solo l'elenco qualitativo dei gruppi ionizzabili e la carica attesa a pH ~2,7 (acido formico) per regole sul grafo, dentro CC-B7 | 0,5 (qualitativo) | alto se numerico, basso se qualitativo | **3** (qualitativo) |
| Potenziale redox | medio-basso: ordina la facilità di ossidazione tra parent simili (trasferimento di elettrone a TiO2 h⁺) | ΔSCF GFN2 + ALPB (IP adiabatico) con retta di calibrazione su potenziali misurati; solo confronto relativo | ~1 (dentro CC-B6) | medio-alto: errore atteso dell'ordine di 0,3-0,5 V (da verificare sulla calibrazione) | **4** |
| CCS (mobilità ionica) | basso per il 3200 (senza mobilità); utile solo con dati IMS (mzML con mobilità) per distinguere candidati | approssimazione delle proiezioni o EHSS sul 3D dello ione (Mesleh et al. 1996 per i metodi di MobCal), in Rust, millisecondi | 1,5-2 | alto: errore di qualche % (da verificare), spesso maggiore della differenza tra isomeri | **5**, da decidere quando i dati IMS saranno in uso |

## Non pertinenti per i TP (non si fanno)
Docking, proteine, fononi, IRC e stati di transizione, superfici, solubilità, ADME-Tox, NMR.

## Ordine proposto
1. Fukui e conformeri (CC-B1, CC-B2): il cuore del pannello.
2. HOMO/LUMO e BDE C–H (CC-B6): BDE solo se la validazione passa.
3. Tautomeri con ionizzazione qualitativa (CC-B7) e polarità per l'ordine di eluizione (CC-B8): facoltativi, CC-B8 soltanto con le coppie di isomeri a RT misurato fornite da Federico (dati nel repository privato).
4. Redox dentro CC-B6 se resta tempo; CCS rinviato.

## Fonti
- Rowan: labs.rowansci.com (elenco dei calcoli indicato dal Coordinatore; i metodi interni di Rowan non sono ripresi).
- Wildman S. A., Crippen G. M. (1999) Prediction of physicochemical parameters by atomic contributions. J. Chem. Inf. Comput. Sci. doi:10.1021/ci990307l
- Mesleh M. F. et al. (1996) Structural information from ion mobility measurements: effects of the long-range potential. J. Phys. Chem. doi:10.1021/jp961623v
- GFN2-xTB, ALPB, ETKDG, MMFF94: bibliografia di `CC_MOTORE_NEL_BROWSER.md` §10.
