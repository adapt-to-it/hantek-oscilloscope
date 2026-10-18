// Tipi, opzioni e formati del trigger per il pannello Controlli. Specchio di crates/h6022-view/src/acquisition.rs.
import type {
  AcquisitionSettings,
  LevelName,
  ModeName,
  PolarityName,
  SlopeName,
  TriggerSpec,
  WidthSpec,
} from '../../types'
import { TIME_STEPS, VOLT_STEPS, formatVolts, step } from '../format'

export type TriggerKind = TriggerSpec['kind']

/** Semiampiezza della fascia attorno al livello, in volt, per i trigger a due soglie. */
const BAND_HALF_V = 0.5
/** Durata iniziale dei trigger temporali, in secondi. */
const DEFAULT_DURATION_S = 1e-3

const MODE_LABEL: Record<ModeName, string> = { auto: 'Auto', normal: 'Normal', single: 'Single', roll: 'Roll' }
const SLOPE_LABEL: Record<SlopeName, string> = { rising: 'Salita', falling: 'Discesa', either: 'Entrambi' }
const SLOPE_ARROW: Record<SlopeName, string> = { rising: '↑', falling: '↓', either: '↕' }
const KIND_LABEL: Record<TriggerKind, string> = {
  edge: 'Fronte',
  pulseWidth: 'Impulso',
  runt: 'Runt',
  window: 'Finestra',
  timeout: 'Timeout',
  pattern: 'Pattern',
}

export const MODE_OPTIONS: readonly { value: ModeName; label: string }[] = (
  ['auto', 'normal', 'single', 'roll'] as const
).map((value) => ({ value, label: MODE_LABEL[value] }))

export const TRIGGER_KINDS: readonly { kind: TriggerKind; label: string }[] = (
  ['edge', 'pulseWidth', 'runt', 'window', 'timeout', 'pattern'] as const
).map((kind) => ({ kind, label: KIND_LABEL[kind] }))

export const SLOPE_OPTIONS: readonly { value: SlopeName; label: string }[] = (
  ['rising', 'falling', 'either'] as const
).map((value) => ({ value, label: SLOPE_LABEL[value] }))

export const POLARITY_OPTIONS: readonly { value: PolarityName; label: string }[] = [
  { value: 'positive', label: 'Positivo' },
  { value: 'negative', label: 'Negativo' },
]

export const LEVEL_OPTIONS: readonly { value: LevelName; label: string }[] = [
  { value: 'high', label: 'Alto' },
  { value: 'low', label: 'Basso' },
  { value: 'any', label: 'Qualsiasi' },
]

export const WIDTH_OPTIONS: readonly { value: WidthSpec['op']; label: string }[] = [
  { value: 'less', label: 'Minore' },
  { value: 'greater', label: 'Maggiore' },
  { value: 'within', label: 'Entro' },
  { value: 'outside', label: 'Fuori' },
]

/** Sorgente del trigger: il valore è una stringa per i gruppi di pulsanti, 1 o 2 nelle impostazioni. */
export const SOURCE_OPTIONS: readonly { value: '1' | '2'; label: string }[] = [
  { value: '1', label: 'CH1' },
  { value: '2', label: 'CH2' },
]

/** Entra o esce dalla finestra: `enter` nelle impostazioni. */
export const ENTER_OPTIONS: readonly { value: 'enter' | 'exit'; label: string }[] = [
  { value: 'enter', label: 'Entra' },
  { value: 'exit', label: 'Esce' },
]

/**
 * Trigger valido del tipo indicato, con la fascia attorno a `level` (±0,5 V) e durate da 1 ms.
 * Per l'impulso l'intervallo "entro" o "fuori" va da 1 ms a 2 ms. Pattern parte da CH1 alto e CH2 basso.
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
  }
}

/** Testo dell'intestazione: "Auto · CH1 ↑ 1.5 V". Per i trigger non a fronte al posto della freccia c'è il nome del tipo. */
export function triggerSummary(acq: AcquisitionSettings): string {
  const { trigger } = acq
  const detail = trigger.kind === 'edge' ? SLOPE_ARROW[trigger.slope] : KIND_LABEL[trigger.kind]
  return `${MODE_LABEL[acq.mode]} · CH${acq.triggerSource} ${detail} ${formatVolts(acq.triggerLevel)}`
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
