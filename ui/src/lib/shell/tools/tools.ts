// Strumenti: note della matematica, passi della frequenza di taglio e chip della classificazione.
import type { MathOp, ViewMode } from '../../types'
import { step } from '../format'
import { tr } from '../../i18n/msg/measures'

/** Frequenze di taglio del filtro digitale, in Hz, in sequenza. */
export const CUTOFF_STEPS: readonly number[] = [
  100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000,
]

/** Chip della classificazione, nell'ordine del design. */
export const CHIPS = ['Clock', 'PWM', 'Seriale', 'Burst', 'Sinusoide', 'Rumore'] as const
export type ChipKind = (typeof CHIPS)[number]

/** Chiave di traduzione di ogni chip: l'identificativo resta quello di sopra. */
const CHIP_KEY = {
  Clock: 'chipClock',
  PWM: 'chipPwm',
  Seriale: 'chipSerial',
  Burst: 'chipBurst',
  Sinusoide: 'chipSine',
  Rumore: 'chipNoise',
} as const satisfies Record<ChipKind, string>

/** Etichetta visibile del chip, nella lingua corrente. */
export function chipLabel(chip: ChipKind): string {
  return tr(CHIP_KEY[chip])
}

/** Classe ricevuta dal backend, da evidenziare tra i chip. `dc` e `other` non ne hanno. */
const CLASS_CHIP = new Map<string, ChipKind>([
  ['clock', 'Clock'],
  ['pwm', 'PWM'],
  ['serial', 'Seriale'],
  ['burst', 'Burst'],
  ['analog', 'Sinusoide'],
  ['noise', 'Rumore'],
])

/** Chip da evidenziare per la classe ricevuta, altrimenti null. */
export function classHighlight(cls: string): ChipKind | null {
  return CLASS_CHIP.get(cls) ?? null
}

/** Frequenza di taglio successiva o precedente, partendo da quella più vicina a `hz`. */
export function stepCutoff(hz: number, dir: 1 | -1): number {
  return step(CUTOFF_STEPS, hz, dir)
}

const OP_KEY = {
  add: 'opAdd',
  sub: 'opSub',
  mul: 'opMul',
  derivative: 'opDerivative',
  integral: 'opIntegral',
} as const satisfies Record<Exclude<MathOp, 'off'>, string>

/** Nota sotto i segmenti della matematica. La traccia M esiste solo nella vista Tempo. */
export function mathNote(op: MathOp, mode: ViewMode): string {
  if (op === 'off') return tr('mathOffNote')
  if (mode !== 'time') return tr('mathOnlyTime')
  return tr(OP_KEY[op])
}
