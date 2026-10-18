//! Spiegazioni dei controlli. Il tasto `i` le attiva; compaiono al passaggio del puntatore.

use eframe::egui::Response;

use crate::state::UiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HelpId {
    Run,
    Stop,
    Single,
    TriggerMode,
    TriggerSource,
    TriggerKind,
    TriggerLevel,
    TriggerSlope,
    Hysteresis,
    Holdoff,
    Pretrigger,
    TimePerDiv,
    HorizontalOffset,
    ChannelEnable,
    VoltsPerDiv,
    VerticalOffset,
    Probe,
    Coupling,
    Cursors,
    View,
    FftWindow,
    Persistence,
    Measurements,
    Vpp,
    Vrms,
    Frequency,
    Duty,
    RiseTime,
    Thd,
    Phase,
    SampleRate,
    RecordLength,
    DecodeProtocol,
    DecodeChannel,
    DecodeThreshold,
    DecodeUartBaud,
    DecodeUartFormat,
    DecodeInverted,
    DecodeCanBitrate,
}

const ALL: &[HelpId] = &[
    HelpId::Run,
    HelpId::Stop,
    HelpId::Single,
    HelpId::TriggerMode,
    HelpId::TriggerSource,
    HelpId::TriggerKind,
    HelpId::TriggerLevel,
    HelpId::TriggerSlope,
    HelpId::Hysteresis,
    HelpId::Holdoff,
    HelpId::Pretrigger,
    HelpId::TimePerDiv,
    HelpId::HorizontalOffset,
    HelpId::ChannelEnable,
    HelpId::VoltsPerDiv,
    HelpId::VerticalOffset,
    HelpId::Probe,
    HelpId::Coupling,
    HelpId::Cursors,
    HelpId::View,
    HelpId::FftWindow,
    HelpId::Persistence,
    HelpId::Measurements,
    HelpId::Vpp,
    HelpId::Vrms,
    HelpId::Frequency,
    HelpId::Duty,
    HelpId::RiseTime,
    HelpId::Thd,
    HelpId::Phase,
    HelpId::SampleRate,
    HelpId::RecordLength,
    HelpId::DecodeProtocol,
    HelpId::DecodeChannel,
    HelpId::DecodeThreshold,
    HelpId::DecodeUartBaud,
    HelpId::DecodeUartFormat,
    HelpId::DecodeInverted,
    HelpId::DecodeCanBitrate,
];

