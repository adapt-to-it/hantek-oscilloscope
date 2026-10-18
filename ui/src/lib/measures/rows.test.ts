import { describe, expect, it } from 'vitest'
import type { ChannelMeasures, MeasureSnapshot } from '../types'
import { measureGroups } from './rows'

function channel(overrides: Partial<ChannelMeasures> = {}): ChannelMeasures {
  return {
    vmax: 3,
    vmin: -1,
    vpp: 4,
    vmean: 1,
    vrms: 2,
    vacRms: 1.5,
    vtop: 3,
    vbase: -1,
    vamp: 4,
    frequency: 11000,
    period: 1e-3,
    dutyPositive: 0.5,
    pulseWidthPositive: 4.5e-5,
    pulseWidthNegative: 4.5e-5,
    riseTime: 2e-6,
    fallTime: 3e-6,
    overshootPct: 12.345,
    preshootPct: null,
    risingEdges: 3,
    fallingEdges: 2,
    ...overrides,
  }
}

function snapshot(ch1: ChannelMeasures | null, ch2: ChannelMeasures | null): MeasureSnapshot {
  return { sequence: 1, channels: [ch1, ch2], phaseDeg: null, delayS: null }
}

function find(groups: ReturnType<typeof measureGroups>, title: string, label: string) {
  const row = groups.find((g) => g.title === title)?.rows.find((r) => r.label === label)
  if (!row) throw new Error(`riga mancante: ${title} / ${label}`)
  return row
}

describe('measureGroups', () => {
  it('elenca gruppi e righe nell\'ordine previsto', () => {
    const groups = measureGroups(null, [true, true])
    expect(groups.map((g) => g.title)).toEqual(['Tensione', 'Livelli e fronti', 'Tempo'])
    expect(groups[0].rows.map((r) => r.label)).toEqual(['Vpp', 'Vmax', 'Vmin', 'Vmedia', 'Vrms', 'Vac rms'])
    expect(groups[1].rows.map((r) => r.label)).toEqual(['Vtop', 'Vbase', 'Ampiezza', 'Salita', 'Discesa', 'Overshoot'])
    expect(groups[2].rows.map((r) => r.label)).toEqual([
      'Frequenza',
      'Periodo',
      'Duty',
      'Larghezza +',
      'Larghezza −',
      'Fronti',
    ])
  })

  it('formatta tensioni, tempi e frequenze', () => {
    const groups = measureGroups(snapshot(channel(), channel({ vpp: 0.5 })), [true, true])
    expect(find(groups, 'Tensione', 'Vpp')).toEqual({ label: 'Vpp', field: 'vpp', ch1: '4 V', ch2: '500 mV' })
    expect(find(groups, 'Livelli e fronti', 'Salita').ch1).toBe('2 µs')
    expect(find(groups, 'Tempo', 'Periodo').ch1).toBe('1 ms')
    expect(find(groups, 'Tempo', 'Frequenza').ch1).toBe('11 kHz')
  })

  it('mostra duty e overshoot in percento con una cifra decimale', () => {
    const groups = measureGroups(snapshot(channel(), channel({ overshootPct: 0 })), [true, true])
    expect(find(groups, 'Tempo', 'Duty').ch1).toBe('50.0 %')
    expect(find(groups, 'Livelli e fronti', 'Overshoot').ch1).toBe('12.3 %')
    expect(find(groups, 'Livelli e fronti', 'Overshoot').ch2).toBe('0.0 %')
  })

  it('scrive i fronti come salita / discesa', () => {
    const groups = measureGroups(snapshot(channel(), null), [true, true])
    expect(find(groups, 'Tempo', 'Fronti').ch1).toBe('3 / 2')
  })

  it('usa "—" per valori mancanti e per canali spenti', () => {
    const groups = measureGroups(snapshot(channel({ frequency: null, riseTime: null }), channel()), [true, false])
    expect(find(groups, 'Tempo', 'Frequenza').ch1).toBe('—')
    expect(find(groups, 'Livelli e fronti', 'Salita').ch1).toBe('—')
    expect(find(groups, 'Tensione', 'Vpp').ch2).toBe('—')
  })

  it('usa "—" ovunque se lo snapshot manca o il canale è assente', () => {
    const none = measureGroups(null, [true, true])
    expect(none.flatMap((g) => g.rows).every((r) => r.ch1 === '—' && r.ch2 === '—')).toBe(true)

    const partial = measureGroups(snapshot(null, channel()), [true, true])
    expect(find(partial, 'Tensione', 'Vmax').ch1).toBe('—')
    expect(find(partial, 'Tensione', 'Vmax').ch2).toBe('3 V')
  })
})
