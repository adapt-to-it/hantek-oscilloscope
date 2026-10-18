import { describe, expect, it } from 'vitest'
import { DB_MAX, DB_MIN, decadeTicks, dbToY, formatDb, freqToX, xToFreq } from './scale'

describe('freqToX', () => {
  it('in lineare va da 0 a nyquist sull\'intera larghezza', () => {
    expect(freqToX(0, 800, 1000, false, 1)).toBe(0)
    expect(freqToX(500, 800, 1000, false, 1)).toBeCloseTo(400)
    expect(freqToX(1000, 800, 1000, false, 1)).toBeCloseTo(800)
  })

  it('in logaritmica segue le decadi', () => {
    // Da 10 Hz a 10 kHz: tre decadi su 400 px.
    expect(freqToX(10, 400, 10000, true, 10)).toBeCloseTo(0)
    expect(freqToX(100, 400, 10000, true, 10)).toBeCloseTo(400 / 3)
    expect(freqToX(1000, 400, 10000, true, 10)).toBeCloseTo((2 * 400) / 3)
    expect(freqToX(10000, 400, 10000, true, 10)).toBeCloseTo(400)
  })

  it('in logaritmica le frequenze sotto minHz stanno sul bordo sinistro', () => {
    expect(freqToX(1, 400, 10000, true, 10)).toBe(0)
    expect(freqToX(0, 400, 10000, true, 10)).toBe(0)
  })

  it('in logaritmica con intervallo degenere non dà NaN', () => {
    expect(freqToX(5, 400, 10, true, 10)).toBe(0)
  })
})

describe('xToFreq', () => {
  it('è l\'inversa di freqToX in lineare', () => {
    const x = freqToX(1234, 640, 5000, false, 1)
    expect(xToFreq(x, 640, 5000, false, 1)).toBeCloseTo(1234)
  })

  it('è l\'inversa di freqToX in logaritmica', () => {
    const x = freqToX(440, 640, 24000, true, 10)
    expect(xToFreq(x, 640, 24000, true, 10)).toBeCloseTo(440)
    expect(xToFreq(0, 640, 24000, true, 10)).toBeCloseTo(10)
    expect(xToFreq(640, 640, 24000, true, 10)).toBeCloseTo(24000)
  })
})

describe('dbToY', () => {
  it('mette DB_MAX in alto e DB_MIN in basso', () => {
    expect(dbToY(DB_MAX, 120)).toBe(0)
    expect(dbToY(DB_MIN, 120)).toBe(120)
    expect(dbToY(-40, 120)).toBeCloseTo(60)
  })
})

describe('decadeTicks', () => {
  it('elenca le decadi nell\'intervallo', () => {
    expect(decadeTicks(10, 20000)).toEqual([10, 100, 1000, 10000])
    expect(decadeTicks(1, 1)).toEqual([1])
  })

  it('salta le decadi fuori dall\'intervallo', () => {
    expect(decadeTicks(11, 99)).toEqual([])
    expect(decadeTicks(20, 10)).toEqual([])
    expect(decadeTicks(0, 100)).toEqual([])
    expect(decadeTicks(1, Infinity)).toEqual([])
  })
})

describe('formatDb', () => {
  it('mostra il segno e una cifra decimale con l\'unità', () => {
    expect(formatDb(-12.34)).toBe('-12.3 dBV')
    expect(formatDb(3)).toBe('+3.0 dBV')
  })

  it('senza unità e con zero decimali per gli assi', () => {
    expect(formatDb(20, 0, false)).toBe('+20')
    expect(formatDb(-100, 0, false)).toBe('-100')
  })

  it('non mostra -0', () => {
    expect(formatDb(-0.04)).toBe('0.0 dBV')
    expect(formatDb(0, 0, false)).toBe('0')
  })
})
