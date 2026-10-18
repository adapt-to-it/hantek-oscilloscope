// Tipi, opzioni e formati del trigger per il pannello Controlli. Specchio di crates/h6022-view/src/acquisition.rs.
import type {
  AcquisitionSettings,
  CondName,
  LevelName,
  LogicOpName,
  ModeName,
  PolarityName,
  SlopeName,
  TriggerSpec,
  WidthSpec,
} from '../../types'
import { TIME_STEPS, VOLT_STEPS, formatVolts, step } from '../format'
import { tr } from '../../i18n/msg/trigger'
import type { TriggerKey } from '../../i18n/msg/trigger'

export type TriggerKind = TriggerSpec['kind']

/** Semiampiezza della fascia attorno al livello, in volt, per i trigger a due soglie. */
const BAND_HALF_V = 0.5
/** Durata iniziale dei trigger temporali, in secondi. */
const DEFAULT_DURATION_S = 1e-3

const MODE_KEY: Record<ModeName, TriggerKey> = {
  auto: 'modeAuto',
  normal: 'modeNormal',
  single: 'modeSingle',
  roll: 'modeRoll',
}
const SLOPE_ARROW: Record<SlopeName, string> = { rising: '↑', falling: '↓', either: '↕' }
const KIND_KEY: Record<TriggerKind, TriggerKey> = {
  edge: 'kindEdge',
  pulseWidth: 'kindPulseWidth',
  runt: 'kindRunt',
  window: 'kindWindow',
  timeout: 'kindTimeout',
  pattern: 'kindPattern',
  logic: 'kindLogic',
  sequence: 'kindSequence',
}

/** Opzioni dei selettori: le etichette sono testi della lingua corrente, quindi si leggono al momento della chiamata. */
export function modeOptions(): readonly { value: ModeName; label: string }[] {
  return (['auto', 'normal', 'single', 'roll'] as const).map((value) => ({ value, label: tr(MODE_KEY[value]) }))
}

export function triggerKinds(): readonly { kind: TriggerKind; label: string }[] {
  return (['edge', 'pulseWidth', 'runt', 'window', 'timeout', 'pattern', 'logic', 'sequence'] as const).map((kind) => ({
    kind,
    label: tr(KIND_KEY[kind]),
  }))
}

export function slopeOptions(): readonly { value: SlopeName; label: string }[] {
  return [
    { value: 'rising', label: tr('slopeRising') },
    { value: 'falling', label: tr('slopeFalling') },
    { value: 'either', label: tr('slopeEither') },
  ]
}

export function polarityOptions(): readonly { value: PolarityName; label: string }[] {
  return [
    { value: 'positive', label: tr('polarityPositive') },
    { value: 'negative', label: tr('polarityNegative') },
  ]
}

export function levelOptions(): readonly { value: LevelName; label: string }[] {
  return [
    { value: 'high', label: tr('levelHigh') },
    { value: 'low', label: tr('levelLow') },
    { value: 'any', label: tr('levelAny') },
  ]
}

export function widthOptions(): readonly { value: WidthSpec['op']; label: string }[] {
  return [
    { value: 'less', label: tr('widthLess') },
    { value: 'greater', label: tr('widthGreater') },
    { value: 'within', label: tr('widthWithin') },
    { value: 'outside', label: tr('widthOutside') },
  ]
}

export const LOGIC_OP_OPTIONS: readonly { value: LogicOpName; label: string }[] = [
  { value: 'and', label: 'AND' },
  { value: 'or', label: 'OR' },
]

/** Condizione di un canale nel trigger logico: fronte, livello rispetto alla soglia o nessuna. */
export function condOptions(): readonly { value: CondName; label: string }[] {
  return [
    { value: 'rising', label: '↑' },
    { value: 'falling', label: '↓' },
    { value: 'either', label: '↕' },
    { value: 'high', label: tr('levelHigh') },
    { value: 'low', label: tr('levelLow') },
    { value: 'ignore', label: '—' },
  ]
}

