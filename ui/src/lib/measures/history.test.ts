import { describe, expect, it } from 'vitest'
import type { ChannelMeasures, MeasureSnapshot } from '../types'
import { HISTORY_SIZE, MeasureHistory } from './history'

function channel(vpp: number): ChannelMeasures {
  return {
    vmax: 0,
    vmin: 0,
    vpp,
    vmean: 0,
    vrms: 0,
    vacRms: 0,
    vtop: 0,
    vbase: 0,
    vamp: 0,
    frequency: null,
    period: null,
    dutyPositive: null,
    pulseWidthPositive: null,
    pulseWidthNegative: null,
    riseTime: null,
    fallTime: null,
    overshootPct: null,
    preshootPct: null,
    risingEdges: 0,
    fallingEdges: 0,
  }
}

function snapshot(ch1: number | null, ch2: number | null = null): MeasureSnapshot {
  return {
    sequence: 1,
    channels: [ch1 === null ? null : channel(ch1), ch2 === null ? null : channel(ch2)],
    phaseDeg: null,
    delayS: null,
  }
}

describe('MeasureHistory', () => {
  it('restituisce i valori dal più vecchio al più recente', () => {
    const history = new MeasureHistory()
    history.push(snapshot(1), 100)
    history.push(snapshot(2), 200)
    history.push(snapshot(3), 300)
    expect(history.values('vpp', 0)).toEqual([1, 2, 3])
    expect(history.times()).toEqual([100, 200, 300])
  })

  it('scarta le letture più vecchie oltre la capacità', () => {
    const history = new MeasureHistory()
    for (let i = 0; i < HISTORY_SIZE + 5; i++) history.push(snapshot(i), i)
    const values = history.values('vpp', 0)
    expect(values).toHaveLength(HISTORY_SIZE)
    expect(values[0]).toBe(5)
    expect(values[HISTORY_SIZE - 1]).toBe(HISTORY_SIZE + 4)
    expect(history.times()[0]).toBe(5)
  })

  it('usa null per canali spenti o valori non finiti', () => {
    const history = new MeasureHistory()
    history.push(snapshot(1, null), 0)
    history.push(snapshot(Number.NaN), 1)
    expect(history.values('vpp', 0)).toEqual([1, null])
    expect(history.values('vpp', 1)).toEqual([null, null])
  })

  it('calcola min, max e media ignorando i null', () => {
    const history = new MeasureHistory()
    expect(history.stats([2, null, 4, 6])).toEqual({ min: 2, max: 6, avg: 4 })
  })

  it('restituisce null se non ci sono valori validi', () => {
    const history = new MeasureHistory()
    expect(history.stats([])).toBeNull()
    expect(history.stats([null, null])).toBeNull()
  })

  it('svuota lo storico con clear', () => {
    const history = new MeasureHistory()
    history.push(snapshot(1), 0)
    history.clear()
    expect(history.values('vpp', 0)).toEqual([])
    expect(history.times()).toEqual([])
    history.push(snapshot(7), 10)
    expect(history.values('vpp', 0)).toEqual([7])
  })
})
