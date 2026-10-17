# Modulo 2 — Esercizi Guidati Passo-Passo

Questo modulo presenta tre esercitazioni pratiche complete su composti chimici
di interesse pubblico e ambientale. Ogni esercizio è strutturato con obiettivi
formativi, formule teoriche, passaggi operativi in mzLab e relative descrizioni
degli screenshot dell'interfaccia.

---

## Esercizio 1: Determinazione di Massa Esatta, Addotti e Pattern Isotopico del Paracetamolo ($C_8H_9NO_2$)

### Obiettivi Didattici
1. Calcolare la massa monoisotopica teorica esatta del paracetamolo.
2. Identificare i picchi relativi agli addotti protonato $[M+H]^+$ e sodiato
   $[M+Na]^+$ nello spettro ESI in polarità positiva.
3. Utilizzare il righello $\Delta m/z$ per convalidare lo scambio $Na^+ / H^+$.
4. Sovrapporre il pattern isotopico teorico ($M, M+1, M+2$) e calcolare l'errore
   sperimentale in parti per milione (ppm).

---

### Passo 1.1: Calcolo Teorico della Massa Monoisotopica
Il paracetamolo (4-acetammidofenolo) ha formula molecolare neutra $C_8H_9NO_2$.
Utilizzando le masse atomiche monoisotopiche accurate IUPAC:
- $^{12}C = 12.000000\text{ Da}$
- $^{1}H = 1.007825\text{ Da}$
- $^{14}N = 14.003074\text{ Da}$
- $^{16}O = 15.994915\text{ Da}$

$$\text{Massa neutra monoisotopica } M = (8 \times 12.000000) + (9 \times 1.007825) + (1 \times 14.003074) + (2 \times 15.994915)$$
$$M = 96.000000 + 9.070425 + 14.003074 + 31.989830 = 151.063329\text{ Da}$$

In elettrospray positivo ($ESI^+$), la specie principale è lo ione
pseudomolecolare protonato $[M+H]^+$. Considerando la massa del protone
($H^+ = 1.007276\text{ Da}$):
$$m/z\ [M+H]^+ = 151.063329 + 1.007276 = 152.070605 \approx 152.0706$$

In presenza di tracce di sodio (vetreria, solventi), si osserva frequentemente
l'addotto sodiato $[M+Na]^+$ ($^{23}Na^+ = 22.989218\text{ Da}$):
$$m/z\ [M+Na]^+ = 151.063329 + 22.989218 = 174.052547 \approx 174.0526$$

---

### Passo 1.2: Individuazione degli Addotti e Misura con il Righello
1. Caricare in mzLab il tracciato del campione contenente paracetamolo
   (oppure selezionare il file di esempio ad alta risoluzione).
2. Spostare il cursore temporale sul cromatogramma in corrispondenza del picco
   eluito a $t_R = 3.45\text{ min}$.

[Screenshot: Cromatogramma TIC nel pannello superiore con cursore verticale
blu posizionato a 3.45 min sull'apice di un picco ben risolto. Nel pannello
inferiore compare lo spettro di massa ad alta risoluzione (intervallo m/z
100 - 250): il picco base a m/z 152.0709 svetta al 100% di intensità, mentre
a m/z 174.0528 è visibile un picco secondario di intensità pari a circa il
12% del picco base.]

3. Fare clic sull'icona del **Righello** (o clic destro sul picco a $m/z\ 152.0709$
   $\rightarrow$ «Misura da questo picco»). Il picco a 152.0709 viene contrassegnato
   con la linea tratteggiata «rif.».
4. Fare clic sul picco a $m/z\ 174.0528$.

[Screenshot: Spettro di massa con il righello attivo tra i due picchi. Sopra
i picchi compare una barra orizzontale delimitata da due marcatori verticali.
Al centro della barra compare un riquadro azzurro con la scritta automatica:
«Na⁺/H⁺ · Δm/z 21.9819 (21.98 Da)». L'etichetta conferma che la distanza di
massa tra i due segnali corrisponde con precisione millesimale allo scambio
protone/sodio.]

