# Spettro: tabella picchi e asse in %

- **Tabella picchi** (`tabella-picchi.js`, `window.TABPICCHI`; pulsante «Tabella picchi» in ogni pannello spettro, agganciato da `pannelli.js`, aggiornato da `drawSpec`; prova `e2e_tabella_picchi`): popover con i picchi che il grafico disegna (`p._a.data[0].d`, intensità non in %), le righe dei dati dello spettro e i comandi.
- **Comandi**: «Soglia min. %» (2, passi 0,5), «Max righe» (50, i più intensi), ordine m/z crescente o intensità decrescente, «Solo picchi etichettati» (`p._a.lbls`). In MS² due colonne in più: Δm/z dal precursore e perdita neutra (precursore − m/z).
- **Uscita**: solo da copiare. «Copia» scrive sugli appunti TSV con riga di intestazione e una tabella html; Ctrl/Cmd+C con la tabella selezionata dà lo stesso TSV; «Copiato» per 2 s. Nessun file scaricato. Numeri con `I18N.num` (sempre il punto, anche in italiano).
- **Asse y in %**: finisce a 100 (`tickMax` in `axes`, grafici.js); il margine sopra il picco base resta nel grafico per le etichette. `p._a.yticks` elenca i valori dell'asse.
- **Pulsante XIC**: solo sugli spettri MS¹ (`ms2` in `xic-spettro.js`); su MS² mancano anche la scelta del picco e i tasti `,` `.`.
