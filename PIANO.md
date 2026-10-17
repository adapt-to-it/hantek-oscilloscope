# Hantek 6022BE per macOS: piano di architettura

## Stato rilevato

- Il dispositivo è collegato: `04b4:6022`, USB 2.0 High Speed, senza firmware caricato.
- Il 6022BE non conserva il firmware. L'host lo carica nella RAM del Cypress FX2LP a ogni connessione. Un firmware difettoso non danneggia il dispositivo: basta scollegarlo.
- Toolchain: Rust 1.97.1 installato. SDCC 4.6.0 disponibile tramite Homebrew, da installare.
- Base del firmware: `Ho-Ro/Hantek6022API`, cartella `Firmware/DSO6022BE`, licenza LGPL-2.1, scritto in C per SDCC sopra fx2lib.

## Limiti dell'hardware

Il software non supera questi limiti. Le prestazioni migliorano dentro questi confini.

| Parametro | Valore |
|---|---|
| Canali | 2 analogici, nessun ingresso digitale |
| Risoluzione | 8 bit |
| Banda | 20 MHz |
| Campionamento | 20 kS/s – 48 MS/s (48 MS/s con un solo canale) |
| Trigger hardware | assente: il trigger avviene nel software |
| Portata USB bulk reale | circa 35–40 MB/s su macOS |
| Accoppiamento AC | richiede la modifica hardware descritta da Ho-Ro |

Conseguenza: con due canali lo streaming continuo senza buchi arriva a circa 16–20 MS/s per canale. Oltre, il software acquisisce a blocchi.

Conseguenza per i decoder: con due soli canali analogici SPI completo (CLK, MOSI, MISO, CS) non è decodificabile. I2C, UART, CAN, 1-Wire, PWM, IR e Manchester sì.

## Architettura

Un workspace Cargo con un motore unico. La GUI, il server MCP e la CLI usano lo stesso motore. Un solo processo possiede il dispositivo USB.

```
hantek6022/
├── firmware/            C, SDCC, LGPL-2.1 (fork del firmware DSO6022BE)
├── crates/
│   ├── h6022-usb        caricamento firmware, comandi vendor, EEPROM, streaming bulk (nusb, senza libusb)
│   ├── h6022-core       buffer circolari, calibrazione, trigger software, base dei tempi, modalità roll
│   ├── h6022-dsp        misure automatiche, FFT, canali matematici, filtri, XY, persistenza
│   ├── h6022-decode     decoder seriali, analisi PWM, riconoscimento di pattern
│   ├── h6022-record     registrazione su disco e formati di esportazione
│   ├── h6022-server     API locale: MCP (HTTP streamable) e WebSocket per Python
│   ├── h6022-gui        app egui/wgpu
│   └── h6022-cli        comandi da terminale e modalità headless
└── python/              client Python di esempio (numpy)
```

### Firmware (C)

Modifiche previste rispetto al firmware di Ho-Ro:
- comando di versione e di capacità, per verificare cosa il firmware supporta;
- dimensione dei pacchetti e configurazione FIFO per massimizzare la portata bulk;
- contatore di sequenza o marcatore di overflow, per rilevare campioni persi;
- frequenza di calibrazione e accoppiamento AC/DC già presenti, mantenuti.

Il binario del firmware entra nell'eseguibile Rust con `include_bytes!`. `build.rs` lo ricompila se SDCC è installato, altrimenti usa il `.hex` versionato.

### Acquisizione (Rust)

- `nusb` usa IOKit direttamente: nessun driver e nessuna dipendenza da libusb.
- Più trasferimenti bulk in coda, per evitare buchi tra un pacchetto e l'altro.
- Thread dedicato all'USB, buffer senza lock verso l'elaborazione.
- Calibrazione di offset e guadagno letta dalla EEPROM, con procedura di ricalibrazione.

### Funzioni da oscilloscopio

- Trigger: fronte, larghezza di impulso, runt, finestra, pendenza, timeout, pattern su due canali. Modalità auto, normale, singola.
- Misure: Vpp, Vmax, Vmin, media, RMS, frequenza, periodo, duty cycle, tempi di salita e discesa, overshoot, fase tra canali.
- Cursori orizzontali e verticali, statistiche sulle misure.
- FFT con finestre (Hann, Blackman-Harris, flat-top), spettrogramma.
- Canali matematici: somma, differenza, prodotto, integrale, derivata, filtri passa-basso e passa-alto.
- Modalità XY, persistenza a colori, roll per le basi dei tempi lente.
- Interpolazione sin(x)/x e decimazione min/max.

### Decoder e analisi

- UART con stima automatica del baud rate, I2C, CAN, 1-Wire, PWM e servo, IR (NEC, RC5), Manchester.
- Riconoscimento di pattern: classificazione automatica del segnale (periodico, PWM, burst, seriale), ricerca di una forma d'onda campione dentro una registrazione, segmentazione dei pacchetti.
- Ogni funzione ha una spiegazione dettagliata. Il tasto `i` mostra o nasconde le spiegazioni sull'elemento sotto il puntatore.

### Server MCP e script

- Il server MCP gira dentro l'app, su `127.0.0.1`, con trasporto HTTP streamable. Lo stesso binario offre una modalità headless.
- Strumenti MCP previsti: stato del dispositivo, configurazione, acquisizione singola, istantanea in tempo reale, misure, decodifica, avvio e arresto della registrazione, analisi automatica del segnale.
- WebSocket binario per Python: streaming dei campioni come array numpy.
- Formati di esportazione: CSV, NumPy `.npy`, WAV, VCD per i dati decodificati, sessione sigrok `.sr`.

## Fasi

Ogni fase si chiude con un test sul dispositivo reale.

1. **Fondamenta.** Workspace, build del firmware, caricamento, comandi base, streaming, CLI `h6022 capture`. Misura della portata reale su questo Mac.
2. **Motore.** Calibrazione, trigger, misure, FFT, test unitari con segnali sintetici.
3. **GUI.** Display, controlli, cursori, misure, spiegazioni con il tasto `i`.
4. **Server.** MCP e WebSocket, client Python di esempio, registrazione su disco.
5. **Decoder e pattern.** Decoder seriali, PWM, classificazione e ricerca di pattern.
6. **Rifinitura.** Profilazione, pacchetto `.app` firmato localmente, documentazione.

## Licenze

- Firmware: LGPL-2.1, come il codice di origine.
- Codice Rust: scritto da zero, senza codice copiato dal progetto Python GPL-3. Licenza proposta: MIT o Apache-2.0. Il firmware resta un file separato caricato sul dispositivo.

## Decisioni aperte

- Licenza del codice Rust.
- Posizione del progetto: questa cartella, con `git init`.
