# Struttura come dato: le prove

Numeri dietro `STRUTTURA_DATO.md`. Tutto **[verificato]** con i comandi di §5, salvo dove è scritto **[ipotesi]**. Strumenti di riferimento: RDKit 2026.09.1 e Open Babel 3.1.0 (pip, solo in `tools/`), OpenChemLib della copia in `mzlab/web/vendor/`, InChI 1.07.5 compilato in wasm qui.

## 1. Insieme di prova (20 molecole) e cosa dicono i formati
Gli indici di «Stereo» e CIP sono quelli di RDKit sullo SMILES scritto. «OB» = lo SMILES canonico di Open Babel è uguale a quello di RDKit?
| Nome | SMILES (CXSMILES di RDKit) | OB | CIP | Stereo (stato) | InChIKey |
|---|---|---|---|---|---|
| timolol_S | `CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1` | diverso | 6S | tetra6 definita | BLJRIMJGRPQVNF-JTQLQIEISA-N |
| timolol_R | `CC(C)(C)NC[C@@H](O)COc1nsnc1N1CCOCC1` | diverso | 6R | tetra6 definita | BLJRIMJGRPQVNF-SNVBAGLBSA-N |
| timolol_unspec (come nel repository) | `CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1` | diverso | 6? | tetra6 **sconosciuta** | BLJRIMJGRPQVNF-UHFFFAOYSA-N |
| timolol_rac | `CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1 \|&1:6\|` | diverso | 6S | tetra6 definita, gruppo `and` | BLJRIMJGRPQVNF-**JTQLQIEISA**-N (= S) |
| timolol_maleate | `…COc1nsnc1N1CCOCC1.O=C(O)/C=C\C(=O)O` | diverso | 6S | tetra6, db23 definite | WLRMANUAADYWEA-NWASOUNVSA-N |
| timolol_d9 | `[2H]C([2H])([2H])C(NC[C@H](O)…)(C([2H])([2H])[2H])C([2H])([2H])[2H]` | diverso | 7S | tetra7 definita | BLJRIMJGRPQVNF-QLLHHHRWSA-N |
| ephedrine (1R,2S) | `CN[C@@H](C)[C@H](O)c1ccccc1` | diverso | 2S 4R | due definite | KWGRBVOPPLSCSI-WPRPVWTQSA-N |
| pseudoephedrine (1S,2S) | `CN[C@@H](C)[C@@H](O)c1ccccc1` | diverso | 2S 4S | due definite | KWGRBVOPPLSCSI-WCBMZHEXSA-N |
| cis-1,2-dimetilcicloesano (meso) | `C[C@@H]1CCCC[C@@H]1C` | = | 1R 6S | due definite | KVZJLSYJROEPSQ-OCAPTIKFSA-N |
| trans relativo, racemo | `C[C@H]1CCCC[C@@H]1C \|&1:1,6\|` | = | 1R 6R | due definite, gruppo `and` | KVZJLSYJROEPSQ-HTQZYQBOSA-N |
| acido maleico (Z) | `O=C(O)/C=C\C(=O)O` | diverso | – | db3 definita | VZCYOOQTPOCHFL-UPHRSURJSA-N |
| acido fumarico (E) | `O=C(O)/C=C/C(=O)O` | diverso | – | db3 definita | VZCYOOQTPOCHFL-OWOJBTEDSA-N |
| but-2-ene non specificato | `CC=CC` | = | – | db1 **sconosciuta** | IAQRGUVFOMOMEM-UHFFFAOYSA-N |
| 2-idrossipiridina | `Oc1ccccn1` | = | – | nessuna | UBQKCCHYAOITMY-UHFFFAOYSA-N |
| 2-piridone | `O=c1cccc[nH]1` | = | – | nessuna | UBQKCCHYAOITMY-UHFFFAOYSA-N (= sopra) |
| acetone | `CC(C)=O` | diverso | – | nessuna | CSCPPACGZOOCGX-UHFFFAOYSA-N |
| propen-2-olo (enolo) | `C=C(C)O` | diverso | – | nessuna | NARVIWMVBMUEOG-UHFFFAOYSA-N (≠ acetone) |
| diclofenac | `O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl` | diverso | – | nessuna | DCOPUUMXTXDBNB-UHFFFAOYSA-N |
| diclofenac sodico | `O=C([O-])Cc1ccccc1Nc1c(Cl)cccc1Cl.[Na+]` | diverso | – | nessuna | KPHWPUGNDIVLNH-UHFFFAOYSA-M |
| esano | `CCCCCC` | = | – | nessuna | VLKZOEOYAKHREP-UHFFFAOYSA-N |

Letture:
- **SMILES**: RDKit e Open Babel scrivono diverso 14 volte su 20. `timolol_unspec` ed `esano` non hanno segni di stereo, ma il primo ha un centro sconosciuto e il secondo nessun centro: la stringa non lo dice.
- **InChIKey**: il secondo blocco separa S, R, non specificato (`UHFFFAOYSA`) e deuterato; separa i due diastereoisomeri e Z/E. Unisce 2-idrossipiridina e 2-piridone (H mobile), non acetone ed enolo. **Racemo = puro S** (stessa chiave): l'InChI standard ignora i gruppi `&`/`o`; anche il trans relativo esce come assoluto (`/m1/s1`), mentre il meso esce senza `/m` `/s` (corretto).
- **InChI di RDKit e di Open Babel**: uguali in 20 casi su 20; e uguali al wasm compilato qui (§3).
- **Sali**: la chiave del sale è un'altra molecola (maleato: un altro primo blocco; sodio: suffisso `-M`). Il nostro lettore rifiuta il `.` [letto, `grafo.rs`].

