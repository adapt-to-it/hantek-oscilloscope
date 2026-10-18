// Spiegazioni dei controlli: compaiono al passaggio del puntatore quando la modalità Spiegazioni è attiva (tasto i).
import { getLocale, type Locale } from '../i18n/i18n.svelte'
import { HELP_EN } from './texts.en'
import { HELP_ES } from './texts.es'
import { HELP_FR } from './texts.fr'

export type HelpId =
  | 'run' | 'stop' | 'single' | 'triggerMode' | 'triggerSource' | 'triggerKind' | 'triggerLevel'
  | 'triggerSlope' | 'hysteresis' | 'holdoff' | 'pretrigger' | 'timePerDiv' | 'horizontalOffset'
  | 'channelEnable' | 'voltsPerDiv' | 'verticalOffset' | 'probe' | 'coupling' | 'cursors' | 'view'
  | 'fftWindow' | 'persistence' | 'measurements' | 'vpp' | 'vrms' | 'vacRms' | 'frequency' | 'duty'
  | 'riseTime' | 'thd' | 'phase' | 'sampleRate' | 'recordLength' | 'math' | 'filter' | 'classify'
  | 'recording' | 'decoder' | 'mcp' | 'device' | 'trend'
  | 'explain'
  | 'status'
  | 'scope'
  | 'panelTabs'
  | 'logFrequency'
  | 'pulsePolarity'
  | 'pulseWidth'
  | 'runt'
  | 'windowTrigger'
  | 'timeoutTrigger'
  | 'patternTrigger'
  | 'logicTrigger'
  | 'otherLevel'
  | 'sequenceTrigger'
  | 'sequenceDelay'
  | 'vmax'
  | 'vmin'
  | 'vmean'
  | 'vtop'
  | 'vbase'
  | 'vamp'
  | 'fallTime'
  | 'overshoot'
  | 'period'
  | 'pulseWidthMeasure'
  | 'edges'
  | 'delay'
  | 'decoderProtocol'
  | 'decoderChannel'
  | 'decoderThreshold'
  | 'engine'

export interface HelpText {
  title: string
  /** Due o tre frasi, al massimo 250 caratteri. */
  text: string
}

