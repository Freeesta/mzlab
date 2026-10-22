# Prove delle regole di frammentazione (`mzlab-cli msn-rules`)

Solo numeri e criteri; nessuno spettro. Comando: `python3 tools/convalida_regole.py` (MassBank 2026.03, Zenodo 19590880, record CC0/CC BY con SMILES; dettagli in `msn-interpreta.md`). Ogni record diventa un grafo a un livello (`grafo_da_picchi`) con la sua formula e il suo SMILES; i picchi frammento escludono quello del precursore.

## Criteri
- Strati separati per attivazione e polarità: `cid_trappola` (CID su trappola), `hcd`, `qtof` (CID su TOF, letto come fascio), `fascio` (CID su altri strumenti, letto come fascio). Mai mescolati.
- Copertura (a) picchi con una sottoformula; (b) più regole della sola formula; (c) più regole della struttura. Intensità in % del picco base. I record con (a) < 0,5 (massa nominale) sono contati a parte (`nominali`) e restano fuori dalla prova a 10 ppm.
- Decoy deterministici (seme per record = FNV-1a dell'id): `ds` legami della struttura rimescolati con scambi che conservano l'ordine (stessa formula, stessi gruppi); `dp` spettro di un altro record con stesso precursore nominale e polarità e formula neutra diversa (lo stesso composto a un'altra energia o un isomero non vale); `dl` perdite spostate di −0,0137 Da.
- La copertura (c) supera la prova se, per ogni decoy applicabile, (c − decoy) ≥ 0,5·c. Una regola con meno di 20 applicazioni vere è «non misurata». Il tasso di una regola è il massimo, sui decoy applicabili, di applicazioni decoy / applicazioni vere; `ds` si applica solo alle regole con `site` o `kind` cleavage (una regola che chiede un gruppo o un elemento lo passa per costruzione). Sopra 0,30 in uno strato con almeno 20 applicazioni la regola scende a priorità 3 in `fragrules-1.json`, con il suo perché.
- Le regole della sola formula valgono per addotti rari ([M+Na]+, [M+NH4]+, [M+Cl]-, [M+HCOO]-) e la copertura (b) resta 0 negli spettri [M+H]+/[M-H]-.

## Registri e scarti
Scartati: attivazione non letta 10897, ione non supportato 250, SMILES non leggibile 693, spettro vuoto 375.

| strato | pol | registri | nominali | (a) | (c) | c−ds | c−dp | c−dl | prova |
|---|---|---|---|---|---|---|---|---|---|
| cid_trappola | − | 333 | 19 | 0.995 | 0.217 | 0.129 | 0.218 | 0.217 | sì |
| cid_trappola | + | 2514 | 89 | 0.998 | 0.283 | 0.168 | 0.284 | 0.283 | sì |
| fascio | − | 35 | 137 | 0.757 | 0 | 0 | 0 | 0 | sì |
| fascio | + | 42 | 326 | 0.845 | 0.049 | 0.034 | 0.049 | 0.049 | sì |
| hcd | − | 5742 | 705 | 0.994 | 0.084 | 0.048 | 0.086 | 0.084 | sì |
| hcd | + | 17562 | 1609 | 0.993 | 0.159 | 0.119 | 0.158 | 0.159 | sì |
| qtof | − | 1618 | 405 | 0.952 | 0.079 | 0.044 | 0.074 | 0.079 | sì |
| qtof | + | 5058 | 949 | 0.938 | 0.17 | 0.113 | 0.169 | 0.17 | sì |

La prova passa in ogni strato; nello strato `fascio` (c) vale al più 0,05, quindi la prova è quasi vuota. Dove (c) è alta (CID su trappola, 0,22-0,28) i decoy `dp` e `dl` la azzerano e `ds` la riduce a circa 0,4.

## Regole declassate a priorità 3

| regola | strato, pol, applicazioni vere, tasso |
|---|---|
| `ion_tropylium` | hcd + 130 0.454 |
| `loss_nh3_amide` | cid_trappola + 30 0.4; hcd + 133 0.361; qtof + 29 0.483 |

Le coppie composte («a+b») seguono le regole che le compongono. Le altre regole hanno tasso ≤ 0,30 o meno di 20 applicazioni (non misurate): restano alla priorità della tabella, senza prova a favore.