## 2. Molfile e coordinate
| Prova | Risultato |
|---|---|
| V2000 2D con cunei (RDKit → RDKit) | 18 su 18 uguali; i due con gruppi `&` RDKit li scrive in V3000 (`MDLV30/STERAC1`) e tornano uguali |
| V2000 2D senza cunei (campo stereo dei legami a 0) | stereo dei centri **persa** in 7 casi su 7; Z/E dei doppi legami tenuta dalle coordinate (maleico, fumarico) |
| but-2-ene non specificato, senza flag «either» | la Z/E viene **inventata** dalle coordinate 2D (non torna uguale) |
| Il nostro `from_molfile` | ignora il campo stereo, `M  ISO`, rifiuta gli H espliciti [letto, `grafo.rs` r. 37-95] |

## 3. InChI in wasm
- Libreria IUPAC `v1.07.5` (MIT), `zig cc -target wasm32-wasi -Oz`, esportate solo `MakeINCHIFromMolfileText` e `GetINCHIKeyFromINCHI` (`tools/struttura_dato/inchi_wasm/shim.c`): **689 873 B**, gzip -9 **286 401 B**. Importa 6 funzioni WASI. Tempo di compilazione ~1 min.
- Sui 20 molfile di §2: 20 InChIKey uguali a RDKit. `AuxInfo=1/…/N:` dà, per ogni atomo canonico di InChI, il numero dell'atomo nel molfile: un ponte già pronto tra i nostri indici e l'ordine di InChI.
- Tetti: non entra in `chem` (18 KB liberi), né in `grafo` (512 000), né in `interp`. Serve un modulo a parte con un tetto nuovo [ipotesi: ~700 000 B; con `-O2` o senza le parti non standard potrebbe scendere, non provato].

## 4. OpenChemLib e l'ordine degli atomi (trappola di `TPMOL`)
`TPMOL.graph` prende le coordinate da `OCL.Molecule.fromSmiles` e presuppone che l'atomo `i` di OpenChemLib sia l'atomo `i` dello SMILES (l'ordine di `interp/1`).
| SMILES | Ordine tenuto |
|---|---|
| timololo S, scritto in due modi | sì (e stesso idcode nei due modi) |
| fenolo scritto con `[H]O…` | **no**: gli atomi pesanti cambiano posto (`C,O,C…` invece di `O,C,C…`) |
| `C[C@]([H])(O)CC` | **no** (`C,C,C,O,C` invece di `C,C,O,C,C`) |
| deuterato, sale con `.`, anello che si apre per primo | sì |
Il motore Rust rifiuta già `[H]` (`error.grafo.element`), quindi oggi la trappola non scatta; con il lettore nuovo (che accetta `[H]`) va chiusa: le coordinate si chiedono per indice canonico, non per ordine di scrittura [ipotesi di soluzione]. Lo SMILES canonico di OpenChemLib è un'altra forma ancora (`n[s]n`, ordine diverso da RDKit).

## 5. Andata e ritorno del record `mol/1`
`confronto.py` costruisce il record da RDKit (atomi in ordine canonico di RDKit, Kekulé, stereo con `refs` ordinati e parità, gruppi), lo serializza in JSON, lo rilegge in una molecola **senza leggere nessuno SMILES**, e confronta.
- CXSMILES (stereo, gruppi `&`, isotopi, cariche): **20 su 20** uguali.
- InChI: **20 su 20** uguali.
- Scoperto strada facendo: per Z/E il record deve dire cis/trans **rispetto a due atomi di riferimento nominati** (non E/Z: le etichette CIP cambiano con i sostituenti); per un centro tetraedrico la parità va riscritta sull'ordine dei riferimenti del record, non su quello dei vicini come li tiene il toolkit.
- Limite della prova: l'ordine canonico qui è quello di RDKit. Il nostro canonico va scritto e provato (P2 di `STRUTTURA_DATO.md`) [ipotesi].

## 6. Lista dei sospetti di Tabui (`TP_Mine/data/composti.csv`)
625 righe: InChIKey coerente con lo SMILES in 625 (0 solo scheletro, 0 diverse); 286 con elementi stereo, 111 con almeno uno non specificato; nessun `.`. Oggi il motore scarta la stereo di tutte e 286.

## 7. Comandi
```
pip install rdkit openbabel-wheel ziglang
python3 tools/struttura_dato/confronto.py --json /tmp/x/conf.json --molfiles /tmp/x/mol
python3 tools/struttura_dato/confronto.py --composti TP_Mine/data/composti.csv
node tools/struttura_dato/ocl_ordine.mjs
bash tools/struttura_dato/inchi_wasm/build.sh /tmp/x/inchi /tmp/x/mol
```
