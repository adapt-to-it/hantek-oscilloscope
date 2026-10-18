// Spiegazioni dei controlli in spagnolo. Stesse chiavi e stesso limite di 250 caratteri di texts.ts.
import type { HelpId, HelpText } from './texts'

export const HELP_ES: Record<HelpId, HelpText> = {
  run: {
    title: 'Iniciar',
    text: 'El botón inicia la adquisición y actualiza la vista de forma continua. Sirve para seguir una señal que cambia con el tiempo. Con una señal PWM de 1 kHz ves la forma repetirse en cada ciclo.',
  },
  stop: {
    title: 'Detener',
    text: 'El botón detiene la adquisición y deja la última forma en pantalla. Sirve para examinar la señal sin que se mueva. Con un flanco demasiado rápido para leerlo, detente en él y obsérvalo con calma.',
  },
  single: {
    title: 'Adquisición única',
    text: 'El botón captura una sola adquisición cuando el trigger se dispara, y después se detiene. Sirve para registrar un evento que ocurre una sola vez, como el encendido de un motor. Configura antes el trigger en el flanco correcto.',
  },
  triggerMode: {
    title: 'Modo de trigger',
    text: 'El modo decide cuándo se actualiza la vista. Automático muestra la señal incluso sin eventos. Normal espera un evento que supere el nivel, así la forma permanece quieta.',
  },
  triggerSource: {
    title: 'Fuente de trigger',
    text: 'Elige el canal que dispara el trigger. El trigger es el punto desde el que empieza cada adquisición. Con una señal en el canal 1 y una referencia en el canal 2, elige el canal 1 para alinear la forma.',
  },
  triggerKind: {
    title: 'Tipo de trigger',
    text: 'Elige qué dispara el trigger, por ejemplo un flanco o una anchura de pulso. El flanco es la opción más sencilla. Con una onda cuadrada de 1 kHz, el trigger por flanco muestra cada periodo en el mismo punto.',
  },
  triggerLevel: {
    title: 'Nivel de trigger',
    text: 'Tensión umbral que la señal debe cruzar para disparar el trigger. También puedes arrastrar el triángulo a la derecha de la pantalla.',
  },
  triggerSlope: {
    title: 'Pendiente de trigger',
    text: 'Elige si el trigger se dispara en el flanco de subida, en el de bajada o en ambos. El flanco de subida es el paso de bajo a alto. Con una onda cuadrada de 1 kHz, el flanco de subida da una imagen estable de cada periodo.',
  },
  hysteresis: {
    title: 'Histéresis',
    text: 'Margen alrededor del nivel de trigger. La señal debe superarlo antes de que se acepte un nuevo trigger, así el ruido no causa disparos múltiples.',
  },
  holdoff: {
    title: 'Espera del trigger',
    text: 'Tiempo mínimo entre dos triggers consecutivos. Útil con señales en ráfaga o con muchos flancos juntos, para enganchar siempre el mismo punto.',
  },
  pretrigger: {
    title: 'Pre-trigger',
    text: 'Muestra una parte de la señal anterior al trigger, para ver qué ocurre antes del evento. Sirve cuando buscas la causa de una avería o de una perturbación. Con un pulso anómalo de 10 ms, el pre-trigger muestra el tramo que lo precede.',
  },
  timePerDiv: {
    title: 'Base de tiempos',
    text: 'Ajusta cuánto tiempo representa una división de la pantalla. Un valor pequeño muestra detalles rápidos, uno grande muestra un intervalo largo. Con una señal PWM de 1 kHz, 200 µs/div muestra dos periodos.',
  },
  horizontalOffset: {
    title: 'Desplazamiento horizontal',
    text: 'Desplaza la vista en el tiempo, hacia la derecha o hacia la izquierda. Sirve para ver una parte de la señal que no aparece en pantalla. Con un evento 5 ms después del trigger, desplaza la vista 5 ms para devolverlo al centro.',
  },
  channelEnable: {
    title: 'Activar canal',
    text: 'Muestra u oculta la traza de un canal. Ocultar un canal libera la pantalla cuando miras el otro. Con dos señales conectadas, activa solo el canal 1 para observarlo mejor.',
  },
  voltsPerDiv: {
    title: 'Voltios por división',
    text: 'Ajusta cuánta tensión representa una división vertical. Un valor pequeño amplía las señales débiles, uno grande hace que las señales fuertes quepan en pantalla. Con una señal de 0 a 3 V, 1 V/div la muestra completa.',
  },
  verticalOffset: {
    title: 'Desplazamiento vertical',
    text: 'Desplaza hacia arriba o hacia abajo la posición del cero del canal. Sirve para separar dos trazas que se superponen. Con dos señales iguales, desplaza una dos divisiones para verlas ambas.',
  },
  probe: {
    title: 'Sonda',
    text: 'Indica la relación de la sonda conectada, por ejemplo 1x o 10x. El instrumento corrige los voltios mostrados según este valor. Con una sonda 10x, elige 10x para leer la tensión real del circuito.',
  },
  coupling: {
    title: 'Acoplamiento',
    text: 'Elige si la señal pasa con la componente continua (DC) o sin ella (AC). En DC ves toda la señal, en AC solo las variaciones. Con 5 V y pequeños rizados, usa AC para verlos.',
  },
  cursors: {
    title: 'Cursores',
    text: 'Muestra dos líneas móviles sobre el gráfico para leer tiempos y tensiones con precisión. Sirven para medir la distancia entre dos puntos. Con una onda de 1 kHz, coloca los cursores en dos flancos de subida para leer el periodo de 1 ms.',
  },
  view: {
    title: 'Vista',
    text: 'El selector cambia lo que muestra el gráfico. La vista temporal muestra la onda, el espectro las frecuencias presentes. Con una señal de 1 kHz, el espectro muestra un pico cerca de 1 kHz.',
  },
  fftWindow: {
    title: 'Ventana FFT',
    text: 'Elige la ventana que atenúa los bordes de la señal antes del análisis de frecuencias. Cada ventana equilibra precisión en frecuencia y en amplitud. Con una señal pura, la ventana Flat-top da la amplitud del pico más correcta.',
  },
  persistence: {
    title: 'Persistencia',
    text: 'Deja desvanecerse las trazas anteriores en lugar de borrarlas de golpe. Muestra cuánto varía la señal con el tiempo. Con una señal que tiene algunas oscilaciones en los flancos, la persistencia muestra la banda de variación.',
  },
  measurements: {
    title: 'Medidas',
    text: 'Muestra valores calculados a partir de la señal, como la tensión máxima, la frecuencia y el tiempo de subida. Se actualizan en cada adquisición. Con una onda cuadrada de 1 kHz, el panel indica una frecuencia cercana a 1 kHz.',
  },
  vpp: {
    title: 'Tensión pico a pico',
    text: 'Indica la diferencia entre el valor máximo y el mínimo de la señal. Sirve para saber cuánto oscila la señal en total. Con una onda que va de 0 a 5 V, la medida marca unos 5 V.',
  },
  vrms: {
    title: 'Tensión eficaz',
    text: 'Indica el valor eficaz de la señal, es decir, la tensión continua que produce el mismo calentamiento. Sirve para evaluar la potencia de una señal alterna. Con una sinusoide de pico 1 V, el valor eficaz es unos 0,71 V.',
  },
  vacRms: {
    title: 'Vac rms',
    text: 'Valor eficaz de la sola parte alterna, sin la componente continua.',
  },
  frequency: {
    title: 'Frecuencia',
    text: 'Indica cuántas veces por segundo se repite la señal. Sirve para verificar la frecuencia de un oscilador o de un PWM. Con un PWM configurado a 1 kHz, la medida muestra un valor cercano a 1 kHz.',
  },
  duty: {
    title: 'Duty cycle',
    text: 'Porcentaje del periodo en que la señal permanece en nivel alto.',
  },
  riseTime: {
    title: 'Tiempo de subida',
    text: 'Tiempo que tarda el flanco en pasar del 10 % al 90 % de la amplitud. Cuanto más corto, más empinado es el flanco.',
  },
  thd: {
    title: 'Distorsión armónica',
    text: 'Indica cuánto se aleja la señal de una sinusoide pura, en porcentaje. Sirve para verificar la calidad de una onda generada por un amplificador o un oscilador. Con una sinusoide limpia, el valor está cerca de cero.',
  },
  phase: {
    title: 'Fase',
    text: 'Indica el desfase entre dos señales de la misma frecuencia, en grados. Sirve para saber si una señal va retrasada respecto a la otra. Con dos ondas de 1 kHz desfasadas un cuarto de periodo, la medida muestra unos 90 grados.',
  },
  sampleRate: {
    title: 'Frecuencia de muestreo',
    text: 'Indica cuántas muestras por segundo toma el instrumento de la señal. Más muestras dan detalles más finos, pero llenan la memoria más deprisa. Con 1 MS/s, una onda de 1 kHz tiene unas mil muestras por ciclo.',
  },
  recordLength: {
    title: 'Longitud del registro',
    text: 'Indica cuántas muestras se registran en una adquisición. Un registro largo cubre más tiempo con más detalle, pero requiere más procesamiento. El instrumento lo calcula según la base de tiempos y la frecuencia de muestreo.',
  },
  math: {
    title: 'Funciones matemáticas',
    text: 'Calcula una traza M a partir de CH1 y CH2: suma, diferencia o producto. La derivada y la integral usan solo CH1. La traza M aparece en pantalla como un canal más.',
  },
  filter: {
    title: 'Filtro',
    text: 'Aplica al canal un filtro paso bajo, paso alto o paso banda. El paso bajo atenúa el ruido rápido, el paso alto la componente continua. Con una señal ruidosa, baja la frecuencia de corte.',
  },
  classify: {
    title: 'Clasificación de la señal',
    text: 'Reconoce el tipo de señal del canal: continua, ruido, reloj, PWM, serie o ráfaga. Indica los decodificadores más adecuados y una estimación de la velocidad. Con una línea serie, la clase sugiere qué decodificador probar.',
  },
  recording: {
    title: 'Grabación',
    text: 'Guarda los frames adquiridos en un archivo CSV, NPY o WAV dentro de la carpeta h6022-recordings. Puedes detenerla a mano o tras un número de frames o de segundos. Durante la grabación, la barra superior muestra REC.',
  },
  decoder: {
    title: 'Decodificador',
    text: 'Decodifica una línea serie UART, I2C o CAN y muestra los bytes bajo la pantalla. Elige el canal, el umbral y la velocidad de la línea. Con un protocolo equivocado, las anotaciones salen erróneas o no aparecen.',
  },
  mcp: {
    title: 'Servidor MCP',
    text: 'El servidor MCP arranca con la aplicación en el puerto 6022. Un cliente como Claude Code lo usa para leer medidas, adquirir y decodificar. Con el punto verde, el servidor es accesible.',
  },
  device: {
    title: 'Dispositivo',
    text: 'Muestra el origen de los datos: Hantek 6022BE por USB o el simulador. El menú indica firmware y calibración. Si el dispositivo no aparece, vuelve a conectar el cable USB.',
  },
  trend: {
    title: 'Tendencia de la medida',
    text: 'Muestra cómo cambia una medida con el tiempo, adquisición tras adquisición. Sirve para notar una deriva lenta que no se ve en la forma de onda individual. Con una alimentación que baja poco a poco, la tendencia muestra la caída.',
  },
  explain: {
    title: 'Explicaciones',
    text: 'Activa o desactiva estas explicaciones. Con el modo activo, pasa el puntero sobre un control para leer para qué sirve. La tecla i hace lo mismo.',
  },
  status: {
    title: 'Estado de la adquisición',
    text: 'Indica si la adquisición está en curso, detenida o esperando un trigger. En modo Normal, «En espera» significa que ningún evento cumple la condición de trigger.',
  },
  scope: {
    title: 'Pantalla',
    text: 'Muestra las trazas en función del tiempo. Pasa el puntero para leer tiempo y tensiones en ese punto. Arrastra los marcadores de la izquierda para mover los canales y el triángulo T de la derecha para el nivel de trigger.',
  },
  panelTabs: {
    title: 'Pestañas del panel',
    text: 'Controles reúne canales, base de tiempos, trigger y pantalla. Decoder configura la decodificación de protocolos. Herramientas contiene grabación, clasificación, matemáticas y filtro.',
  },
  logFrequency: {
    title: 'Eje logarítmico',
    text: 'Muestra el eje de frecuencias del espectro en escala logarítmica. Sirve para ver juntos armónicos bajos y altos, por ejemplo de 50 Hz a 50 kHz.',
  },
  pulsePolarity: {
    title: 'Polaridad del impulso',
    text: 'Elige si medir los impulsos positivos, es decir, por encima del nivel, o los negativos, por debajo del nivel.',
  },
  pulseWidth: {
    title: 'Anchura de impulso',
    text: 'El trigger se dispara cuando un impulso dura menos, más, dentro o fuera de los límites configurados. Sirve para capturar un impulso anómalo en un tren PWM.',
  },
  runt: {
    title: 'Trigger runt',
    text: 'Se dispara con un impulso que supera el umbral bajo pero no alcanza el alto. Sirve para encontrar impulsos débiles en una señal digital, por ejemplo en una alimentación inestable.',
  },
  windowTrigger: {
    title: 'Trigger de ventana',
    text: 'Se dispara cuando la señal entra o sale de la banda entre el umbral bajo y el umbral alto. Sirve para vigilar una tensión que debe permanecer dentro de un intervalo.',
  },
  timeoutTrigger: {
    title: 'Trigger de timeout',
    text: 'Se dispara cuando la señal no presenta el flanco elegido durante el tiempo configurado. Sirve para capturar un reloj que se para o un bus que permanece inactivo.',
  },
  patternTrigger: {
    title: 'Trigger de patrón',
    text: 'Se dispara cuando ambos canales adoptan la combinación de niveles alto y bajo elegida. El valor «cualquiera» excluye el canal de la comparación.',
  },
  logicTrigger: {
    title: 'Trigger lógico',
    text: 'Cada canal tiene una condición: flanco, alto, bajo o ignorar. AND se dispara cuando todas son verdaderas, OR cuando basta una. Sirve para capturar un evento que puede aparecer en uno de los dos canales.',
  },
  otherLevel: {
    title: 'Nivel del otro canal',
    text: 'Umbral del canal que no es la fuente. Así cada canal tiene su propio nivel en los triggers lógico y A → B.',
  },
  sequenceTrigger: {
    title: 'Trigger A → B',
    text: 'El flanco A en el canal fuente arma el trigger. La adquisición se dispara en el primer flanco B del otro canal, que pasa a ser el tiempo cero. Sirve para medir una causa y su efecto.',
  },
  sequenceDelay: {
    title: 'Retardo B',
    text: 'Tiempo mínimo entre el flanco A y el flanco B. Los flancos B que llegan antes se ignoran. Con 0 vale el primer flanco B después de A.',
  },
  vmax: {
    title: 'Vmax',
    text: 'Tensión más alta en la ventana visible. Incluye picos y perturbaciones.',
  },
  vmin: {
    title: 'Vmin',
    text: 'Tensión más baja en la ventana visible. Incluye picos y perturbaciones.',
  },
  vmean: {
    title: 'Vmedia',
    text: 'Valor medio de la tensión en la ventana. Corresponde a la componente continua de la señal.',
  },
  vtop: {
    title: 'Vtop',
    text: 'Nivel alto estable de la señal, calculado con el histograma de tensiones. Excluye overshoot y picos, a diferencia de Vmax.',
  },
  vbase: {
    title: 'Vbase',
    text: 'Nivel bajo estable de la señal, calculado con el histograma de tensiones. Excluye undershoot y picos, a diferencia de Vmin.',
  },
  vamp: {
    title: 'Amplitud',
    text: 'Diferencia entre Vtop y Vbase. Para una onda cuadrada es la excursión real entre los dos niveles, sin los picos.',
  },
  fallTime: {
    title: 'Tiempo de bajada',
    text: 'Tiempo que tarda la señal en bajar del 90 % al 10 % de la amplitud. Sirve para evaluar la velocidad de una salida digital.',
  },
  overshoot: {
    title: 'Overshoot',
    text: 'Cuánto supera la señal a Vtop tras un flanco, en porcentaje de la amplitud. Un valor alto indica resonancias o una carga no adaptada.',
  },
  period: {
    title: 'Periodo',
    text: 'Duración de un ciclo completo de la señal, es decir, la inversa de la frecuencia. Muestra «—» si la ventana no contiene un periodo entero.',
  },
  pulseWidthMeasure: {
    title: 'Anchura de impulso',
    text: 'Duración de la parte alta (+) y de la parte baja (−) de un ciclo, medida a mitad de la amplitud.',
  },
  edges: {
    title: 'Flancos',
    text: 'Número de flancos de subida y de bajada en la ventana visible.',
  },
  delay: {
    title: 'Retardo CH1–CH2',
    text: 'Tiempo entre un flanco de CH1 y el flanco correspondiente de CH2. Sirve para medir la latencia de un circuito.',
  },
  decoderProtocol: {
    title: 'Protocolo',
    text: 'Elige el protocolo a decodificar, por ejemplo UART, I2C o CAN. Los parámetros de abajo cambian según el protocolo.',
  },
  decoderChannel: {
    title: 'Canal del decodificador',
    text: 'Elige el canal que transporta la señal a decodificar.',
  },
  decoderThreshold: {
    title: 'Umbral del decodificador',
    text: 'Tensión que separa el 0 del 1 lógico. Con el umbral automático, el decodificador usa el punto medio entre los niveles alto y bajo de la señal.',
  },
  engine: {
    title: 'Motor',
    text: 'Fuente de datos y número de frames por segundo mostrados. La frecuencia de muestreo medida indica las muestras que llegan realmente desde el dispositivo.',
  },
}
