# Da dove viene questo ione? {kicker: Capitolo 17}

{.lead} Nel full scan uno ione che compare con il trattamento sembra un prodotto di trasformazione. Ma può essere un frammento del composto di partenza, rotto nella sorgente prima dell'analisi. Questo capitolo spiega come si ragiona e quali misure aiutano; la finestra «Da dove viene questo ione?» del programma le calcola, la conclusione la scrivi tu.

## Il problema: la frammentazione in sorgente

Gli ioni formati dall'elettrospray attraversano una regione a pressione intermedia dove un potenziale (*declustering potential*, DP, nel 3200 QTRAP) li accelera e li fa urtare con le molecole di gas. Se l'energia depositata è sufficiente, lo ione si rompe *prima* di Q1: è la **frammentazione in sorgente** (*in-source fragmentation*, ISF). Il frammento è uno ione vero, con il suo *m/z*, e Q1 lo trasmette come qualunque altro. Nello spettro non c'è nulla che lo distingua da un prodotto di trasformazione presente in soluzione.

Per un composto presente a concentrazioni elevate, come il precursore nei primi tempi di campionamento, gli ioni generati per ISF possono raggiungere intensità confrontabili con lo ione molecolare. Il fenomeno è ben documentato in metabolomica, dove porta a scambiare frammenti per metaboliti (Xu, Lu, Rabinowitz, *Anal. Chem.* 2015, 87, 2273-2281).

```html
<div class="box"><b>Rilevanza analitica</b><p>Se si classifica come &laquo;prodotto di trasformazione&raquo; uno ione che &egrave; solo un frammento generato in sorgente, la tabella dei TP conterr&agrave; un composto che in soluzione non esiste. Nella scala di confidenza di Schymanski et al. (<i>Environ. Sci. Technol.</i> 2014, 48, 2097-2098) un <i>m/z</i> senza altre evidenze resta a un livello basso: qui le evidenze si costruiscono con rigore metodologico.</p></div>
```

## Che cosa distingue un frammento in sorgente da un prodotto

### 1. Lo stesso profilo cromatografico

Il frammento nasce dopo la colonna, nello stesso istante dello ione da cui deriva. Il suo cromatogramma estratto (XIC) ha quindi la **stessa forma e lo stesso apice** di quello del progenitore, scansione per scansione. Un prodotto di trasformazione è un'altra molecola: di solito ha un'altra polarità e quindi un altro tempo di ritenzione. Si misura la correlazione dei profili e la differenza fra gli apici, con la sua incertezza.

### 2. Un rapporto costante: F contro P

Se in ogni scansione una frazione fissa *r* del progenitore si rompe, l'intensità del frammento F è *r* volte quella del progenitore P: nel grafico F contro P i punti stanno su una **retta per l'origine**. Il rapporto F/P dovrebbe essere lo stesso anche in campioni diversi, perché dipende dallo strumento (DP, temperatura della sorgente), non dalla chimica del campione. Attenzione: segnali molto intensi saturano il rivelatore e la retta si piega anche per un frammento vero.

### 3. Il comportamento nel tempo di trattamento

Un frammento segue il progenitore: scende con lui. Un intermedio di degradazione parte da zero, sale e poi scende (cinetica A → B → C, capitolo 2). È un indizio, non una regola: con pochi tempi molti andamenti sono compatibili con entrambe le ipotesi, e un prodotto che si forma e si consuma in modo proporzionale al progenitore può imitare un frammento.

### 4. Il DP e l'MS^2^

Alzando il DP un frammento in sorgente cresce a spese del progenitore; un prodotto in soluzione non cambia. È la prova più netta, ma richiede acquisizioni a più DP che non sempre ci sono. Un MS^2^ dello ione molecolare mostra gli stessi frammenti prodotti in sorgente: se il tuo ione compare nell'MS^2^ del candidato progenitore, è compatibile con essere un suo frammento. Se questi dati mancano, scrivilo tra i limiti.

## Un esempio svolto: l'atrazina

