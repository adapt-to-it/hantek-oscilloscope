import { describe, expect, it } from 'vitest'
import { cursorReadout } from './cursors'
import { formatHz, formatTime, formatVolts } from '../shell/format'
import type { CursorPositions } from '../types'

const pos: CursorPositions = { t1: -0.001, t2: 0.001, v1: 0.5, v2: -1.5 }

describe('cursorReadout', () => {
  it('modo off non produce righe', () => {
    expect(cursorReadout('off', pos)).toEqual([])
  })

  it('modo time mostra t1, t2, Δt e 1/Δt', () => {
    const rows = cursorReadout('time', pos)
    expect(rows.map((r) => r.k)).toEqual(['t1', 't2', 'Δt', '1/Δt'])
    expect(rows[0].v).toBe(formatTime(-0.001))
    expect(rows[1].v).toBe(formatTime(0.001))
    expect(rows[2].v).toBe(formatTime(0.002))
    expect(rows[3].v).toBe(formatHz(1 / 0.002))
  })

  it('modo voltage mostra V1, V2 e ΔV con ΔV = V2 − V1', () => {
    const rows = cursorReadout('voltage', pos)
    expect(rows.map((r) => r.k)).toEqual(['V1', 'V2', 'ΔV'])
    expect(rows[0].v).toBe(formatVolts(0.5))
    expect(rows[1].v).toBe(formatVolts(-1.5))
    expect(rows[2].v).toBe(formatVolts(-2))
  })

  it('modo both unisce le righe di tempo e tensione nell\'ordine', () => {
    const rows = cursorReadout('both', pos)
    expect(rows.map((r) => r.k)).toEqual(['t1', 't2', 'Δt', '1/Δt', 'V1', 'V2', 'ΔV'])
  })

  it('con Δt = 0 la riga 1/Δt vale il trattino', () => {
    const rows = cursorReadout('time', { ...pos, t1: 0.002, t2: 0.002 })
    const inv = rows.find((r) => r.k === '1/Δt')
    expect(inv?.v).toBe('—')
    expect(rows.find((r) => r.k === 'Δt')?.v).toBe(formatTime(0))
  })

  it('1/Δt usa il valore assoluto di Δt, quindi non è negativo', () => {
    const rows = cursorReadout('time', { ...pos, t1: 0.002, t2: 0 })
    expect(rows.find((r) => r.k === '1/Δt')?.v).toBe(formatHz(1 / 0.002))
  })
})
