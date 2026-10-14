# Accessibilità della vista Dati: lacune note
Obiettivo WCAG 2.2 AA; lettore di schermo di riferimento VoiceOver (macOS/iPad), secondo controllo NVDA (Windows). Il test `e2e_a11y_dati` (giro `--fumo`) fallisce su ogni violazione axe `serious`/`critical` negli stati che copre; un id regola tollerato si mette in `KNOWN` dentro il test e si elenca qui.
- **Regole tollerate**: nessuna.
- **Stati non coperti da axe**: mondo HR (Banco, formule, albero MSn), Disegno, Pratica, finestre di integrazione e calcolatrice, tour guidato, app desktop.
- **Livelli non controllati**: violazioni axe `moderate`/`minor`; l'ordine di lettura e le etichette dei grafici (`TDATI` dà l'alternativa testuale ma nessuno ha provato un lettore di schermo).
- **Tastiera**: il test verifica che Tab raggiunga almeno il 90 % dei comandi visibili e che ognuno mostri un indicatore del fuoco; non cerca trappole del fuoco dentro i grafici né l'ordine di Tab.
- **Contrasto**: le legende con voce spenta (`.leg .tg.off`, opacità .38) non sono coperte dal test finché nessuno le spegne; vanno controllate se il test le spegne.