```html
<p>L'atrazina (C<sub>8</sub>H<sub>14</sub>ClN<sub>5</sub>, massa monoisotopica 215.0938) d&agrave; in ESI+ lo ione [M+H]<sup>+</sup> a <i>m/z</i> 216.1 (con l'isotopo del <sup>37</sup>Cl a 218.1). Una sua rottura molto comune &egrave; la perdita di propene (C<sub>3</sub>H<sub>6</sub>, 42.047 Da): 216.101 &minus; 42.047 = <b>174.054</b>. Lo stesso <i>m/z</i> 174 &egrave; per&ograve; anche lo ione [M+H]<sup>+</sup> della desisopropilatrazina (C<sub>5</sub>H<sub>8</sub>ClN<sub>5</sub>, 173.0468), un vero prodotto di degradazione. A risoluzione unitaria i due non si distinguono per massa: un picco a 174 in un campione di atrazina trattata pu&ograve; essere l'uno, l'altro, o entrambi.</p>
```

```html
<table class="t">
<thead><tr><th>Misura</th><th>174 frammento in sorgente</th><th>174 desisopropilatrazina</th></tr></thead>
<tbody>
<tr><td>Apice dell'XIC rispetto a 216</td><td>coincide (entro una scansione)</td><td>spostato: &egrave; pi&ugrave; polare, in fase inversa esce prima</td></tr>
<tr><td>F contro P (174 contro 216)</td><td>retta per l'origine</td><td>nuvola o curva, non proporzionale</td></tr>
<tr><td>F/P nei campioni</td><td>costante</td><td>cambia con il tempo di trattamento</td></tr>
<tr><td>Cinetica</td><td>scende con l'atrazina</td><td>parte da zero, sale, poi scende</td></tr>
</tbody></table>
```

Sono le attese *qualitative* di ciascuna ipotesi, non misure. Con i tuoi dati avrai quasi sempre una situazione mista (per esempio due componenti sovrapposte: la finestra te lo segnala con un avviso). In quel caso la risposta onesta è «non distinguibile con questi dati», e dire quali dati servirebbero (uno standard del solo composto di partenza a più DP, un MS^2^).

## Come si usa la finestra

1. Apri la finestra con il **clic destro** su un picco dello spettro, su un XIC o sulla mappa RT-*m/z* (voce «Da dove viene m/z ...?»), oppure con il pulsante nel pannello XIC.
1. Scegli tu il **candidato progenitore** (*m/z* del composto di partenza o di un altro ione più pesante) e, se vuoi, la finestra di tempo. Il programma non sceglie al posto tuo.
1. Leggi le quattro misure con le loro spiegazioni e gli **avvisi** (saturazione, picchi non risolti, pochi campioni, picco poco chiaro).
1. Aggiungi al grafico dei profili gli ioni co-eluenti della tabella che ti sembrano rilevanti: se un terzo ione ha lo stesso profilo, forse tutti e tre derivano dallo stesso progenitore.
1. Scrivi la tua **ipotesi**, con le evidenze a favore e contro, nel riquadro finale: resta nel taccuino. Esporta in Excel per allegare i numeri alla relazione.

## Come i numeri sono calcolati

I cromatogrammi estratti sono ripuliti dalla linea di base e levigati con un filtro di Savitzky-Golay; i picchi sono riconosciuti per trasformata wavelet (come nell'algoritmo *centWave*, Tautenhahn et al., *BMC Bioinformatics* 2008, 9, 504). La somiglianza dei profili è la correlazione di Pearson sulla finestra del picco, con un intervallo di confidenza ottenuto per ricampionamento a blocchi (le scansioni vicine non sono indipendenti). Il rapporto F/P è ricavato con una regressione che tiene conto dell'errore su entrambi gli assi. Tutte le soglie che si possono usare per decidere sono state tarate su dati sintetici e su pochi dati reali: sono un aiuto per la lettura, non un verdetto.

```html
<div class="box"><b>Limiti</b><p>A risoluzione unitaria un <i>m/z</i> &egrave; un candidato, non un'identificazione. Due componenti isobare con lo stesso tempo di ritenzione danno un picco unico che nessuna di queste misure separa. I tre strumenti pi&ugrave; forti (variare il DP, l'MS<sup>2</sup> del progenitore, lo standard del solo composto di partenza) richiedono acquisizioni in pi&ugrave; rispetto al full scan di routine.</p></div>
```

## Domande per riflettere

- Il tuo ione ha l'apice *esattamente* sullo stesso scan del progenitore. Basta per dire che è un frammento? Quale prodotto di trasformazione potrebbe comportarsi così?
- Il rapporto F/P è costante nei campioni ma il segnale del progenitore è saturato. Come cambia il peso di questa evidenza?
- Che cosa ti aspetteresti di vedere nell'MS^2^ del progenitore se il tuo ione fosse un suo frammento?
