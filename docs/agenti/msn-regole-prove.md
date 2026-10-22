# Prove delle regole di frammentazione (`mzlab-cli msn-rules`)

Solo numeri e criteri; nessuno spettro. Comando: `python3 tools/convalida_regole.py` (MassBank 2026.03, Zenodo 19590880, record CC0/CC BY con SMILES; dettagli in `msn-interpreta.md`). Ogni record diventa un grafo a un livello (`grafo_da_picchi`) con la sua formula e il suo SMILES; i picchi frammento escludono quello del precursore.

## Criteri
- Strati separati per attivazione e polarità: `cid_trappola` (CID su trappola), `hcd`, `qtof` (CID su TOF, letto come fascio), `fascio` (CID su altri strumenti, letto come fascio). Mai mescolati.
- Copertura (a) picchi con una sottoformula; (b) più regole della sola formula; (c) più regole della struttura. Intensità in % del picco base.
- Decoy deterministici (seme per record = FNV-1a dell'id): `ds` legami della struttura rimescolati con scambi che conservano l'ordine (stessa formula, stessi gruppi); `dp` spettro di un altro record con stesso precursore nominale e polarità; `dl` perdite spostate di −0,0137 Da.
- La copertura (c) supera la prova se è almeno 0,20 sopra ogni decoy. Una regola con meno di 20 applicazioni vere è «non misurata». Il tasso di una regola è il massimo, sui decoy applicabili (`ds` solo per le regole che leggono la struttura), di applicazioni decoy / applicazioni vere; sopra 0,30 in uno strato con almeno 20 applicazioni la regola scende a priorità 3 in `fragrules-1.json`, con il suo perché.
- Le regole della sola formula valgono per addotti rari ([M+Na]+, [M+NH4]+, [M+Cl]-, [M+HCOO]-) e la copertura (b) resta 0 negli spettri [M+H]+/[M-H]-.

## Registri e scarti
Registri usati per strato e polarità; scartati: attivazione non letta 10897, ione non supportato 250, SMILES non leggibile 693, spettro vuoto 375.

| strato | pol | registri | (a) | (c) | c−ds | c−dp | c−dl | prova |
|---|---|---|---|---|---|---|---|---|
| cid_trappola | − | 352 | 0.985 | 0.214 | 0.127 | 0.182 | 0.214 | no |
| cid_trappola | + | 2603 | 0.986 | 0.279 | 0.166 | 0.248 | 0.279 | no |
| fascio | − | 172 | 0.225 | 0 | 0 | 0 | 0 | no |
| fascio | + | 368 | 0.153 | 0.006 | 0.004 | -0.022 | 0.005 | no |
| hcd | − | 6447 | 0.982 | 0.083 | 0.047 | 0.065 | 0.083 | no |
| hcd | + | 19171 | 0.986 | 0.158 | 0.118 | 0.134 | 0.157 | no |
| qtof | − | 2023 | 0.867 | 0.072 | 0.04 | 0.04 | 0.072 | no |
| qtof | + | 6007 | 0.879 | 0.158 | 0.106 | 0.115 | 0.158 | no |

La copertura (c) non passa in nessuno strato: ds conserva i gruppi, quindi le regole che chiedono solo un gruppo si attivano anche sulla struttura rimescolata; dp e dl separano meglio (c−dp 0,13–0,25 nei CID e HCD). Lo strato `fascio` ha troppo pochi registri per dire altro.

## Regole declassate a priorità 3

| regola | strato, pol, applicazioni vere, tasso |
|---|---|
| `ion_tropylium` | hcd + 130 0.454 |
| `loss_c2h3no_n_methylcarbamate` | qtof + 26 0.462 |
| `loss_ch2o2_acid_pos` | cid_trappola + 38 0.553 |
| `loss_ch5n_n_methyl` | qtof + 35 0.343 |
| `loss_co2_carboxylate` | qtof − 138 0.413 |
| `loss_h2o_acid_pos` | cid_trappola + 101 0.366; qtof + 219 0.374 |
| `loss_h2o_alcohol` | cid_trappola + 444 0.869; hcd − 111 1.18; hcd + 850 0.92; qtof − 26 1.192; qtof + 409 1.015 |
| `loss_hcl` | cid_trappola + 45 1; hcd − 159 1; hcd + 167 1; qtof − 35 1; qtof + 63 1 |
| `loss_hcn_heteroaromatic` | hcd + 40 1 |
| `loss_hexose` | qtof − 22 0.364 |
| `loss_hf` | hcd − 41 1; hcd + 69 1; qtof − 21 1; qtof + 34 1 |
| `loss_nh3_amide` | cid_trappola + 30 0.4; hcd + 133 0.361; qtof + 30 0.467 |
| `loss_nh3_amine` | cid_trappola + 90 1.378; hcd + 420 1.214; qtof + 130 1.023 |
| `loss_so2_sulfonamide` | hcd − 25 0.64; hcd + 21 0.81 |

Le coppie composte («a+b») seguono le regole che le compongono. Le regole né declassate né citate hanno tasso ≤ 0,30 o meno di 20 applicazioni (non misurate): restano alla priorità della tabella, senza prova a favore.
