# Memoria: regole di scelta, esperienza d'uso e proposte aperte
Seguito di `memoria.md` (contratto, budget, lastre, prove) e `compat-safari.md` («Memoria su WebKit»): perché il budget è fatto così, che cosa vede l'utente quando la memoria non basta, che cosa resta da costruire.

## Regole
- Un rifiuto per memoria si guarda prima a monte: troppe feature di rumore (soglia fissa sotto il rumore dei dati) pesano più di qualunque tetto; la soglia stimata dai dati (`params::noise_from_hist`) sta in `Recipe::for_index`, così sito, `mzlab-cli` e i passi del Flow hanno la stessa ricetta.
- Il budget più grande cambia solo la via (memoria o lastre), mai i byte del risultato; ogni fase stima prima di allocare. Il tetto di costruzione di 4 GiB non riserva memoria.
- Rust decide budget e discesa (`budget_auto`, `budget_down`) dai fatti del dispositivo; JS raccoglie i fatti (`deviceMemory`, core, telefono, iOS, prova di crescita) e scrive il registro, senza soglie in MB scritte a mano.
- La prova di crescita gira in un'istanza a parte (`rust-sonda.js`), perché la memoria WASM non si restringe: una prova dentro il motore gli lascerebbe gigabyte. Misura il limite del browser, non la RAM libera; oltre 2 s vale la tabella dei livelli.
- `measureUserAgentSpecificMemory` non si usa: chiede l'isolamento cross-origin che GitHub Pages non dà.
- Non conviene lo spill della tabella allineata (letta a caso da `finish` e `rowmeta`) né della cache del motore (si rifà dall'indice).
- Sullo stesso contratto girano i passi del Flow sul sito, nell'app e in `mzlab-cli flow run` (nativo: budget = RAM/4): stessa stima per passo, stessa via, stessi byte.

## Esperienza d'uso
- Predefinito sicuro: nessuna domanda finché la stima entra nel budget sicuro; le lastre sono silenziose (stessi byte), si vede solo l'avanzamento.
- Avviso prima di partire, una volta per corsa, quando la stima supera il budget sicuro ma entra in quello alto (`memoria-avviso.js`): il pulsante sicuro è il predefinito (Invio, Esc, clic fuori), «Use more memory» è la scelta dell'utente. Oltre il budget alto il rifiuto porta `libero_mb` (l'eccesso è contro il libero, non contro il tetto) e il consiglio «Try fewer files at once.».
- Dopo una morte del motore: una riga nello stato, non una finestra («The engine ran out of memory…; finished steps are saved; it restarts with less memory»), le fasi finite restano dalla cache; la discesa vale per la sessione.
- Mai errori muti: un passo fallito mostra chiave, numeri e consiglio.

## Proposte non realizzate
- Proposta non realizzata: un solo «permesso grande» per origine, Web Lock `mzlab-mem-grande` (esclusivo anche fra schede), per un worker che usa più del budget sicuro; chi l'ha usato chiude il proprio worker a fine passo se supera 512 MB.
- Proposta non realizzata: terzo pulsante «Work in parts (slower)» nell'avviso, dove il passo ha le lastre, e «≈ X MB · in memory/in parts» con «part N of M» su ogni passo della tela prima e durante la corsa.
- Proposta non realizzata: `flow::stima_passo(job, ingressi) -> {mb, via}` unica per CLI, desktop e sito, con `mem_mb` e `via` per passo in `prov/1` e `corsa/1`.
- Proposta non realizzata: `Stored` e `Spill` del runner web su file OPFS letti a intervalli con `createSyncAccessHandle` (heap del worker durante `align` ≤ una lastra + un file), invece di copiare tutti i `.mzfeatf` nell'heap del worker.
- Proposta non realizzata: ripiego `--import-memory` (la pagina crea la `WebAssembly.Memory` col massimo scelto, stesso `.wasm`) se un browser non istanzia un modulo con massimo 4 GiB.