---

### Passo 1.3: Simulazione del Pattern Isotopico
1. Fare clic destro sul picco principale a $m/z\ 152.0709$.
2. Selezionare **«Profilo isotopico di una formula…»**.
3. Nella casella di testo digitare la formula con il relativo addotto:
   `C8H9NO2 [M+H]+` e premere Invio.

[Screenshot: Ingrandimento dello spettro di massa nella regione m/z 151.5 -
155.0. Sul picco sperimentale a m/z 152.0709 è sovrapposto un cerchietto rosso
pieno centrato sull'apice (M, 100%). A m/z 153.0741 è visibile il picco M+1
con il cerchietto teorico al 9.2%. A m/z 154.0758 è visibile il piccolo picco
M+2 con il cerchietto teorico allo 0.8%. L'altezza e la posizione dei cerchietti
rossi coincidono perfettamente con i picchi sperimentali.]

Spiegazione chimica delle abbondanze isotopiche:
- **Picco $M+1$ ($9.2\%$ rispetto ad $M$):** dominato dal contributo di $^{13}C$
  (8 atomi di carbonio $\times 1.08\% \approx 8.64\%$), a cui si sommano i
  piccoli contributi di $^{15}N$ ($0.37\%$), $^{2}H$ ($0.13\%$) e $^{17}O$ ($0.08\%$).
- **Picco $M+2$ ($0.8\%$ rispetto ad $M$):** dominato dal contributo dell'isotopo
  pesante dell'ossigeno $^{18}O$ (2 atomi $\times 0.205\% = 0.41\%$) e dalla
  probabilità statistica che siano presenti contemporaneamente due atomi di $^{13}C$
  nella stessa molecola ($\approx 0.33\%$).

---

### Passo 1.4: Calcolo dell'Errore Sperimentale di Massa (in ppm)
L'accuratezza di misura di uno spettrometro di massa ad alta risoluzione
si esprime in parti per milione (ppm) mediante la formula:

$$\Delta m\ (\text{ppm}) = \frac{m/z_{\text{sperimentale}} - m/z_{\text{teorico}}}{m/z_{\text{teorico}}} \times 10^6$$

Nel nostro spettro:
- $m/z_{\text{sperimentale}} = 152.0709$
- $m/z_{\text{teorico}} = 152.0706$

$$\Delta m\ (\text{ppm}) = \frac{152.0709 - 152.0706}{152.0706} \times 10^6 = \frac{+0.0003}{152.0706} \times 10^6 = +1.97\text{ ppm}$$

Poiché l'errore calcolato ($+1.97\text{ ppm}$) è inferiore alla soglia
convenzionale di affidabilità strumentale ($\le 5\text{ ppm}$), l'assegnazione
della formula $C_8H_9NO_2$ è pienamente confermata.

---

## Esercizio 2: Ricerca e Discriminazione di Fotoprodotti della Carbamazepina ($C_{15}H_{12}N_2O$)

### Obiettivi Didattici
1. Riconoscere il composto progenitore (parent) Carbamazepina.
2. Formulare l'ipotesi di foto-ossidazione/idrossilazione ($+O$, $+15.9949\text{ Da}$)
   e calcolare il valore esatto di $m/z$ atteso per i prodotti di trasformazione (TP).
3. Discriminare mediante tempo di ritenzione ed estrazione XIC i diversi
   isomeri a $m/z\ 253.0972$: Carbamazepina-10,11-epossido (CBZ-EP),
   Oxcarbazepina (OXC) e derivati idrossilati fenolici (OH-CBZ).

---

