# Verifica del lotto 14 di Gemini (crate `mzlab-mol`)

Fase 1: solo lettura e misura, nessuna modifica al codice. Ogni voce del report è confrontata con il codice di `origin/main`; i tempi sono misurati in release (`cargo run --release`, un solo thread, macchina condivisa: ±15 %). Banco di prova: caffeina, colesterolo, coronene, adamantano, cubano, 18-corona-6, C60, tripeptide, più i 812 SMILES di `tests/data/golden*.json` per l'aromaticità. Banchi in `scratchpad/bench14/` (non nel repository).

Legenda: **V** vero, **F** falso, **P** parziale. Gruppo: **a** sicuro (output identico), **b** bug reale (output cambia), **c** da non fare.

## Esito in breve

- Il guadagno vero è altrove rispetto a quanto dice Gemini: `Ctx::new` (`descr.rs:17,40`) percepisce l'aromaticità **due volte** e `descriptors()` costruisce **cinque** `Ctx` (`descr.rs:156,185,194,201,221`); `aromatic_bonds` a sua volta calcola `rings()` due volte. Totale: 10 percezioni di anelli per `descriptors()`, ~80-100 % del tempo. Sistemarlo lascia l'output identico.
- Gemini ha trovato un bug reale in `aromatic.rs::donated` (anelli uniti da un doppio legame ponte), ma il **fix che propone rompe molecole vere** (5 su 812 del corpus golden cambiano, es. fenantrene 16→12 legami aromatici in una forma di Kekulé). Fix corretto proposto sotto (B1).
- Tre suggerimenti sono sbagliati e non vanno applicati: `max+1` come numero di classi in `refine` (F), `c.nb[i].first()` per l'idrogeno esplicito in Crippen (F, `nb` di un H è sempre vuota), certificato binario in `canon` (cambia l'ordine canonico e quindi ogni hash).
- «20 000 foglie» e «128 `with_values`» sono i casi peggiori, non quelli reali: misurate 1-33 foglie in `canon` e 8-16 chiamate in `flips`.

## Tabella per voce

