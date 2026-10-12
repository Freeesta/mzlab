# Accessibilità

mzLab punta alle regole **WCAG 2.2 livello AA** per Dati, Disegno, Teoria e Pratica. Non tutto è garantito: l'editor chimico (Ketcher) è un componente di terzi e ha i suoi limiti.

## Che cosa è previsto
- **Tastiera**: i comandi si raggiungono da tastiera; i pulsanti hanno un nome per i lettori di schermo (`aria-label`). Nei grafici, zoom e spostamento funzionano anche senza trascinare (pulsanti e scorciatoie; Backspace torna alla vista intera).
- **Colori**: solo i colori del tema, tema chiaro e scuro, contrasto del testo controllato nei due temi; i grafici usano una tavolozza leggibile anche da chi ha una visione dei colori alterata (Okabe-Ito).
- **Ogni grafico ha la sua tabella dei dati**, per chi non può leggere il disegno.
- **Bersagli**: i comandi misurano almeno 24×24 px (40 px su schermo tattile).
- **Movimento**: se il sistema chiede di ridurre le animazioni, le animazioni si riducono.
- **Testo**: la dimensione si regola dalle impostazioni (ingranaggio); l'interfaccia è in italiano e inglese (Teoria e Pratica solo in italiano).

## Provato come
Controlli automatici (axe) sui grafici e sulle schermate principali. Non c'è ancora una verifica con persone che usano lettori di schermo.

## Se trovi una barriera
Segnalala con una [issue](https://github.com/Freeesta/mzlab/issues/new/choose): descrivi il browser, il dispositivo e l'ausilio che usi. Supporto *best effort*, senza tempi garantiti. Se non riesci a usare una parte del programma durante una lezione, chiedi al docente un'alternativa.