impl HelpId {
    pub fn all() -> &'static [HelpId] {
        ALL
    }

    /// Titolo breve.
    pub fn title(self) -> &'static str {
        match self {
            HelpId::Run => "Avvia",
            HelpId::Stop => "Ferma",
            HelpId::Single => "Singola acquisizione",
            HelpId::TriggerMode => "Modo trigger",
            HelpId::TriggerSource => "Sorgente trigger",
            HelpId::TriggerKind => "Tipo di trigger",
            HelpId::TriggerLevel => "Livello trigger",
            HelpId::TriggerSlope => "Pendenza trigger",
            HelpId::Hysteresis => "Isteresi",
            HelpId::Holdoff => "Attesa trigger",
            HelpId::Pretrigger => "Pre-trigger",
            HelpId::TimePerDiv => "Base dei tempi",
            HelpId::HorizontalOffset => "Spostamento orizzontale",
            HelpId::ChannelEnable => "Attiva canale",
            HelpId::VoltsPerDiv => "Volt per divisione",
            HelpId::VerticalOffset => "Spostamento verticale",
            HelpId::Probe => "Sonda",
            HelpId::Coupling => "Accoppiamento",
            HelpId::Cursors => "Cursori",
            HelpId::View => "Vista",
            HelpId::FftWindow => "Finestra FFT",
            HelpId::Persistence => "Persistenza",
            HelpId::Measurements => "Misure",
            HelpId::Vpp => "Tensione picco-picco",
            HelpId::Vrms => "Tensione efficace",
            HelpId::Frequency => "Frequenza",
            HelpId::Duty => "Duty cycle",
            HelpId::RiseTime => "Tempo di salita",
            HelpId::Thd => "Distorsione armonica",
            HelpId::Phase => "Fase",
            HelpId::SampleRate => "Frequenza di campionamento",
            HelpId::RecordLength => "Lunghezza della registrazione",
            HelpId::DecodeProtocol => "Protocollo del decoder",
            HelpId::DecodeChannel => "Canale del decoder",
            HelpId::DecodeThreshold => "Soglia del decoder",
            HelpId::DecodeUartBaud => "Velocità UART",
            HelpId::DecodeUartFormat => "Formato UART",
            HelpId::DecodeInverted => "Linea invertita",
            HelpId::DecodeCanBitrate => "Bitrate CAN",
        }
    }

    /// Spiegazione in italiano: cosa fa, quando usarlo, un esempio pratico.
    pub fn text(self) -> &'static str {
        match self {
            HelpId::Run => {
                "Il pulsante avvia l'acquisizione e aggiorna la vista in continuo. \
                 Serve a seguire un segnale che cambia nel tempo. \
                 Con un segnale PWM a 1 kHz vedi la forma ripetersi a ogni ciclo."
            }
            HelpId::Stop => {
                "Il pulsante ferma l'acquisizione e lascia l'ultima forma sullo schermo. \
                 Serve a esaminare il segnale senza che si muova. \
                 Con un fronte troppo veloce per essere letto, fermati su di esso e osservalo con calma."
            }
            HelpId::Single => {
                "Il pulsante cattura una sola acquisizione quando il trigger scatta, poi si ferma. \
                 Serve a registrare un evento che capita una volta sola, come l'accensione di un motore. \
                 Imposta prima il trigger sul fronte giusto."
            }
            HelpId::TriggerMode => {
                "Il modo decide quando la vista si aggiorna. Automatico mostra il segnale anche senza eventi. \
                 Normale aspetta un evento che supera il livello, così la forma resta ferma. \
                 Con un segnale costante, il modo normale è la scelta più stabile."
            }
            HelpId::TriggerSource => {
                "Sceglie il canale che fa scattare il trigger. Il trigger è il punto da cui parte ogni acquisizione. \
                 Con un segnale sul canale 1 e un riferimento sul canale 2, scegli il canale 1 per allineare la forma."
            }
            HelpId::TriggerKind => {
                "Sceglie che cosa fa scattare il trigger, per esempio un fronte o una larghezza d'impulso. \
                 Il fronte è la scelta più semplice. \
                 Con un'onda quadra a 1 kHz, il trigger sul fronte mostra ogni periodo nello stesso punto."
            }
            HelpId::TriggerLevel => {
                "Imposta la tensione che il segnale deve superare per far scattare il trigger. \
                 Serve a scegliere a quale altezza l'onda viene allineata. \
                 Con un segnale tra 0 e 3,3 V, un livello di 1,5 V ferma la forma al centro della salita."
            }
            HelpId::TriggerSlope => {
                "Sceglie se il trigger scatta sul fronte di salita, su quello di discesa o su entrambi. \
                 Il fronte di salita è il passaggio da basso ad alto. \
                 Con un'onda quadra a 1 kHz, il fronte di salita dà un'immagine stabile di ogni periodo."
            }
            HelpId::Hysteresis => {
                "Impone una piccola fascia di tolleranza intorno al livello del trigger. \
                 Evita che il rumore faccia scattare il trigger più volte. \
                 Con un segnale rumoroso, aumenta l'isteresi finché la forma si stabilizza."
            }
            HelpId::Holdoff => {
                "Il tempo di attesa blocca il trigger per un intervallo dopo ogni scatto. \
                 Serve con un segnale a burst, cioè con impulsi ravvicinati. \
                 Con un'attesa più lunga della durata del burst, il trigger aggancia sempre il primo impulso."
            }
            HelpId::Pretrigger => {
                "Mostra una parte del segnale prima del trigger, così vedi che cosa succede prima dell'evento. \
                 Serve quando cerchi la causa di un guasto o di un disturbo. \
                 Con un impulso anomalo a 10 ms, il pre-trigger mostra il tratto che lo precede."
            }
            HelpId::TimePerDiv => {
                "Imposta quanto tempo rappresenta una divisione dello schermo. \
                 Un valore piccolo mostra dettagli veloci, un valore grande mostra un intervallo lungo. \
                 Con un segnale PWM a 1 kHz, 200 µs/div mostra due periodi."
            }
            HelpId::HorizontalOffset => {
                "Sposta la vista nel tempo, verso destra o verso sinistra. \
                 Serve per vedere una parte del segnale che non compare sullo schermo. \
                 Con un evento 5 ms dopo il trigger, sposta la vista di 5 ms per riportarlo al centro."
            }
            HelpId::ChannelEnable => {
                "Mostra o nasconde la traccia di un canale. \
                 Nascondere un canale libera lo schermo quando guardi l'altro. \
                 Con due segnali collegati, attiva solo il canale 1 per osservarlo meglio."
            }
            HelpId::VoltsPerDiv => {
                "Imposta quanta tensione rappresenta una divisione verticale. \
                 Un valore piccolo ingrandisce i segnali deboli, un valore grande fa stare i segnali forti sullo schermo. \
                 Con un segnale da 0 a 3 V, 1 V/div lo mostra per intero."
            }
            HelpId::VerticalOffset => {
                "Sposta verso l'alto o verso il basso la posizione dello zero del canale. \
                 Serve a separare due tracce che si sovrappongono. \
                 Con due segnali uguali, sposta uno di due divisioni per vederli entrambi."
            }
            HelpId::Probe => {
                "Indica il rapporto della sonda collegata, per esempio 1x o 10x. \
                 Lo strumento corregge i volt mostrati in base a questo valore. \
                 Con una sonda 10x, scegli 10x per leggere la tensione reale del circuito."
            }
            HelpId::Coupling => {
                "Decide se il segnale passa con la sua componente continua o senza. \
                 Con l'accoppiamento in continua vedi tutto il segnale, con quello in alternata solo le variazioni. \
                 Con un segnale di 5 V con piccole ondulazioni, usa l'alternata per vedere le ondulazioni."
            }
            HelpId::Cursors => {
                "Mostra due linee che puoi spostare sul grafico per leggere tempi e tensioni con precisione. \
                 Servono a misurare la distanza tra due punti. \
                 Con un'onda a 1 kHz, posiziona i cursori su due fronti di salita per leggere il periodo di 1 ms."
            }
            HelpId::View => {
                "Il selettore cambia ciò che mostra il grafico. \
                 La vista nel tempo mostra l'onda, lo spettro le frequenze presenti. \
                 Con un segnale a 1 kHz, lo spettro mostra un picco vicino a 1 kHz."
            }
            HelpId::FftWindow => {
                "Sceglie la finestra che attenua i bordi del segnale prima dell'analisi delle frequenze. \
                 Ogni finestra bilancia precisione in frequenza e in ampiezza. \
                 Con un segnale puro, la finestra Flat-top dà l'ampiezza del picco più corretta."
            }
            HelpId::Persistence => {
                "Lascia sfumare le tracce precedenti invece di farle sparire di colpo. \
                 Mostra quanto un segnale varia nel tempo. \
                 Con un segnale che ha qualche oscillazione sui fronti, la persistenza mostra la fascia di variazione."
            }
            HelpId::Measurements => {
                "Mostra valori calcolati dal segnale, come tensione massima, frequenza e tempo di salita. \
                 Si aggiornano a ogni acquisizione. \
                 Con un'onda quadra a 1 kHz, il pannello indica una frequenza vicina a 1 kHz."
            }
            HelpId::Vpp => {
                "Indica la differenza tra il valore massimo e quello minimo del segnale. \
                 Serve a sapere quanto il segnale oscilla in totale. \
                 Con un'onda che va da 0 a 5 V, la misura segna circa 5 V."
            }
            HelpId::Vrms => {
                "Indica il valore efficace del segnale, cioè la tensione continua che produce lo stesso riscaldamento. \
                 Serve a valutare la potenza di un segnale alternato. \
                 Con una sinusoide di picco 1 V, il valore efficace è circa 0,71 V."
            }
            HelpId::Frequency => {
                "Indica quante volte al secondo il segnale si ripete. \
                 Serve a verificare la frequenza di un oscillatore o di un PWM. \
                 Con un PWM impostato a 1 kHz, la misura mostra un valore vicino a 1 kHz."
            }
            HelpId::Duty => {
                "Indica la percentuale del periodo in cui il segnale resta sopra il livello medio. \
                 Serve a controllare la larghezza degli impulsi di un PWM. \
                 Con un PWM al 25%, la misura mostra un valore vicino al 25%."
            }
            HelpId::RiseTime => {
                "Indica quanto tempo impiega il segnale a passare dal 10% al 90% dell'ampiezza nel fronte di salita. \
                 Serve a valutare quanto è veloce un passaggio. \
                 Con un fronte che sale in 2 µs, la misura mostra un valore vicino a 2 µs."
            }
            HelpId::Thd => {
                "Indica quanto il segnale si discosta da una sinusoide pura, in percentuale. \
                 Serve a verificare la qualità di un'onda generata da un amplificatore o da un oscillatore. \
                 Con una sinusoide pulita, il valore è vicino allo zero."
            }
            HelpId::Phase => {
                "Indica lo sfasamento tra due segnali con la stessa frequenza, in gradi. \
                 Serve a capire se un segnale è in ritardo rispetto all'altro. \
                 Con due onde a 1 kHz sfasate di un quarto di periodo, la misura mostra circa 90 gradi."
            }
            HelpId::SampleRate => {
                "Indica quanti campioni al secondo lo strumento prende dal segnale. \
                 Più campioni danno dettagli più fini, ma riempiono più in fretta la memoria. \
                 Con 1 MS/s, un'onda a 1 kHz ha circa mille campioni per ciclo."
            }
            HelpId::RecordLength => {
                "Indica quanti campioni vengono registrati in un'acquisizione. \
                 Un record lungo copre più tempo con più dettaglio, ma richiede più elaborazione. \
                 Lo strumento lo calcola in base alla base dei tempi e alla frequenza di campionamento."
            }
            HelpId::DecodeProtocol => {
                "Sceglie il protocollo da cercare nel segnale, come UART, I2C o CAN. \
                 Con Attiva le annotazioni compaiono sotto il display. \
                 Con un protocollo sbagliato le annotazioni risultano errate o assenti, quindi cambialo finché i dati hanno senso."
            }
            HelpId::DecodeChannel => {
                "Sceglie il canale da decodificare. Con I2C il canale scelto è SCL e l'altro è SDA. \
                 Con un protocollo a un solo filo, usa il canale su cui arriva il segnale. \
                 Un canale spento non può essere decodificato."
            }
            HelpId::DecodeThreshold => {
                "Imposta la tensione che separa il livello alto da quello basso. \
                 Con la soglia automatica il decoder la stima dal segnale. \
                 Con un segnale rumoroso o di ampiezza ridotta, indica tu il valore in volt."
            }
            HelpId::DecodeUartBaud => {
                "Imposta la velocità della linea in bit al secondo. \
                 Con il baud automatico il decoder la stima dalla durata degli impulsi più brevi. \
                 Se conosci la velocità del dispositivo, indicala: i bit vengono campionati nel punto giusto."
            }
            HelpId::DecodeUartFormat => {
                "Descrive come è composto ogni carattere: bit dati, parità e bit di stop. \
                 Valori sbagliati producono errori di framing o di parità. \
                 Il formato più comune è 8 bit dati, nessuna parità e 1 bit di stop."
            }
            HelpId::DecodeInverted => {
                "Inverte il livello logico della linea, così il riposo risulta basso invece che alto. \
                 Serve con segnali RS-232 o con linee passate attraverso un transistor. \
                 Se tutti i byte risultano errati, prova a invertire la linea."
            }
            HelpId::DecodeCanBitrate => {
                "Imposta la velocità del bus CAN in bit al secondo, per esempio 500 kbit/s. \
                 Deve essere uguale a quella di tutti i nodi della rete. \
                 Con una velocità sbagliata il decoder non trova frame validi."
            }
        }
    }
}

/// Con `state.help` attivo, mostra titolo e spiegazione al passaggio del puntatore.
pub fn attach(response: Response, id: HelpId, state: &UiState) -> Response {
    if !state.help {
        return response;
    }
    response.on_hover_ui(|ui| {
        ui.set_max_width(320.0);
        ui.strong(id.title());
        ui.label(id.text());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_every_variant_once() {
        let all = HelpId::all();
        assert_eq!(all.len(), 39);
        for (i, id) in all.iter().enumerate() {
            assert!(!all[i + 1..].contains(id), "duplicato: {id:?}");
        }
    }

    #[test]
    fn every_title_and_text_is_present_and_short() {
        for id in HelpId::all() {
            assert!(!id.title().is_empty(), "titolo vuoto: {id:?}");
            assert!(!id.text().is_empty(), "testo vuoto: {id:?}");
            assert!(id.text().chars().count() < 500, "testo troppo lungo: {id:?}");
        }
    }
}
