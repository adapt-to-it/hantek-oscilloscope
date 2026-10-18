// Scale e formattazione dei valori. Specchio di crates/h6022-view/src/timebase.rs.

/** Secondi per divisione in sequenza 1-2-5, da 100 ns a 10 s. */
export const TIME_STEPS: readonly number[] = [
  1e-7, 2e-7, 5e-7, 1e-6, 2e-6, 5e-6, 1e-5, 2e-5, 5e-5, 1e-4, 2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2, 5e-2,
  1e-1, 2e-1, 5e-1, 1, 2, 5, 10,
]

/** Volt per divisione in sequenza 1-2-5, da 10 mV a 5 V (all'ingresso dell'oscilloscopio). */
export const VOLT_STEPS: readonly number[] = [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5]

/**
 * Valore adiacente a `current` in `steps`: `dir` > 0 il successivo, < 0 il precedente.
 * Se `current` non è nella lista, parte dal valore più vicino. Si ferma agli estremi.
 */
export function step(steps: readonly number[], current: number, dir: number): number {
  if (steps.length === 0) return current
  let index = steps.indexOf(current)
  if (index < 0) {
    index = 0
    for (let i = 1; i < steps.length; i++) {
      if (Math.abs(steps[i] - current) < Math.abs(steps[index] - current)) index = i
    }
  }
  const last = steps.length - 1
  if (dir > 0) index = Math.min(index + 1, last)
  else if (dir < 0) index = Math.max(index - 1, 0)
  return steps[index]
}

type Tier = readonly [threshold: number, factor: number, unit: string]

/** Tre cifre significative al massimo, senza zeri finali: 1.5, 100, 12.3. */
export function significant(value: number): string {
  return String(Number(value.toPrecision(3)))
}

/** Arrotonda a tre cifre significative prima di scegliere l'unità: 0,9996 V diventa 1 V, non 1000 mV. */
function roundSignificant(value: number): number {
  if (value === 0 || !Number.isFinite(value)) return value
  return Number(value.toPrecision(3))
}

/** Valore con segno nell'unità della prima soglia non superata. Ultima soglia infinita. */
function scaled(value: number, zero: string, tiers: readonly Tier[]): string {
  const magnitude = roundSignificant(Math.abs(value))
  if (magnitude === 0) return zero
  const sign = value < 0 ? '-' : ''
  const [, factor, unit] = tiers.find(([threshold]) => magnitude < threshold) ?? tiers[tiers.length - 1]
  return `${sign}${significant(magnitude * factor)} ${unit}`
}

/** "100 ns", "2 µs", "50 ms", "1 s"; negativi con il segno meno. */
export function formatTime(seconds: number): string {
  return scaled(seconds, '0 s', [
    [1e-6, 1e9, 'ns'],
    [1e-3, 1e6, 'µs'],
    [1, 1e3, 'ms'],
    [Infinity, 1, 's'],
  ])
}

/** "10 mV", "1 V", "500 mV". */
export function formatVolts(volts: number): string {
  return scaled(volts, '0 V', [
    [1, 1e3, 'mV'],
    [Infinity, 1, 'V'],
  ])
}

/** "50 Hz", "11 kHz", "48 MHz". */
export function formatHz(hz: number): string {
  return scaled(hz, '0 Hz', [
    [1e3, 1, 'Hz'],
    [1e6, 1e-3, 'kHz'],
    [Infinity, 1e-6, 'MHz'],
  ])
}

/** Offset in divisioni: "0 div", "1.5 div", "-0.5 div". */
export function formatDiv(divisions: number): string {
  const text = significant(Math.abs(divisions))
  return divisions < 0 ? `-${text} div` : `${text} div`
}