### Passo 2.1: Proprietà del Parent e Ipotesi di Trasformazione
La Carbamazepina ha formula molecolare $C_{15}H_{12}N_2O$:
- Massa monoisotopica neutra: $236.094965\text{ Da}$
- $[M+H]^+$ teorico: $237.102241\text{ Da} \approx 237.1022$

Nei processi di ossidazione avanzata (fotocatalisi eterogenea con $TiO_2$ o
fotolisi UV), l'attacco di radicali ossidrili $\cdot OH$ o di ossigeno singoletto
determina l'incorporazione di un atomo di ossigeno ($+O$, $+15.994915\text{ Da}$):
- Formula dei fotoprodotti $+O$: $C_{15}H_{12}N_2O_2$
- Massa neutra monoisotopica: $236.094965 + 15.994915 = 252.089880\text{ Da}$
- $[M+H]^+$ teorico: $252.089880 + 1.007276 = 253.097156\text{ Da} \approx 253.0972$

---

### Passo 2.2: Estrazione dell'XIC a Risoluzione di 5 ppm
1. Nella vista dati, aprire il menu di estrazione o fare clic destro sul grafico.
2. Inserire il valore di $m/z = 253.0972$ con tolleranza impostata a $5\text{ ppm}$.
3. Eseguire l'estrazione su tutta la serie cinetica (file a $t_0, t_5, t_{15}, t_{30}$).

[Screenshot: Pannello cromatografico con il tracciato XIC estratto a m/z
253.0972 (± 5 ppm). Mentre al tempo zero (t0) la traccia è completamente piatta
sul rumore di fondo, nei campioni irraggiati (t15 e t30) compaiono tre picchi
cromatografici distinti e perfettamente separati: Picco A a tR = 4.82 min,
Picco B a tR = 5.65 min e Picco C a tR = 6.40 min. In alto una legenda a colori
distingue i diversi tempi di campionamento.]

---

### Passo 2.3: Discriminazione Chimica degli Isomeri
Tutti e tre i picchi possiedono lo stesso ione $[M+H]^+$ a $m/z\ 253.0972$
(sono isomeri costituzionali con la medesima formula bruta $C_{15}H_{12}N_2O_2$):
1. **Carbamazepina-10,11-epossido (CBZ-EP):**
   L'ossigenazione avviene sul doppio legame centrale alifatico C10-C11,
   formando un anello ossiranico (epossido a ponte). È un fotoprodotto tipico
   e il principale metabolita biologico. In fase inversa C18, possiede una
   ritenzione intermedia (Picco B a $5.65\text{ min}$).
2. **Oxcarbazepina (OXC):**
   Cheto-derivato (10-osso-10,11-diidrocarbamazepina) caratterizzato da un
   carbonile in C10 e idrogeni saturi in C11. Eluisce a $t_R = 6.40\text{ min}$
   (Picco C).
3. **Idrossicarbamazepine fenoliche (es. 2-OH-CBZ o 3-OH-CBZ):**
   L'attacco radicalico $\cdot OH$ su uno dei due anelli benzenici aromatici
   forma un gruppo fenolico libero. Questo aumenta notevolmente la polarità
   della molecola e la sua solubilità nella fase mobile acquosa: in fase
   inversa eluisce significativamente prima del parent (Picco A a $4.82\text{ min}$).

[Screenshot: Spettro MS/MS a confronto per il picco A e per il picco B.
Nel riquadro superiore, lo spettro di frammentazione del picco B (epossido)
mostra la perdita caratteristica del carbammile (-43 Da, HNCO) verso m/z 210.0913
e la formazione del frammento acridinico tipico a m/z 180.0808. Nel riquadro
inferiore, il picco A (idrossilato fenolico) mostra lo ione carbammil-decarbossilato
a m/z 210.0913 e il frammento aromatico ossigenato spostato di +16 Da a
m/z 196.0757.]

---

## Esercizio 3: Analisi di Spettri Tandem MS/MS di Derivati Fenolici