| # | Voce di Gemini | Esito | File:riga e prova | Gruppo |
|---|---|---|---|---|
| 1.1 | `Graph::new` scarta `rings()`, `aromatic_bonds` lo rifà | **V** | `aromatic.rs:12` e `:68`; `rings()` non usa `in_ring`, quindi si può passare il risultato. Misura: `aromatic_bonds` colesterolo 32,4→16,6 µs, caffeina 11,4→7,4, coronene 48→29, C60 283→189 | a |
| 1.2 | BFS con `prev`/`VecDeque` nuovi per legame; `seen: BTreeSet<Vec>` con `clone` | **V** (piccolo) | `aromatic.rs:94-97`, `:76-84`. Il `clone` avviene per ogni legame d'anello, anche se duplicato. Con `sort`+`dedup` serve un sort **stabile** (si tiene il primo anello a parità di atomi); l'ordine degli anelli non conta (i flag si fanno con OR) | a |
| 1.3 | Unione di anelli fusi via `BTreeSet` | **V** (micro) | `aromatic.rs:30-31`; `atoms` già ordinati (`key.sort_unstable`, `:83`) | a |
| 1.4a | `double: Vec` solo per contare | **V** (micro) | `aromatic.rs:143-148` | a |
| 1.4b | «Bug»: `in_ring[v]` conta 1 elettrone per un doppio legame esociclico verso un altro anello | **P** | Il bug esiste (B1). Ma `ring_atoms.binary_search(&v)` **peggiora**: la forma di Kekulé con C4a=C4b del fenantrene passa da 16 a 12 legami aromatici (l'anello laterale ha il doppio legame verso un atomo dell'anello centrale, fuso, non nell'insieme testato). Sul corpus: 7 molecole su 812 cambiano, 5 sono molecole vere regredite | b |
| 2.1 | `refine` alloca `Vec<Vec>` per atomo, `BTreeSet` iniziale, «colli critici» | **P** | `canon.rs:80-87`: alloca davvero. Quota di `refine` nel tempo di `canonical_order`: 25-32 % (piccole), 52 % (coronene, 18-corona-6), 64 % (C60). Con buffer piatti + ordinamento di indici (prototipo): C60 4632→3633 µs (-22 %), coronene -4,5 %, 18-corona-6 -11 %, piccole -1…-5 %; **ordini canonici identici** (checksum FNV sui permessi delle 8 molecole uguale). `SmallVec` = nuova dipendenza: no. `cls.iter().max()+1` al posto del `BTreeSet`: **F**, `explore` passa `split` (`:118-121`, valori `cls*2+bit`, non densi): un conteggio gonfiato può far fermare il ciclo prima della partizione equa | a (buffer piatti), c (`max+1`, `SmallVec`) |
| 2.2 | `a.el.clone()` in `initial` | **V** (trivial) | `canon.rs:70`; una volta per `canonical_order`, chiave `(&str,…)` basta | a |
| 2.3 | Foglia = `Mol` + JSON, «fino a 20 000 volte» | **P** | `canon.rs:129-149`. Foglie reali: caffeina 1, colesterolo 2, adamantano/cubano 7, 18-corona-6 3, C60 33 (il potatore per automorfismi funziona). Il costo foglia è comunque 33-63 % del tempo (8,7 µs su 16,4 caffeina; 1565 su 4632 C60). Certificato **binario**: confronta con `cert < *b` su testo JSON (`:145`); un ordine binario scelto diverso sceglie un'altra foglia → ordine canonico e ogni hash cambiano. Early-abort sul testo JSON richiederebbe un serializzatore a flusso: complessità alta | c (binario); a solo «salta `move_layers` nella foglia» (il core non dipende dai layer), non misurato |
| 3.1 | Accessori `singles/doubles/triples/arom_nb` allocano | **V** (piccolo) | `descr.rs:103-122`; dopo il fix `Ctx::new` la tipizzazione vale 2-9 µs (colesterolo 25,5 totali, di cui 16,6 aromaticità): risparmio atteso ≤ 10 % di `crippen_logp` | a (bassa priorità) |
| 3.2 | Scansione O(B) per H esplicito → `c.nb[i].first()` | **F** (soluzione) | `descr.rs:25-33`: `nb[x]` si riempie solo se `atoms[x].el != "H"`; per un H `nb` è **sempre vuota**, il `first()` darebbe tutti `HS` e cambierebbe logP. La scansione O(B) è vera ma trascurabile; alternativa corretta: un vettore `host[h]` riempito in `Ctx::new` | c (come scritto) |
| 3.3 | `if q == 0 { return NS }` ridondante in `nitrogen` | **V** (cosmetico) | `crippen.rs:273` dentro `if q == 0`; il compilatore già lo elimina, guadagno 0 | a |
| 4.1 | `flat(d)`/`drawn(d)` ricalcolati | **V** (micro) | `molfile_stereo.rs:62` (per centro), `:194` (per doppio legame); O(atomi) ciascuno | a |
| 4.2 | `adjacency(d)` costruita due volte | **V** (micro) | `molfile_stereo.rs:46` e `:152` | a |
| 4.3 | `Vec` in `parity` → array fissi | **V** (nessun guadagno) | `parity` è chiamata solo da `tetra` dopo `k==3 \|\| k==4` (`:50`, `:64`), quindi 3-4 vicini sono garantiti | a |
| 4.3b | `powf(1.5)` → `vol²<1e-12·s³` | **P** | Matematicamente uguale, non bit per bit: la soglia `:123` può decidere diversamente in una banda sottilissima (`vol` quasi esattamente `1e-6·scale^1.5`). Guadagno trascurabile (una `powf` per centro) | c |
| 4.4 | `in_small_ring`: BFS fino all'intero grafo | **V** | `molfile_stereo.rs:130-147`; fermarsi a distanza 6 è equivalente (a distanza 7 il codice attuale restituisce comunque `false`). Per un doppio legame aciclico oggi esplora tutto il componente. Guadagno: O(N) per doppio legame, µs su molfile di centinaia di atomi | a |
| 5.1 | Controllo `len` posticipato in `renumbered` | **P** | `renumber.rs:12-26`: spostarlo non guadagna nulla e cambia il testo dell'errore per `len > n` («non è una permutazione» → «un indice per atomo»); `old < n` serve proprio per quel caso | c |
| 5.2 | `..c.clone()` clona `xy` per scartarlo (e `Prop`, `Region`) | **V**, snippet **incompleto** | `renumber.rs:74-94`. `Coords` ha anche `from: String` (`model.rs:101-105`), `Prop` ha `name/from/can`: lo snippet di Gemini non compila e, omettendo `from`, perderebbe dati. Con campi espliciti: `renumbered` con coordinate 5,06 → ~3,9 µs (-1,2 µs, `xy` di 28 atomi = 28 allocazioni scartate). Output identico | a |
| 5.3 | `inversions` con `flat_map` → doppio `for` | **V** (cosmetico) | `renumber.rs:181-186` (e identico in `smiles_write.rs:154-157`); 3-4 elementi | a |
| 6.1 | `flips`: 128 `with_values` con `neighbours()` ricalcolato | **P** | `sites.rs:132-167`. Il tetto 128 è solo il caso «mai un flip». Misurato: adamantano 8 chiamate, cubano 16, inositolo 12. `neighbours()` + tutto `with_values` senza canonico = 39 µs su 1650 (adamantano), 20 su 10 090 (cubano): **<2 %**. Il costo vero è `canonical()` su record con stereo definito in molecole simmetriche (cubano 10,1 ms, adamantano 1,65 ms, inositolo 1,96 ms per `stereo_sites`). Il fix proposto (`work_mol`) non lo tocca | a (guadagno <2 %, non vale) |
| 6.2 | `dead_pair`: due DFS e 6 vettori | **V**, equivalente | `sites.rs:221-241`: `y∈bx` ⇔ y raggiungibile da x senza c; i due rami sono disgiunti altrimenti. Una DFS e uscita al primo `live` danno lo stesso booleano. Guadagno non misurabile (colesterolo: `stereo_sites` intero 40 µs) | a |
| 7.1 | `atom_text`/`ring_digit`/`tail` allocano `String` | **V** | `smiles_write.rs:165-185, 202-208, 240-265`. Riscrittura con `write!` su un `String` unico: stesso testo, tranne `chg == i8::MIN` (oggi `-(-128)` in release dà «--128», in debug panico; con `{n}` «-128»). Guadagno atteso 20-30 % di `to_smiles` (colesterolo 11,7 µs), non misurato | a |
| 7.2 | `closures[u]` clonato e ordinato due volte | **V** | `smiles_write.rs:129-130` e `:213-214`; ordinare una volta a fine `visit` di ogni componente è valido (le `pos` dei partner sono complete) | a |
| 7.3 | Cifra di anello con `u64` | **P** | `smiles_write.rs:220`: il codice attuale non ha tetto. `used \| 1` e `trailing_ones` sono corretti fino a 63 anelli aperti insieme, poi `1 << 64` va in panico. Servono `u128` o ripiego | a (con `u128`/ripiego) |
| 8.1 | `key_atoms() -> Vec` | **V** (nessun guadagno) | `trasf_chain.rs:12-17`, `trasf.rs:144-153`; 1-2 elementi, chiamato una volta per elemento stereo del centro | a |
| 8.2 | `contains` → `binary_search` su `centre/added/atoms` | **P** | `validate_certain` garantisce `centre`/`added` strettamente crescenti (`trasf.rs:65,71`) e `validate_uncertain` gli atomi della regione (`:90`), ma `Trasf` ha campi `pub` e `compose`/`touched_stereo` possono ricevere un record non validato; su 1-5 elementi la scansione lineare è altrettanto veloce. `BTreeSet` in `validate_certain` → `vec<bool>`: richiede di spostare il controllo `h >= size` (`:68`) prima, e per un record doppiamente errato cambia il messaggio che compare per primo | c |
| 8.3 | `Trasf::canonical`: saltare `renumbered` se `po` è l'identità | **P** | `trasf_chain.rs:136`: `renumbered` normalizza anche l'ordine di legami/stereo (`renumber.rs:37,44`) e `check_bonds` non richiede che siano ordinati: con `po` identità il record può avere comunque un `core_json` diverso. Corretto solo se il record è già normalizzato, cosa da verificare con un confronto che costa quanto l'hash | c |
| 9.1 | `check_bonds`: legami ordinati, `windows` al posto del `BTreeSet` | **F** (premessa) | `validate.rs:34-55`: `validate` si applica a ogni record (`from_json`), e le `bonds` non devono essere ordinate; solo `renumbered` le ordina. La variante «`adj.contains` durante `check_bonds`» è invece **V** e identica (stesso primo errore, stesso messaggio). `validate` intero: 1,2-11 µs, guadagno sotto 1 µs | a (variante `adj`) |
| 9.2 | `check_groups`: scansione di `stereo` e `BTreeSet` per atomo | **V** | `validate.rs:169-193`; i gruppi sono rari, gli `Stereo` pochi: nessun guadagno misurabile | a (bassa priorità) |

## Gruppo (a): sicuri, output identico, in ordine di valore

| Priorità | Cosa | Dove | Guadagno misurato |
|---|---|---|---|
| 1 | `Ctx::new` calcola `aromatic_bonds()` una volta e ne deriva `ar` (oggi anche `aromatic_atoms()` richiama `aromatic_bonds()`) | `descr.rs:17,40` | `crippen_logp`: caffeina 29,6→10,2 µs, colesterolo 69→25,5, C60 627→204 (-65…-67 %) |
| 2 | `descriptors()` con un solo `Ctx` (oggi 5: `contributions`, `tpsa`, `hbd`, `hba`, `rotatable_bonds`) | `descr.rs:156,185,194,201,221` | percezione ×10 = 78-116 % del tempo di `descriptors()` (159-570 µs); atteso ≥5×. Servono varianti interne che prendono `&Ctx`; i metodi pubblici restano |
| 3 | `Graph::new` restituisce gli anelli (1.1) | `aromatic.rs:12,68` | `aromatic_bonds` -35…-49 % |
| 4 | Un solo `aromatic_bonds` in `from_record` (oggi `aromatic_atoms()` + `aromatic_bonds()`) | `mzlab-feat/src/chem/grafo_record.rs:24` | -50 % di quella chiamata |
| 5 | `refine` con buffer piatti (2.1) | `canon.rs:77-96` | `canonical_order` -5…-22 % (C60 -22 %); ordini identici verificati |
| 6 | `move_layers` con campi espliciti (5.2) | `renumber.rs:72-95` | -1,2 µs su 5 con coordinate |
| 7 | `smiles_write` con `write!` e chiusure ordinate una volta (7.1-7.3) | `smiles_write.rs` | atteso 20-30 % di `to_smiles`, non misurato |
| 8 | Micro: 1.2, 1.3, 1.4a, 2.2, 3.1, 4.1, 4.2, 4.4, 5.3, 6.2, 9.1 variante `adj`, 9.2 | vari | sotto il rumore; cosmesi, da accorpare solo se si tocca già il file |

Prova di identità consigliata per la fase 2: `cargo test -p mzlab-mol` (golden RDKit su 812 SMILES + `golden_molfile`), e un confronto byte per byte di `canonical_order`, `to_smiles`, `core_json`, `descriptors()` su un corpus (≥ 2 000 record dei composti già presenti in `golden.json` `composti`) tra `origin/main` e il ramo.

## Gruppo (b): bug reali (cambia l'output)

### B1. Doppio legame ponte tra due anelli non fusi conta come «dentro» (`aromatic.rs:152`)

`donated` conta 1 elettrone per un doppio legame verso un atomo `v` con `in_ring[v]`; deve valere solo se il legame **è un legame d'anello** (allora l'anello dell'atomo è fuso con quello di `v`; se il legame è in un anello `R'`, `R'` e l'anello testato condividono almeno il legame accanto, perché l'atomo sp² ha un solo vicino fuori dall'anello). Un doppio legame **ponte** tra due anelli non fusi è esociclico e, come con `=CH2`, deve dare `None`.

