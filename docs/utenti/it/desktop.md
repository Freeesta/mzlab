# Installare l'app desktop

L'app desktop mostra la stessa pagina del sito in una finestra, con un motore di calcolo nativo che apre file grandi più in fretta. **Non è necessaria**: il sito funziona da solo. I file restano sul tuo computer, come nel sito.

## Dove scaricarla
Dalla pagina [Releases](https://github.com/Freeesta/mzlab/releases) del repository. Se non c'è ancora nessuna release, usa il sito. Sono previsti: macOS (`.dmg`, Apple Silicon e Intel) e Windows (`.exe`); Linux (AppImage e `.deb`) quando sarà pubblicato.

## Installare
- **Windows**: apri il file `.exe` e segui la procedura. Se compare «Windows ha protetto il PC» (SmartScreen), scegli **Ulteriori informazioni → Esegui comunque**, solo se il file viene dalla pagina Releases del repository ufficiale.
- **macOS**: apri il `.dmg` e trascina mzlab in Applicazioni. Se l'app non è firmata, al primo avvio tieni premuto Ctrl e clicca sull'icona, poi **Apri → Apri**; in alternativa Impostazioni di Sistema → Privacy e sicurezza → **Apri comunque**.
- Controlla che il file corrisponda al `SHA256SUMS` della release, se presente.

## Aggiornare
Nelle impostazioni (ingranaggio) c'è **«Controlla aggiornamenti»**: scarica solo i file della pagina cambiati, dal sito ufficiale `https://freeesta.github.io/mzlab/`, e ne controlla l'impronta SHA-256. Se serve un'app nuova (parte nativa), il messaggio rimanda alla pagina Releases. Durante le lezioni vale il [congelamento](docenti.md#congelamento-della-versione-durante-le-lezioni).

## Problemi
Se l'app non parte, prova il sito nel browser e segnala il problema con una [issue](https://github.com/Freeesta/mzlab/issues/new/choose) (sistema operativo e versione, nessun dato di laboratorio). Supporto *best effort*.
