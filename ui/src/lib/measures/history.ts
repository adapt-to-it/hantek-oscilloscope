// Storico delle misure per l'andamento nel tempo: buffer circolare degli ultimi snapshot.
import type { ChannelMeasures, MeasureSnapshot } from '../types'

/** Numero di letture conservate: 120 frame, circa gli ultimi minuti di acquisizione. */
export const HISTORY_SIZE = 120

interface Entry {
  /** Istante della lettura, in millisecondi. */
  t: number
  snapshot: MeasureSnapshot
}

export interface MeasureStats {
  min: number
  max: number
  avg: number
}

export class MeasureHistory {
  private readonly buffer: (Entry | undefined)[] = new Array(HISTORY_SIZE)
  private next = 0
  private count = 0

  /** Aggiunge uno snapshot; oltre la capacità sovrascrive il più vecchio. `now` in millisecondi. */
  push(snapshot: MeasureSnapshot, now: number): void {
    this.buffer[this.next] = { t: now, snapshot }
    this.next = (this.next + 1) % HISTORY_SIZE
    this.count = Math.min(this.count + 1, HISTORY_SIZE)
  }

  /** Istanti delle letture in millisecondi, dal più vecchio al più recente. */
  times(): number[] {
    return this.entries().map((entry) => entry.t)
  }

  /** Valori del campo per il canale, dal più vecchio al più recente; null dove manca o non è finito. */
  values(field: keyof ChannelMeasures, ch: 0 | 1): (number | null)[] {
    return this.entries().map(({ snapshot }) => {
      const value = snapshot.channels[ch]?.[field]
      return typeof value === 'number' && Number.isFinite(value) ? value : null
    })
  }

  /** Minimo, massimo e media dei valori validi; null se non ce ne sono. */
  stats(values: readonly (number | null)[]): MeasureStats | null {
    let min = Infinity
    let max = -Infinity
    let sum = 0
    let n = 0
    for (const value of values) {
      if (value === null) continue
      min = Math.min(min, value)
      max = Math.max(max, value)
      sum += value
      n++
    }
    return n === 0 ? null : { min, max, avg: sum / n }
  }

  clear(): void {
    this.buffer.fill(undefined)
    this.next = 0
    this.count = 0
  }

  private entries(): Entry[] {
    const start = (this.next - this.count + HISTORY_SIZE) % HISTORY_SIZE
    const out: Entry[] = []
    for (let i = 0; i < this.count; i++) {
      out.push(this.buffer[(start + i) % HISTORY_SIZE] as Entry)
    }
    return out
  }
}
