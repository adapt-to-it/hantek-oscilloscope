# Hantek 6022BE per macOS: descrizione della GUI per il redesign

Questo documento descrive ogni vista e ogni funzione dell'app desktop. Gli screenshot sono in `docs/screenshots/`. Sono stati catturati con il simulatore, in una finestra di 1280 × 1350 punti.

## Contesto

L'app controlla un oscilloscopio USB a due canali (Hantek 6022BE). Il dispositivo campiona fino a 48 MS/s con una banda di circa 20 MHz. La risoluzione è di 8 bit. L'app mostra il segnale in tempo reale a circa 60 frame al secondo, calcola le misure e decodifica i protocolli digitali.

Utenti tipici: hobbisti di elettronica, maker, studenti, tecnici che fanno debug di schede con microcontrollori. L'app deve funzionare sia per chi conosce gli oscilloscopi da banco sia per chi li usa per la prima volta. Per questo esiste la modalità "Spiegazioni" (vedi sotto).

La stessa acquisizione è raggiungibile anche da Claude Code tramite un server MCP integrato. La GUI mostra lo stato di questo server nella barra superiore.

Piattaforma: macOS, finestra ridimensionabile, dimensione iniziale 1280 × 800. Tema scuro. L'interfaccia è in italiano.

## Struttura della finestra

La finestra ha quattro aree:

| Area | Posizione | Contenuto |
|---|---|---|
| Barra di stato | in alto, a tutta larghezza | stato del motore, frame al secondo, stato del server MCP |
| Display | al centro, in alto | tracce, griglia, marcatori, cursori. Cambia con la vista: Tempo, Spettro, XY |
| Striscia del decoder | sotto il display, solo se il decoder è attivo | annotazioni del protocollo e riepilogo |
| Tabella delle misure | sotto il display | misure automatiche per CH1 e CH2 |
| Pannello dei controlli | colonna a destra, larghezza 300 punti, scorrevole | sezioni comprimibili: Acquisizione, Orizzontale, CH1, CH2, Trigger, Display, Decoder |

Colori dei canali, usati ovunque (tracce, etichette, intestazioni, marcatori):
- CH1: giallo.
- CH2: azzurro.

## Barra di stato

Esempio: `Motore: in esecuzione   61.5 frame/s   MCP http://127.0.0.1:6022/mcp`

- **Stato del motore**: in esecuzione, in attesa di trigger, fermo.
- **Frame al secondo**: quante schermate nuove arrivano dal motore.
- **Server MCP**: indirizzo del server se attivo; "MCP spento" se avviato con `--no-server`; messaggio di errore se la porta è occupata.
- **Errori**: un errore di configurazione compare in rosso per 5 secondi. Esempio: frequenza non supportata con due canali.

## Viste del display

La vista si sceglie nella sezione Display, oppure con il tasto `F` (alterna Tempo e Spettro).

### Vista Tempo (screenshot 01, 02, 03)

È la vista principale: tensione in funzione del tempo, come su un oscilloscopio da banco.