/** Sorgente del trigger: il valore è una stringa per i gruppi di pulsanti, 1 o 2 nelle impostazioni. */
export const SOURCE_OPTIONS: readonly { value: '1' | '2'; label: string }[] = [
  { value: '1', label: 'CH1' },
  { value: '2', label: 'CH2' },
]

/** Entra o esce dalla finestra: `enter` nelle impostazioni. */
export function enterOptions(): readonly { value: 'enter' | 'exit'; label: string }[] {
  return [
    { value: 'enter', label: tr('enterIn') },
    { value: 'exit', label: tr('enterOut') },
  ]
}

/**
 * Trigger valido del tipo indicato, con la fascia attorno a `level` (±0,5 V) e durate da 1 ms.
 * Per l'impulso l'intervallo "entro" o "fuori" va da 1 ms a 2 ms. Pattern parte da CH1 alto e CH2 basso.
 * Logico e A → B partono con fronti di salita e il livello dell'altro canale uguale a `level`.
 */
export function defaultSpec(kind: TriggerKind, level: number): TriggerSpec {
  switch (kind) {
    case 'edge':
      return { kind, slope: 'rising' }
    case 'pulseWidth':
      return {
        kind,
        polarity: 'positive',
        width: { op: 'greater', a: DEFAULT_DURATION_S, b: 2 * DEFAULT_DURATION_S },
      }
    case 'runt':
      return { kind, polarity: 'positive', low: level - BAND_HALF_V, high: level + BAND_HALF_V }
    case 'window':
      return { kind, low: level - BAND_HALF_V, high: level + BAND_HALF_V, enter: true }
    case 'timeout':
      return { kind, slope: 'rising', seconds: DEFAULT_DURATION_S }
    case 'pattern':
      return { kind, ch1: 'high', ch2: 'low' }
    case 'logic':
      return { kind, op: 'or', ch1: 'rising', ch2: 'rising', otherLevel: level }
    case 'sequence':
      return { kind, slopeA: 'rising', slopeB: 'rising', otherLevel: level, delay: 0 }
  }
}

/** Testo dell'intestazione: "Auto · CH1 ↑ 1.5 V". Per i trigger non a fronte al posto della freccia c'è il nome del tipo. */
export function triggerSummary(acq: AcquisitionSettings): string {
  const { trigger } = acq
  const detail = trigger.kind === 'edge' ? SLOPE_ARROW[trigger.slope] : tr(KIND_KEY[trigger.kind])
  return `${tr(MODE_KEY[acq.mode])} · CH${acq.triggerSource} ${detail} ${formatVolts(acq.triggerLevel)}`
}

export type StepKind = 'time' | 'volt'

/**
 * Valore adiacente a `current` nella sequenza 1-2-5 del tipo indicato: `dir` > 0 il successivo, < 0 il precedente.
 * Le tensioni possono essere negative. Da zero si passa a ±10 mV, e da −10 mV si passa a zero salendo.
 */
export function stepValue(kind: StepKind, current: number, dir: number): number {
  if (kind === 'time') return step(TIME_STEPS, current, dir)
  const magnitude = Math.abs(current)
  if (magnitude === 0) return dir > 0 ? VOLT_STEPS[0] : dir < 0 ? -VOLT_STEPS[0] : 0
  if (current > 0) return step(VOLT_STEPS, magnitude, dir)
  if (dir > 0 && magnitude <= VOLT_STEPS[0]) return 0
  return -step(VOLT_STEPS, magnitude, -dir)
}

/** Passo del ritardo A → B: come `stepValue('time')`, ma con lo zero sotto il primo gradino. */
export function stepDelay(current: number, dir: number): number {
  if (current <= 0) return dir > 0 ? TIME_STEPS[0] : 0
  if (dir < 0 && current <= TIME_STEPS[0]) return 0
  return step(TIME_STEPS, current, dir)
}
