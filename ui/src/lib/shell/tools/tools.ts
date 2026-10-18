// Strumenti: note della matematica, passi della frequenza di taglio e chip della classificazione.
import type { MathOp, ViewMode } from '../../types'
import { step } from '../format'

/** Frequenze di taglio del filtro digitale, in Hz, in sequenza. */
export const CUTOFF_STEPS: readonly number[] = [
  100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000,
]

/** Chip della classificazione, nell'ordine del design. */
export const CHIPS = ['Clock', 'PWM', 'Seriale', 'Burst', 'Sinusoide', 'Rumore'] as const
export type ChipKind = (typeof CHIPS)[number]

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

const OP_NOTES: Record<Exclude<MathOp, 'off'>, string> = {
  add: 'M = CH1 + CH2 in V, sulla scala di CH1',
  sub: 'M = CH1 − CH2 in V, sulla scala di CH1',
  mul: 'M = CH1 × CH2 in V², sulla scala di CH1',
  derivative: 'M = d/dt CH1 in V/s, sulla scala di CH1',
  integral: 'M = ∫ CH1 dt in V·s, sulla scala di CH1',
}

/** Nota sotto i segmenti della matematica. La traccia M esiste solo nella vista Tempo. */
export function mathNote(op: MathOp, mode: ViewMode): string {
  if (op === 'off') return 'Scegli un’operazione tra i canali: la traccia M compare nella vista Tempo'
  if (mode !== 'time') return 'La traccia M si vede nella vista Tempo'
  return OP_NOTES[op]
}
