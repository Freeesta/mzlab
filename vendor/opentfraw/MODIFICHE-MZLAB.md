# opentfraw 2.0.0 modificato per mzLab

Copia di OpenTFRaw 2.0.0 (Nathan Riley / Sigilweaver, Apache-2.0, https://github.com/Sigilweaver/OpenTFRaw) con modifiche di mzLab, come richiede la licenza.

- `src/reader.rs`, `src/scan_event.rs`: la tabella degli eventi di scansione dei file v66 con eventi di dimensione variabile (Orbitrap Tribrid) ha tre tipi di evento — primario (MS1), dipendente e MS2+ indipendente (finestre DIA, 288 byte) — e le dimensioni si trovano percorrendo la tabella con ciascuno schema noto finché termina esattamente dove comincia il flusso successivo (`walk_events`). Prima gli eventi DIA sfasavano tutta la tabella: livello, polarità, filtro e punti di quasi metà delle scansioni erano sbagliati.
- `src/scan_event.rs`: i coefficienti di calibrazione dell'evento si cercano subito dopo la finestra di scansione (numero di parametri a `finestra + 16` quando plausibile), non solo a `64 → 80` o `corpo − 64`: gli eventi MS1 dei Tribrid (finestra a 8, numero a 24) restavano senza coefficienti e il profilo usciva con la frequenza al posto del m/z.
