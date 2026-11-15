# Il primo file di un lotto: cancello delle copie, misure

Misure del cancello delle copie del sito (`createCopyGate` in `rust-carica.js`, scelta `copiesOf` in `lr.js`; comportamento in `dati-carico.md`, «Il primo file prima»): in un lotto al più K copie in OPFS (`lr_put`) insieme, nell'ordine dell'elenco; K = 0 = tutte insieme, come senza cancello. Predefinito K = 2. Solo sito: l'app desktop carica già un file alla volta (catena di `upload0` in `shell.js`) e non copia i file.

Strumento: `tests_e2e/misura_caricamento.py --scenarios copie,raw-copie,copie-mem,cinquanta` (bracci in `tests_e2e/scenari_caricamento.py`, `copie` = localStorage `mzlab.dev.copie`). Base `main` `7e81096`, sito di `tools/prepara_sito.py`, Chrome for Testing 153 headless, un profilo persistente caldo per giro (finestra normale, isolamento cross-origin), macchina cloud 4 vCPU Intel Xeon 2,30 GHz. N = 5 per cella, bracci alternati giro per giro, mediana [min-max], ms dal `change` dell'input. CPU 1× e 4× rallentano solo il thread principale della pagina (i worker vanno a piena velocità).

File: mzML sintetico 50,7 MB (2400 scansioni) e `.raw` Thermo sintetico 50,6 MB come in `misure-caricamento.md` («First file and many files»); il lotto da 50 = mzML di ~10 MB (480 scansioni). Un lotto = il primo file più copie con altri nomi. TIC provvisorio = `#ticprov` disegnato; primo grafico = un pannello disegnato; tutto indicizzato = niente più in lettura o apertura.

**Soglia** (decisa prima di misurare): primo grafico con dieci file ≥ 25 % prima di K = 0 E tutto il lotto entro +5 %. K = 2 la passa a 1× (mzML −27 %, lotto −13 %; `.raw` −49 %, −15 %) e per i 50 file (−53 %, −1 %; 4×: −41 %, −4 %); a 4× con dieci mzML il primo grafico è solo −10 % (K = 1: −23 %).

K = 1 dà il primo grafico più presto ma allunga il lotto oltre +5 % in due misure su tre a 1× (50 file +12 %, braccio con la memoria +15 %): una copia alla volta lascia fermi i costruttori quando i file sono piccoli. Indici identici: SHA-256 di ogni `.mzidx` uguale fra K = 2 e K = 0 (`tests_e2e/e2e_copie_coda.py`).

Dove va il tempo del primo file (mzML, 1×, un giro tipico): da solo il costruttore legge in 213 ms e il grafico arriva 309 ms dopo l'indice; con K = 1 la lettura dura 586 ms (CPU divisa con la copia seguente e gli altri costruttori) e il grafico arriva 535 ms dopo l'indice (il worker del motore serve anche le copie); con K = 0 la copia del primo file finisce solo con le altre (1,8 s invece di 0,3 s).

**Dieci mzML da 53 MB**

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | uno | 471 [468-554] | 1306 [1268-1483] | 1313 [1280-1494] | - | - |
| 1× | dieci-k0 | 2036 [1908-2124] | 3456 [3104-3683] | 6764 [6308-6966] | +0 % | +0 % |
| 1× | dieci-k1 | 629 [571-657] | 2372 [2233-2595] | 6440 [6157-6646] | -31 % | -5 % |
| 1× | dieci-k2 | 885 [750-1184] | 2538 [2376-2990] | 5871 [5169-5995] | -27 % | -13 % |
| 1× | dieci-k3 | 1061 [975-1425] | 2776 [2536-3182] | 5648 [5104-6373] | -20 % | -16 % |
| 4× | uno | 627 [594-752] | 2230 [2155-2444] | 2310 [2269-2570] | - | - |
| 4× | dieci-k0 | 2445 [2243-2708] | 4554 [4403-4919] | 8546 [7863-8892] | +0 % | +0 % |
| 4× | dieci-k1 | 941 [791-1119] | 3487 [3181-4365] | 7497 [7304-8981] | -23 % | -12 % |
| 4× | dieci-k2 | 1160 [930-1306] | 4097 [3494-4365] | 7616 [7143-8022] | -10 % | -11 % |
| 4× | dieci-k3 | 1421 [1238-1755] | 4323 [3992-4982] | 7996 [7526-9027] | -5 % | -6 % |