### Obiettivi Didattici
1. Identificare le frammentazioni diagnostiche tipiche dei composti contenenti
   funzioni fenoliche sotto dissociazione indotta da collisione (CID).
2. Riconoscere le perdite neutre caratteristiche di monossido di carbonio ($-CO$,
   $-27.9944\text{ Da}$) e di acqua ($-H_2O$, $-18.0106\text{ Da}$).
3. Utilizzare il righello $\Delta m/z$ di mzLab e la tabella integrata delle
   perdite neutre per l'attribuzione strutturale.

---

### Passo 3.1: Chimica della Frammentazione dei Fenoli
In spettrometria di massa tandem (MS/MS in cella di collisione a gas inerte),
gli ioni generati da composti fenolici subiscono riarrangiamenti strutturali
altamente specifici:
- **Perdita di Monossido di Carbonio ($-CO$, $-28\text{ Da}$ nominale, $-27.9944\text{ Da}$ esatta):**
  Lo ione molecolare o il frammento fenolico subisce una tautomerizzazione
  cheto-enolica verso la forma cicloesadienonica; segue una contrazione d'anello
  con espulsione di $CO$ e formazione di uno ione a cinque termini ciclopentadienilico
  altamente stabilizzato.
- **Perdita di Acqua ($-H_2O$, $-18\text{ Da}$ nominale, $-18.0106\text{ Da}$ esatta):**
  Frequente quando sulla catena laterale o sul nucleo sono presenti idrossili
  alifatici o gruppi carbossilici/amminici adiacenti capaci di cedere protoni.

---

### Passo 3.2: Misura delle Perdite Neutre in mzLab
1. Caricare un file contenente spettri di frammentazione MS2 (es. `MS2_t15.mzML`).
2. Sul cromatogramma, posizionarsi sulla scansione corrispondente all'isolamento
   del precursore idrossilato (ione progenitore isolato a $m/z\ 210.0913$).
3. Aprire il pannello inferiore dedicato allo spettro MS2.

[Screenshot: Spettro MS/MS tandem con asse verticale in intensità relativa
percentuale (0 - 100%). Il picco del precursore a m/z 210.0913 è presente a bassa
intensità residua. Nel corpo dello spettro sono visibili picchi frammento intensi
a m/z 182.0965, m/z 167.0730 e m/z 154.1012.]

4. Attivare il **Righello** cliccando sul picco precursore a $m/z\ 210.0913$
   (impostato come «rif.»).
5. Fare clic sul frammento secondario a $m/z\ 182.0965$.

[Screenshot: Applicazione del righello tra il precursore a m/z 210.0913 e il
frammento a m/z 182.0965. Sopra l'intervallo compare l'indicazione:
«Δm/z 27.9948 (27.99 mDa)». Facendo clic destro sull'etichetta si apre il menu
a tendina evidenziando la prima voce: «Cerca 28.0 nelle perdite neutre».]

6. Selezionare la voce **«Cerca nelle perdite neutre»**:
   Nel pannello laterale si apre la banca dati delle perdite neutre con il campo
   di ricerca automaticamente popolato con $27.99$.
   La tabella restituisce come primo risultato univoco:
   - *Formula:* $CO$
   - *Massa esatta:* $27.99437\text{ Da}$
   - *Descrizione:* «Monossido di carbonio (contrazione di anelli fenolici e chinonici)».

7. Ripetere la misura con il righello partendo dal frammento a $m/z\ 182.0965$
   fino al frammento a $m/z\ 164.0858$:
   - La differenza rilevata è $\Delta m/z = 18.0107\text{ Da}$, corrispondente
     alla perdita successiva di una molecola di acqua ($-H_2O$, $18.01056\text{ Da}$).
   - La sequenza combinata $-CO - H_2O$ (perdita complessiva di $46.0051\text{ Da}$)
     conferma in modo inequivocabile la natura idrossiaromatica dell'analita.