export const HELP: Record<HelpId, HelpText> = {
  run: {
    title: 'Avvia',
    text: 'Il pulsante avvia l’acquisizione e aggiorna la vista in continuo. Serve a seguire un segnale che cambia nel tempo. Con un segnale PWM a 1 kHz vedi la forma ripetersi a ogni ciclo.',
  },
  stop: {
    title: 'Ferma',
    text: 'Il pulsante ferma l’acquisizione e lascia l’ultima forma sullo schermo. Serve a esaminare il segnale senza che si muova. Con un fronte troppo veloce per essere letto, fermati su di esso e osservalo con calma.',
  },
  single: {
    title: 'Singola acquisizione',
    text: 'Il pulsante cattura una sola acquisizione quando il trigger scatta, poi si ferma. Serve a registrare un evento che capita una volta sola, come l’accensione di un motore. Imposta prima il trigger sul fronte giusto.',
  },
  triggerMode: {
    title: 'Modo trigger',
    text: 'Il modo decide quando la vista si aggiorna. Automatico mostra il segnale anche senza eventi. Normale aspetta un evento che supera il livello, così la forma resta ferma.',
  },
  triggerSource: {
    title: 'Sorgente trigger',
    text: 'Sceglie il canale che fa scattare il trigger. Il trigger è il punto da cui parte ogni acquisizione. Con un segnale sul canale 1 e un riferimento sul canale 2, scegli il canale 1 per allineare la forma.',
  },
  triggerKind: {
    title: 'Tipo di trigger',
    text: 'Sceglie che cosa fa scattare il trigger, per esempio un fronte o una larghezza d’impulso. Il fronte è la scelta più semplice. Con un’onda quadra a 1 kHz, il trigger sul fronte mostra ogni periodo nello stesso punto.',
  },
  triggerLevel: {
    title: 'Livello trigger',
    text: 'Tensione di soglia che il segnale deve attraversare per far scattare il trigger. Puoi anche trascinare il triangolo a destra del display.',
  },
  triggerSlope: {
    title: 'Pendenza trigger',
    text: 'Sceglie se il trigger scatta sul fronte di salita, su quello di discesa o su entrambi. Il fronte di salita è il passaggio da basso ad alto. Con un’onda quadra a 1 kHz, il fronte di salita dà un’immagine stabile di ogni periodo.',
  },
  hysteresis: {
    title: 'Isteresi',
    text: 'Margine attorno al livello di trigger. Il segnale deve superarlo prima che un nuovo trigger sia accettato, così il rumore non causa trigger multipli.',
  },
  holdoff: {
    title: 'Attesa trigger',
    text: 'Tempo minimo tra due trigger consecutivi. Utile con segnali a raffica o con molti fronti ravvicinati, per agganciare sempre lo stesso punto.',
  },
  pretrigger: {
    title: 'Pre-trigger',
    text: 'Mostra una parte del segnale prima del trigger, così vedi che cosa succede prima dell’evento. Serve quando cerchi la causa di un guasto o di un disturbo. Con un impulso anomalo a 10 ms, il pre-trigger mostra il tratto che lo precede.',
  },
  timePerDiv: {
    title: 'Base dei tempi',
    text: 'Imposta quanto tempo rappresenta una divisione dello schermo. Un valore piccolo mostra dettagli veloci, un valore grande mostra un intervallo lungo. Con un segnale PWM a 1 kHz, 200 µs/div mostra due periodi.',
  },
  horizontalOffset: {
    title: 'Spostamento orizzontale',
    text: 'Sposta la vista nel tempo, verso destra o verso sinistra. Serve per vedere una parte del segnale che non compare sullo schermo. Con un evento 5 ms dopo il trigger, sposta la vista di 5 ms per riportarlo al centro.',
  },
  channelEnable: {
    title: 'Attiva canale',
    text: 'Mostra o nasconde la traccia di un canale. Nascondere un canale libera lo schermo quando guardi l’altro. Con due segnali collegati, attiva solo il canale 1 per osservarlo meglio.',
  },
  voltsPerDiv: {
    title: 'Volt per divisione',
    text: 'Imposta quanta tensione rappresenta una divisione verticale. Un valore piccolo ingrandisce i segnali deboli, un valore grande fa stare i segnali forti sullo schermo. Con un segnale da 0 a 3 V, 1 V/div lo mostra per intero.',
  },
  verticalOffset: {
    title: 'Spostamento verticale',
    text: 'Sposta verso l’alto o verso il basso la posizione dello zero del canale. Serve a separare due tracce che si sovrappongono. Con due segnali uguali, sposta uno di due divisioni per vederli entrambi.',
  },
  probe: {
    title: 'Sonda',
    text: 'Indica il rapporto della sonda collegata, per esempio 1x o 10x. Lo strumento corregge i volt mostrati in base a questo valore. Con una sonda 10x, scegli 10x per leggere la tensione reale del circuito.',
  },
  coupling: {
    title: 'Accoppiamento',
    text: 'Sceglie se il segnale passa con la componente continua (DC) o senza (AC). In DC vedi tutto il segnale, in AC solo le variazioni. Con 5 V e piccole ondulazioni, usa AC per vederle.',
  },
  cursors: {
    title: 'Cursori',
    text: 'Mostra due linee che puoi spostare sul grafico per leggere tempi e tensioni con precisione. Servono a misurare la distanza tra due punti. Con un’onda a 1 kHz, posiziona i cursori su due fronti di salita per leggere il periodo di 1 ms.',
  },
  view: {
    title: 'Vista',
    text: 'Il selettore cambia ciò che mostra il grafico. La vista nel tempo mostra l’onda, lo spettro le frequenze presenti. Con un segnale a 1 kHz, lo spettro mostra un picco vicino a 1 kHz.',
  },
  fftWindow: {
    title: 'Finestra FFT',
    text: 'Sceglie la finestra che attenua i bordi del segnale prima dell’analisi delle frequenze. Ogni finestra bilancia precisione in frequenza e in ampiezza. Con un segnale puro, la finestra Flat-top dà l’ampiezza del picco più corretta.',
  },
  persistence: {
    title: 'Persistenza',
    text: 'Lascia sfumare le tracce precedenti invece di farle sparire di colpo. Mostra quanto un segnale varia nel tempo. Con un segnale che ha qualche oscillazione sui fronti, la persistenza mostra la fascia di variazione.',
  },
  measurements: {
    title: 'Misure',
    text: 'Mostra valori calcolati dal segnale, come tensione massima, frequenza e tempo di salita. Si aggiornano a ogni acquisizione. Con un’onda quadra a 1 kHz, il pannello indica una frequenza vicina a 1 kHz.',
  },
  vpp: {
    title: 'Tensione picco-picco',
    text: 'Indica la differenza tra il valore massimo e quello minimo del segnale. Serve a sapere quanto il segnale oscilla in totale. Con un’onda che va da 0 a 5 V, la misura segna circa 5 V.',
  },
  vrms: {
    title: 'Tensione efficace',
    text: 'Indica il valore efficace del segnale, cioè la tensione continua che produce lo stesso riscaldamento. Serve a valutare la potenza di un segnale alternato. Con una sinusoide di picco 1 V, il valore efficace è circa 0,71 V.',
  },
  vacRms: {
    title: 'Vac rms',
    text: 'Valore efficace della sola parte alternata, senza la componente continua.',
  },
  frequency: {
    title: 'Frequenza',
    text: 'Indica quante volte al secondo il segnale si ripete. Serve a verificare la frequenza di un oscillatore o di un PWM. Con un PWM impostato a 1 kHz, la misura mostra un valore vicino a 1 kHz.',
  },
  duty: {
    title: 'Duty cycle',
    text: 'Percentuale del periodo in cui il segnale resta a livello alto.',
  },
  riseTime: {
    title: 'Tempo di salita',
    text: 'Tempo che il fronte impiega per passare dal 10 % al 90 % dell’ampiezza. Più è breve, più il fronte è ripido.',
  },
  thd: {
    title: 'Distorsione armonica',
    text: 'Indica quanto il segnale si discosta da una sinusoide pura, in percentuale. Serve a verificare la qualità di un’onda generata da un amplificatore o da un oscillatore. Con una sinusoide pulita, il valore è vicino allo zero.',
  },
  phase: {
    title: 'Fase',
    text: 'Indica lo sfasamento tra due segnali con la stessa frequenza, in gradi. Serve a capire se un segnale è in ritardo rispetto all’altro. Con due onde a 1 kHz sfasate di un quarto di periodo, la misura mostra circa 90 gradi.',
  },
  sampleRate: {
    title: 'Frequenza di campionamento',
    text: 'Indica quanti campioni al secondo lo strumento prende dal segnale. Più campioni danno dettagli più fini, ma riempiono più in fretta la memoria. Con 1 MS/s, un’onda a 1 kHz ha circa mille campioni per ciclo.',
  },
  recordLength: {
    title: 'Lunghezza della registrazione',
    text: 'Indica quanti campioni vengono registrati in un’acquisizione. Un record lungo copre più tempo con più dettaglio, ma richiede più elaborazione. Lo strumento lo calcola in base alla base dei tempi e alla frequenza di campionamento.',
  },
  math: {
    title: 'Funzioni matematiche',
    text: 'Calcola una traccia M da CH1 e CH2: somma, differenza o prodotto. Derivata e integrale usano solo CH1. La traccia M compare sullo schermo come un canale in più.',
  },
  filter: {
    title: 'Filtro',
    text: 'Applica al canale un filtro passa-basso, passa-alto o passa-banda. Il passa-basso attenua il rumore veloce, il passa-alto la componente continua. Con un segnale rumoroso, abbassa la frequenza di taglio.',
  },
  classify: {
    title: 'Classificazione del segnale',
    text: 'Riconosce il tipo di segnale sul canale, per esempio continua, rumore, clock, PWM, seriale o burst. Indica anche i decoder più adatti e una stima della velocità. Con una linea seriale, la classe suggerisce il decoder da provare.',
  },
  recording: {
    title: 'Registrazione',
    text: 'Salva i frame acquisiti in un file CSV, NPY o WAV nella cartella h6022-recordings. Puoi fermarla a mano o dopo un numero di frame o di secondi. Durante la registrazione la barra in alto mostra REC.',
  },
  decoder: {
    title: 'Decoder',
    text: 'Decodifica una linea seriale UART, I2C o CAN e mostra i byte sotto il display. Scegli il canale, la soglia e la velocità della linea. Con un protocollo sbagliato le annotazioni risultano errate o assenti.',
  },
  mcp: {
    title: 'Server MCP',
    text: 'Il server MCP parte con l’app sulla porta 6022. Un client come Claude Code lo usa per leggere misure, acquisire e decodificare. Con il pallino verde il server è raggiungibile.',
  },
  device: {
    title: 'Dispositivo',
    text: 'Mostra la sorgente dei dati: Hantek 6022BE via USB oppure il simulatore. Il menu indica firmware e calibrazione. Se il dispositivo non compare, ricollega il cavo USB.',
  },
  trend: {
    title: 'Andamento della misura',
    text: 'Mostra come una misura cambia nel tempo, acquisizione dopo acquisizione. Serve a notare una deriva lenta che non si vede nella singola forma d’onda. Con la tensione di alimentazione che scende piano, l’andamento mostra la discesa.',
  },
  explain: {
    title: 'Spiegazioni',
    text: 'Attiva o disattiva queste spiegazioni. Con la modalità attiva, passa il puntatore su un controllo per leggere a cosa serve. Il tasto i fa lo stesso.',
  },
  status: {
    title: 'Stato acquisizione',
    text: 'Indica se l’acquisizione è in corso, ferma o in attesa di un trigger. In modo Normale, «In attesa» significa che nessun evento soddisfa la condizione di trigger.',
  },
  scope: {
    title: 'Display',
    text: 'Mostra le tracce in funzione del tempo. Passa il puntatore per leggere tempo e tensioni in quel punto. Trascina i marcatori a sinistra per spostare i canali e il triangolo T a destra per il livello di trigger.',
  },
  panelTabs: {
    title: 'Schede del pannello',
    text: 'Controlli raccoglie canali, base dei tempi, trigger e display. Decoder imposta la decodifica dei protocolli. Strumenti contiene registrazione, classificazione, matematica e filtro.',
  },
  logFrequency: {
    title: 'Asse logaritmico',
    text: 'Mostra l’asse delle frequenze dello spettro in scala logaritmica. Serve a vedere insieme armoniche basse e alte, per esempio da 50 Hz a 50 kHz.',
  },
  pulsePolarity: {
    title: 'Polarità impulso',
    text: 'Sceglie se misurare gli impulsi positivi, cioè sopra il livello, o quelli negativi, sotto il livello.',
  },
  pulseWidth: {
    title: 'Larghezza impulso',
    text: 'Il trigger scatta quando un impulso dura meno, più, dentro o fuori dai limiti impostati. Serve a catturare un impulso anomalo in un treno PWM.',
  },
  runt: {
    title: 'Trigger runt',
    text: 'Scatta su un impulso che supera la soglia bassa ma non raggiunge quella alta. Serve a trovare impulsi deboli in un segnale digitale, per esempio per un’alimentazione instabile.',
  },
  windowTrigger: {
    title: 'Trigger a finestra',
    text: 'Scatta quando il segnale entra o esce dalla fascia tra soglia bassa e soglia alta. Serve a sorvegliare una tensione che deve restare in un intervallo.',
  },
  timeoutTrigger: {
    title: 'Trigger timeout',
    text: 'Scatta quando il segnale non presenta il fronte scelto per il tempo impostato. Serve a catturare un clock che si ferma o un bus che resta inattivo.',
  },
  patternTrigger: {
    title: 'Trigger a pattern',
    text: 'Scatta quando entrambi i canali assumono la combinazione di livelli alto e basso scelta. Il valore «qualsiasi» esclude il canale dal confronto.',
  },
  logicTrigger: {
    title: 'Trigger logico',
    text: 'Ogni canale ha una condizione: fronte, alto, basso o ignora. AND scatta quando tutte sono vere, OR quando ne basta una. Serve a catturare un evento che può comparire su uno dei due canali.',
  },
  otherLevel: {
    title: 'Livello altro canale',
    text: 'Soglia del canale che non è la sorgente. Ogni canale ha così il proprio livello nei trigger logico e A → B.',
  },
  sequenceTrigger: {
    title: 'Trigger A → B',
    text: 'Il fronte A sul canale sorgente arma il trigger. Il frame scatta al primo fronte B sull’altro canale, che diventa il tempo zero. Serve a misurare una causa e il suo effetto.',
  },
  sequenceDelay: {
    title: 'Ritardo B',
    text: 'Tempo minimo tra il fronte A e il fronte B. I fronti B che arrivano prima vengono ignorati. Con 0 vale il primo fronte B dopo A.',
  },
  vmax: {
    title: 'Vmax',
    text: 'Tensione più alta nella finestra visibile. Include picchi e disturbi.',
  },
  vmin: {
    title: 'Vmin',
    text: 'Tensione più bassa nella finestra visibile. Include picchi e disturbi.',
  },
  vmean: {
    title: 'Vmedia',
    text: 'Valore medio della tensione nella finestra. Corrisponde alla componente continua del segnale.',
  },
  vtop: {
    title: 'Vtop',
    text: 'Livello alto stabile del segnale, calcolato dall’istogramma delle tensioni. Esclude overshoot e picchi, a differenza di Vmax.',
  },
  vbase: {
    title: 'Vbase',
    text: 'Livello basso stabile del segnale, calcolato dall’istogramma delle tensioni. Esclude undershoot e picchi, a differenza di Vmin.',
  },
  vamp: {
    title: 'Ampiezza',
    text: 'Differenza tra Vtop e Vbase. Per un’onda quadra è l’escursione reale tra i due livelli, senza i picchi.',
  },
  fallTime: {
    title: 'Tempo di discesa',
    text: 'Tempo che il segnale impiega a scendere dal 90 % al 10 % dell’ampiezza. Serve a valutare la velocità di un’uscita digitale.',
  },
  overshoot: {
    title: 'Overshoot',
    text: 'Quanto il segnale supera Vtop dopo un fronte, in percentuale dell’ampiezza. Un valore alto indica risonanze o un carico non adattato.',
  },
  period: {
    title: 'Periodo',
    text: 'Durata di un ciclo completo del segnale, cioè l’inverso della frequenza. Mostra «—» se la finestra non contiene un periodo intero.',
  },
  pulseWidthMeasure: {
    title: 'Larghezza impulso',
    text: 'Durata della parte alta (+) e della parte bassa (−) di un ciclo, misurata a metà ampiezza.',
  },
  edges: {
    title: 'Fronti',
    text: 'Numero di fronti di salita e di discesa nella finestra visibile.',
  },
  delay: {
    title: 'Ritardo CH1–CH2',
    text: 'Tempo tra un fronte di CH1 e il fronte corrispondente di CH2. Serve a misurare la latenza di un circuito.',
  },
  decoderProtocol: {
    title: 'Protocollo',
    text: 'Sceglie il protocollo da decodificare, per esempio UART, I2C o CAN. I parametri sotto cambiano in base al protocollo.',
  },
  decoderChannel: {
    title: 'Canale del decoder',
    text: 'Sceglie il canale che porta il segnale da decodificare.',
  },
  decoderThreshold: {
    title: 'Soglia del decoder',
    text: 'Tensione che separa lo 0 dall’1 logico. Con la soglia automatica il decoder usa il punto medio tra i livelli alto e basso del segnale.',
  },
  engine: {
    title: 'Motore',
    text: 'Sorgente dei dati e numero di frame al secondo mostrati. La frequenza di campionamento misurata indica i campioni che arrivano davvero dal dispositivo.',
  },
}

const HELP_BY_LOCALE: Record<Locale, Record<HelpId, HelpText>> = { it: HELP, en: HELP_EN, fr: HELP_FR, es: HELP_ES }

/** Spiegazione nella lingua corrente; in italiano se la traduzione manca. Undefined per un id sconosciuto. */
export function helpText(id: HelpId): HelpText | undefined {
  return HELP_BY_LOCALE[getLocale()][id] ?? HELP[id]
}
