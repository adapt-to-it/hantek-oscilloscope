import { describe, expect, it } from 'vitest'
import { TIME_STEPS, VOLT_STEPS, formatDiv, formatHz, formatTime, formatVolts, step } from './format'

describe('step', () => {
  it('si ferma agli estremi della sequenza', () => {
    expect(step(TIME_STEPS, 10, 1)).toBe(10)
    expect(step(TIME_STEPS, 1e-7, -1)).toBe(1e-7)
  })

  it('va ai vicini e parte dal valore più vicino se fuori lista', () => {
    expect(step(TIME_STEPS, 1e-4, 1)).toBe(2e-4)
    expect(step(TIME_STEPS, 1e-4, -1)).toBe(5e-5)
    expect(step(VOLT_STEPS, 0.3, 1)).toBe(0.5)
    expect(step(VOLT_STEPS, 0.3, -1)).toBe(0.1)
  })
})

describe('formatTime', () => {
  it('usa l\'unità e tre cifre significative', () => {
    expect(formatTime(1e-4)).toBe('100 µs')
    expect(formatTime(2e-7)).toBe('200 ns')
    expect(formatTime(0.05)).toBe('50 ms')
    expect(formatTime(10)).toBe('10 s')
    expect(formatTime(0)).toBe('0 s')
    expect(formatTime(-2e-6)).toBe('-2 µs')
  })

  it('passa all\'unità successiva dopo l\'arrotondamento', () => {
    expect(formatTime(0.9996e-3)).toBe('1 ms')
  })
})

describe('formatVolts', () => {
  it('sceglie mV sotto un volt', () => {
    expect(formatVolts(1)).toBe('1 V')
    expect(formatVolts(0.5)).toBe('500 mV')
    expect(formatVolts(0.01)).toBe('10 mV')
    expect(formatVolts(0.9996)).toBe('1 V')
    expect(formatVolts(5)).toBe('5 V')
  })
})

describe('formatHz', () => {
  it('scala le frequenze di campionamento', () => {
    expect(formatHz(48e6)).toBe('48 MHz')
    expect(formatHz(11000)).toBe('11 kHz')
    expect(formatHz(50)).toBe('50 Hz')
  })
})

describe('formatDiv', () => {
  it('mostra gli offset in divisioni', () => {
    expect(formatDiv(0)).toBe('0 div')
    expect(formatDiv(1.5)).toBe('1.5 div')
    expect(formatDiv(-0.5)).toBe('-0.5 div')
  })
})