- **Griglia**: 10 divisioni orizzontali e 8 verticali, con tacche secondarie sugli assi centrali.
- **Tracce**: una per canale attivo, nel colore del canale.
- **Marcatori di livello zero**: triangoli sul bordo sinistro, uno per canale, alla quota di 0 V del canale. Si spostano con l'offset verticale.
- **Marcatore del trigger, in alto**: triangolo nel colore della sorgente del trigger. Indica l'istante del trigger (t = 0). Si sposta con il pretrigger e l'offset orizzontale.
- **Marcatore del livello di trigger, a destra**: triangolo trascinabile con il mouse. Trascinarlo cambia il livello di trigger.
- **Riga informativa in basso nel display**: `CH1 1 V/div   CH2 1 V/div   100 µs/div   fs 16 MHz   TRIG'D`. L'ultima voce è `TRIG'D` quando il frame è sincronizzato su un trigger, `AUTO` quando il motore lo ha prodotto senza trigger.
- **Senza dati**: il display mostra "In attesa di dati".
- **Interpolazione**: con pochi campioni per divisione la traccia viene interpolata (sin(x)/x) per apparire continua.

**Cursori** (screenshot 02): due linee verticali (tempo, t1 e t2) e due orizzontali (tensione, V1 e V2), in grigio chiaro, trascinabili. Le etichette riportano i valori e le differenze: Δt, 1/Δt (frequenza), ΔV. Modi: Off, Tempo, Tensione, Entrambi. Una seconda scelta indica a quale canale si riferiscono i cursori di tensione.

**Persistenza** (screenshot 03): le ultime 8 schermate restano visibili e sfumano verso la trasparenza. Serve a vedere jitter, glitch e segnali instabili.

### Vista Spettro (screenshot 05)

Mostra lo spettro di frequenza (FFT) di ciascun canale attivo.

- Asse verticale in dBV (decibel rispetto a 1 V efficace), da +20 a −120 dBV.
- Asse orizzontale in frequenza, lineare o logaritmico.
- **Picchi**: le frequenze principali sono etichettate sul grafico con frequenza e livello. Esempio: "10.8 kHz 2.7 dBV".
- **Lettura al passaggio del mouse**: una linea verticale segue il puntatore e mostra frequenza e livello in quel punto.
- **Finestra FFT**: Rettangolare, Hann, Hamming, Blackman-Harris, Flat-top. Flat-top dà ampiezze precise; Hann e Blackman-Harris separano meglio frequenze vicine.

Nota per il design: le etichette dei picchi oggi si sovrappongono tra loro e alla legenda dell'asse.

### Vista XY (screenshot 04)

CH1 sull'asse orizzontale, CH2 sull'asse verticale. Serve per le figure di Lissajous, per confrontare fase e frequenza di due segnali, e per le curve caratteristiche dei componenti. Richiede entrambi i canali attivi. I punti sono disegnati con trasparenza al 30 %.

Nota: nello screenshot i due segnali sono onde quadre, quindi la figura si riduce a pochi punti. Con due sinusoidi compare un'ellisse o una figura di Lissajous.

## Striscia del decoder (screenshot 06, 07, 08, 09)

Compare sotto il display quando il decoder è attivo. Allinea sul tempo del display le parti del messaggio decodificato.

- Ogni elemento è un rettangolo con testo, posto tra l'istante di inizio e di fine. Esempi: `start`, `0x55 'U'`, `stop`, `duty 50,0 %`.
- Il colore dipende dal tipo di elemento. Il testo è nero, tranne negli errori:

| Tipo | Significato | Colore attuale |
|---|---|---|
| data | byte di dati | azzurro |
| address | indirizzo I2C | arancione |
| id | identificativo CAN | viola |
| ack | conferma ricevuta | verde |
| nack | conferma mancante | rosso chiaro |
| start / stop | inizio e fine del frame | grigio |
| command | comando (IR, 1-Wire) | verde acqua |
| crc | controllo di integrità | giallo |
| reset / presence | impulsi 1-Wire | marrone |
| error | errore di framing, parità, CRC | rettangolo rosso pieno con testo bianco |

- Se un elemento è troppo stretto per il testo, il testo viene omesso e resta solo il rettangolo.
- **Riga di riepilogo** sotto la striscia: numero di byte, numero di errori, byte in esadecimale e in testo. Esempio: `2 byte, 0 errori: 55 55 "UU"`.
- Se il decoder non può lavorare (per esempio I2C senza CH2 attivo), la striscia mostra un messaggio in rosso.

Protocolli disponibili:

| Protocollo | Canali | Cosa mostra |
|---|---|---|
| UART | 1 | start, byte (hex e carattere), parità, stop, errori di framing |
| I2C | 2 (SCL e SDA) | start, start ripetuto, indirizzo e lettura/scrittura, byte, ack/nack, stop |
| CAN 2.0A/B | 1 | inizio, identificativo standard o esteso, lunghezza, dati, CRC, ack |
| 1-Wire | 1 | reset, presenza, comando ROM, byte |
| Manchester | 1 | bit e byte |
| PWM | 1 | duty cycle di ogni periodo |
| Servo | 1 | larghezza dell'impulso e angolo stimato, per esempio "1,50 ms (90°)" |
| IR NEC | 1 | indirizzo, comando, ripetizione |
| IR RC5 | 1 | bit di toggle, indirizzo, comando |

## Tabella delle misure

Una riga per misura, una colonna per canale (intestazioni nel colore del canale). Quando una misura non è calcolabile compare "—". Esempio: la frequenza di un livello continuo.

| Misura | Significato |
|---|---|
| Vpp | tensione picco-picco |
| Vmax, Vmin | valore massimo e minimo |
| Vmedia | valore medio (componente continua) |
| Vrms | valore efficace totale |
| Vac rms | valore efficace della sola parte alternata |
| Vtop, Vbase | livello alto e basso stabili di un'onda quadra, senza sovraelongazioni |
| Ampiezza | Vtop − Vbase |
| Frequenza, Periodo | ricavati dai fronti di salita |
| Duty | percentuale del periodo a livello alto |
| Larghezza +, Larghezza − | durata dell'impulso alto e basso |
| Salita, Discesa | tempo dal 10 % al 90 % del fronte |
| Overshoot | sovraelongazione oltre Vtop, in percentuale |
| Fronti | numero di fronti di salita / discesa nel frame |

Sotto la tabella, due misure tra canali:
- **Fase CH1-CH2**: sfasamento in gradi.
- **Ritardo CH1-CH2**: ritardo tra i fronti dei due canali.

Le misure si aggiornano solo quando arriva un frame nuovo.

## Pannello dei controlli

Tutte le sezioni sono comprimibili. Ogni controllo ha una spiegazione che compare al passaggio del mouse quando le spiegazioni sono attive.

### Acquisizione
- **Run / Stop / Single**: avvia, ferma, oppure acquisisce un solo frame al prossimo trigger e poi si ferma. Il pulsante attivo è evidenziato.
- **Trigger** (modo): Auto (mostra il segnale anche senza trigger), Normal (aggiorna solo sui trigger), Single, Roll (scorrimento continuo per segnali lenti, senza trigger).
- **Riga informativa**: frequenza di campionamento e lunghezza del record scelte in automatico dalla base dei tempi. Esempio: "Frequenza 16 MHz, record 16000 campioni".
- **Misurata**: frequenza di campionamento effettiva, calcolata sui dati ricevuti via USB.

### Orizzontale
- **Tempo/div**: base dei tempi, con pulsanti − e + e un menu a scelta (sequenza 1-2-5 da 100 ns a 10 s per divisione).
- **Offset**: spostamento orizzontale in divisioni. Il pulsante **Centra** lo azzera.
- **Pretrigger**: cursore 0–100 %; indica quanta parte della schermata mostra il segnale prima del trigger.

### CH1 e CH2 (uguali)
- **Attivo**: mostra o nasconde il canale. Con un solo canale il dispositivo può campionare più velocemente.
- **V/div**: scala verticale con − e + e menu (sequenza 1-2-5 da 10 mV a 5 V per divisione). La portata hardware (±5 V, ±2,5 V, ±1 V, ±0,5 V) viene scelta in automatico.
- **Offset**: spostamento verticale in divisioni.
- **Sonda**: x1 o x10. Corregge le tensioni mostrate per il fattore della sonda.
- **Accoppiamento**: DC o AC. Indica come è impostato il commutatore sul dispositivo.

### Trigger
- **Sorgente**: CH1 o CH2.
- **Tipo**, con i campi che cambiano a seconda del tipo:
  - Edge: fronte di salita, discesa o entrambi.
  - PulseWidth: impulso alto o basso, minore di, maggiore di, dentro o fuori un intervallo di durata.
  - Runt: impulso che supera una soglia bassa ma non una soglia alta.
  - Window: il segnale entra o esce da una fascia di tensione.
  - Timeout: nessun fronte per un tempo stabilito.
  - Pattern: combinazione di livelli alto, basso o indifferente su CH1 e CH2.
- **Livello**: tensione di soglia (trascinabile anche sul display).
- **Isteresi**: margine che evita trigger multipli sul rumore.
- **Holdoff**: tempo minimo tra due trigger.

### Display
- **Vista**: Tempo, Spettro, XY.
- **Finestra FFT**: vedi Vista Spettro.
- **Asse delle frequenze logaritmico**.
- **Persistenza**.
- **Cursori**: modo e canale di riferimento.
- **Spiegazioni**: attiva i suggerimenti. Sotto compare "Suggerimento: tasto i".

### Decoder
- **Attiva**: mostra la striscia sotto il display.
- **Protocollo**: vedi tabella dei protocolli.
- **Canale**: canale da decodificare (per I2C è il canale di SCL; SDA è l'altro).
- **Soglia automatica** e valore della soglia in volt (modificabile se la soglia automatica è spenta).
- Campi specifici del protocollo:
  - UART: Baud automatico, velocità in bit/s, bit di dati (5–9), parità (Nessuna, Pari, Dispari), bit di stop (1, 1,5, 2), Linea invertita.
  - CAN: Bitrate, Linea invertita.
  - Gli altri protocolli non hanno parametri.

## Spiegazioni (screenshot 10)

Con il tasto `i`, o con la casella "Spiegazioni", ogni controllo e ogni misura mostra al passaggio del mouse un riquadro con titolo e 2–3 frasi. Esistono 39 spiegazioni. Esempio: "Isteresi", "Tempo di salita", "Velocità UART".

Nota: lo screenshot 10 mostra la modalità attiva ma non un riquadro aperto, perché la cattura automatica non simula il passaggio del mouse. Il riquadro attuale è largo al massimo 320 punti: titolo in grassetto e testo normale.

## Tastiera e mouse

| Comando | Effetto |
|---|---|
| Spazio | Run / Stop |
| S | Single |
| F | alterna Tempo e Spettro |
| I | attiva o disattiva le spiegazioni |
| P | salva un'immagine PNG della finestra |
| Trascinamento del triangolo a destra | cambia il livello di trigger |
| Trascinamento dei cursori | sposta t1, t2, V1, V2 |
| Mouse sullo spettro | lettura di frequenza e livello |

I tasti non agiscono mentre si scrive in un campo.

## Funzioni non visibili nella GUI

Queste funzioni esistono nel server MCP e nel comando `h6022`, ma la GUI non ha ancora controlli per usarle. Il designer può prevederli:

- **Registrazione su file** in CSV, NumPy `.npy` o WAV, con avvio, arresto e durata massima.
- **Esportazione logica** in VCD o sigrok `.sr`, con le annotazioni del decoder, da aprire in PulseView o GTKWave.
- **Classificazione del segnale**: riconosce in automatico clock, PWM, seriale, burst, sinusoide, rumore.
- **Ricerca di pacchetti** e **ricerca di una forma d'onda** dentro la registrazione.
- **Funzioni matematiche** tra canali (somma, differenza, prodotto, derivata, integrale) e **filtri digitali** (passa-basso, passa-alto, passa-banda).
- **Calibrazione** letta dalla EEPROM del dispositivo, visibile solo con `h6022 info`.
- **Stato del dispositivo**: versione firmware, collegato o no, simulatore in uso.

## Difetti visibili negli screenshot

- Il pannello dei controlli si comporta male in altezza. Con la finestra a 800 punti la sezione Decoder viene disegnata sopra la sezione Trigger e nasconde Livello, Isteresi, Holdoff e Display. Con la finestra alta (come negli screenshot) resta invece un grande spazio vuoto prima del Decoder. Causa: il Decoder sta fuori dall'area scorrevole del pannello. Da risolvere nel redesign.
- Le etichette dei picchi nello spettro si sovrappongono.
- La tabella delle misure occupa poco spazio in larghezza e lascia vuota la parte destra dell'area sotto il display.
- La barra di stato non mostra se il dispositivo reale è collegato o se è in uso il simulatore.

## Elenco degli screenshot

| File | Contenuto |
|---|---|
| 01-tempo.png | vista Tempo, due canali, onda quadra 11 kHz su CH1 e 1 kHz su CH2 |
| 02-tempo-cursori.png | vista Tempo con cursori di tempo e tensione |
| 03-persistenza.png | vista Tempo con persistenza attiva |
| 04-xy.png | vista XY |
| 05-spettro.png | vista Spettro, asse logaritmico, finestra Flat-top |
| 06-decoder-pwm.png | decoder PWM su CH1 |
| 07-decoder-uart.png | decoder UART su CH1 |
| 08-decoder-i2c.png | decoder I2C (i segnali simulati non sono I2C: la striscia mostra lo stato senza dati) |
| 09-decoder-can.png | decoder CAN (stesso caso) |
| 10-spiegazioni-attive.png | modalità Spiegazioni attiva |