**Dieci `.raw` da 51 MB**

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | raw-uno | 269 [226-310] | 992 [926-1051] | 1026 [950-1061] | - | - |
| 1× | raw-dieci-k0 | 581 [481-650] | 4390 [3848-4727] | 5613 [4802-5963] | +0 % | +0 % |
| 1× | raw-dieci-k1 | 245 [210-287] | 1593 [1528-1680] | 4895 [4415-5223] | -64 % | -13 % |
| 1× | raw-dieci-k2 | 349 [292-480] | 2239 [2090-2747] | 4763 [4562-5213] | -49 % | -15 % |
| 1× | raw-dieci-k3 | 422 [307-526] | 3115 [2813-3472] | 5057 [4788-5084] | -29 % | -10 % |
| 4× | raw-uno | 406 [386-429] | 1910 [1446-1942] | 1948 [1530-2046] | - | - |
| 4× | raw-dieci-k0 | 792 [675-1024] | 5603 [5238-5998] | 7900 [7291-8986] | +0 % | +0 % |
| 4× | raw-dieci-k1 | 440 [418-519] | 2980 [2774-3327] | 5975 [5739-6435] | -47 % | -24 % |
| 4× | raw-dieci-k2 | 566 [459-665] | 3479 [3371-3737] | 6679 [6505-7896] | -38 % | -15 % |
| 4× | raw-dieci-k3 | 597 [528-1040] | 4034 [3482-5637] | 6848 [6698-9131] | -28 % | -13 % |

**Cinquanta mzML da 10 MB** (fino a 50 file il lotto va a pipeline: nessuna domanda «Scegli»)

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | cinquanta-k0 | 2395 [2197-2752] | 3183 [2818-3658] | 12841 [11414-14527] | +0 % | +0 % |
| 1× | cinquanta-k1 | 588 [513-995] | 1310 [1224-1832] | 14362 [13178-15887] | -59 % | +12 % |
| 1× | cinquanta-k2 | 744 [646-810] | 1482 [1390-1629] | 12692 [12517-13147] | -53 % | -1 % |
| 4× | cinquanta-k0 | 3739 [3410-3762] | 5598 [5177-6009] | 25515 [23631-28049] | +0 % | +0 % |
| 4× | cinquanta-k1 | 1730 [1443-2288] | 3428 [3142-4313] | 23054 [20942-24078] | -39 % | -10 % |
| 4× | cinquanta-k2 | 1617 [1535-1994] | 3308 [3163-4479] | 24551 [23479-25358] | -41 % | -4 % |

**Memoria, dieci mzML** (`measureUserAgentSpecificMemory` ogni 500 ms, gruppo di agenti della pagina; RSS di tutti i processi del browser da `/proc`; nessun errore in nessun giro)

| CPU | arm | first chart | all indexed | all vs K0 | page memory peak MB | RSS peak MB |
|---|---|---|---|---|---|---|
| 1× | dieci+mem-k0 | 3475 [3437-4459] | 6191 [6049-7677] | +0 % | 65 [64-200] | 1910 [1795-2012] |
| 1× | dieci+mem-k1 | 2662 [2343-2772] | 7139 [6553-7820] | +15 % | 100 [64-115] | 1816 [1765-1917] |
| 1× | dieci+mem-k2 | 2614 [2264-2803] | 6277 [5858-6596] | +1 % | 66 [59-102] | 1929 [1778-1957] |
| 4× | dieci+mem-k0 | 5292 [4507-6339] | 9563 [8470-10602] | +0 % | 67 [66-161] | 1832 [1799-1884] |
| 4× | dieci+mem-k1 | 4002 [3671-4186] | 8494 [7781-9022] | -11 % | 104 [60-161] | 1832 [1763-1875] |
| 4× | dieci+mem-k2 | 3955 [3172-4642] | 8152 [6841-8811] | -15 % | 117 [59-161] | 1792 [1767-1819] |