Esempio: bifluorenilidene-p-chinodimetano `C1(=C2C3=CC=CC=C3C3=CC=CC=C23)C=CC(=C2C3=CC=CC=C3C3=CC=CC=C23)C=C1`. Il corrispondente con `=CH2` (`C=C1C=CC(=C)C=C1`) dà 0/8 legami aromatici, quello con `=C` in un anello dà il ciclo centrale aromatico (4 elettroni dei due C=C interni + 1 + 1 contati per i due doppi legami ponte = 6): incoerenza della stessa regola.

| Variante | Legami aromatici sul chinodimetano | Cambi sul corpus di 812 SMILES |
|---|---|---|
| attuale (`in_ring[v]`) | 30/38 | – |
| Gemini (`ring_atoms.binary_search(&v)`) | 24/38 | **7**: chinodimetano 30→24, fenantrene K2 16→12 e 5 molecole vere (16→12, 16→11, 19→16, 24→21, 15→16): dipendenza dalla forma di Kekulé |
| **proposta: `ring_bond[k]`** (legame del doppio legame appartiene a un anello, calcolato in `Graph::new` da `rings()`) | 24/38 | **1**: solo il chinodimetano (30→24); fenantrene e tutto il resto invariati |

Prima→dopo: legami aromatici 30→24; `crippen_logp` 6,142→8,077 (i sei atomi del ciclo centrale passano da tipi aromatici a tipi alifatici). Nessun altro cambio nel corpus. `aromatic_bonds` non entra in SMILES, hash, certificato canonico, `mol/1`, `.mzlib`: tocca solo logP/MR/TPSA-HBA (via `Ctx`) e il flag di `grafo_record`. Decide Federico (è una regola di aromaticità nostra), ma il rischio è minimo: un caso su 812.

