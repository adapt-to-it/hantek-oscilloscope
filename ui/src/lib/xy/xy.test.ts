import { describe, expect, it } from 'vitest'
import type { ChannelSettings } from '../types'
import { XY_DIVISIONS, pxToVolts, voltsToPx } from './xy'

const ch = (voltsPerDiv: number, offsetDiv = 0): ChannelSettings => ({
  enabled: true,
  voltsPerDiv,
  offsetDiv,
})

describe('voltsToPx', () => {
  it('mette 0 V al centro dell asse senza offset', () => {
    expect(voltsToPx(0, ch(1), 400)).toBe(200)
  })

  it('una divisione corrisponde a size / 8 pixel', () => {
    expect(voltsToPx(0.5, ch(0.5), 400)).toBeCloseTo(200 + 400 / XY_DIVISIONS)
    expect(voltsToPx(-1, ch(0.5), 400)).toBeCloseTo(200 - 2 * (400 / XY_DIVISIONS))
  })

  it('applica l offset in divisioni', () => {
    // offsetDiv = 1 sposta lo zero di una divisione verso destra.
    expect(voltsToPx(0, ch(1, 1), 400)).toBeCloseTo(200 + 400 / XY_DIVISIONS)
  })

  it('rifiuta voltsPerDiv non positivi o non finiti', () => {
    expect(() => voltsToPx(0, ch(0), 400)).toThrow(RangeError)
    expect(() => voltsToPx(0, ch(-1), 400)).toThrow(RangeError)
    expect(() => voltsToPx(0, ch(Number.NaN), 400)).toThrow(RangeError)
  })

  it('rifiuta una dimensione non positiva', () => {
    expect(() => voltsToPx(0, ch(1), 0)).toThrow(RangeError)
  })
})

describe('pxToVolts', () => {
  it('e l inversa di voltsToPx', () => {
    const settings = [ch(0.2), ch(2, -1.5), ch(0.05, 3)]
    for (const c of settings) {
      for (const v of [-3.2, -0.4, 0, 0.7, 4.1]) {
        const px = voltsToPx(v, c, 320)
        expect(pxToVolts(px, c, 320)).toBeCloseTo(v, 9)
      }
    }
  })

  it('riporta 0 V al centro dell asse', () => {
    expect(pxToVolts(160, ch(1), 320)).toBeCloseTo(0, 12)
  })

  it('rifiuta voltsPerDiv non positivi', () => {
    expect(() => pxToVolts(10, ch(0), 320)).toThrow(RangeError)
  })
})
