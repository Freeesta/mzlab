---
name: steward
description: Regole di mzLab per portare una PR propria a CI verde (rossi, conflitti, revisioni) e riferire al Coordinatore. Da leggere quando arrivano eventi su una PR aperta da questa chat.
---
# Portare una PR di mzLab a verde

La PR la unisce il Coordinatore (COO), mai la chat che l'ha scritta. La chat la porta a verde e lo dice una volta.

1. **CI rossa**: leggi solo la coda del log del job rosso (`controllo-github`), riproduci in locale, correggi, `python3 tools/pre_pr.py`, un push. Rosso anche su main o in una zona che la PR non tocca: una riga al COO (job, perché non è tuo) e nessun rilancio a vuoto. `e2e_tour` è instabile noto: un solo rilancio.
2. **Conflitto con main**: solo allora `tools/fondi_main.py` (merge, mai rebase o push forzato); i file generati si rigenerano con lo strumento, mai a mano. Una PR pulita non si aggiorna con main.
3. **Revisioni**: correggi le richieste piccole e locali; quelle grandi (API, formati, più file) vanno al COO con la tua proposta.
4. **Comunicazione**: niente commenti su GitHub; tutto passa al COO con `send_message`, nella sessione indicata dal prompt. A CI verde mandi UN messaggio: `PR #n · SHA (40 caratteri) · CI verde · cosa resta`. Un blocco vero è una riga con due opzioni, poi continui con altro.
5. **Mai**: unire, abilitare l'auto-merge senza che il prompt lo dica, push su main, saltare o mettere in quarantena un test, commit vuoti, chiedere a Federico.
6. **Dopo il merge**: la PR successiva parte da un ramo nuovo da `origin/main`. Se il contesto supera 200k token, manda il blocco di consegna (≤ 15 righe) e fermati.