## Gruppo (c): da non fare

- Certificato binario o hash incrementale in `canon.rs` (2.3): cambia la foglia minima, quindi ordine canonico e hash di tutti i record già salvati (`CAN_VERSION` andrebbe alzata). Il guadagno è limitato a 1-33 foglie per molecola.
- `cls.iter().max()+1` al posto del `BTreeSet` (2.1) e `SmallVec` (nuova dipendenza).
- `c.nb[i].first()` per gli H espliciti (3.2): cambia logP.
- `vol*vol < 1e-12*scale³` al posto di `powf` (4.3b), `binary_search` su `centre/added` (8.2), salto di `renumbered` per `po` identità (8.3), `windows` su `bonds` non ordinati (9.1), spostamento del controllo `len` (5.1): o non bit-identici, o presuppongono invarianti che il tipo non impone, o cambiano messaggi d'errore, per guadagni sotto il microsecondo.
- Riscrivere `flips` con un `work_mol` (6.1): meno del 2 % di `stereo_sites`; il costo è nel canonico.

## Fuori dal report di Gemini (da valutare)

- `stereo_sites` di molecole simmetriche costa ms (cubano 10 ms, adamantano 1,65 ms, inositolo 2 ms) perché ogni prova canonizza un record con stereo definito; una cache per sito o un canonico a un solo livello potrebbero valere molto più dell'intera lista di Gemini. Non toccato in questa fase.
- `rotatable_bonds` (`descr.rs`, `in_ring` per legame) è O(B·(N+B)) con `vec` per chiamata: dopo i punti 1-2 diventa il costo principale di `descriptors()`.
