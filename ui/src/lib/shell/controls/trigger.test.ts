import { describe, expect, it } from 'vitest'
import type { AcquisitionSettings } from '../../types'
import { defaultSpec, stepDelay, stepValue, triggerSummary } from './trigger'
import { TIME_STEPS } from '../format'

function acq(patch: Partial<AcquisitionSettings>): AcquisitionSettings {
  return {
    mode: 'auto',
    probe: [1, 1],
    ac: [false, false],
    pretrigger: 0.5,
    triggerSource: 1,
    trigger: { kind: 'edge', slope: 'rising' },
    triggerLevel: 1.5,
    hysteresis: 0,
    holdoff: 0,
    ...patch,
  }
}

describe('defaultSpec', () => {
  it('dà un fronte di salita', () => {
    expect(defaultSpec('edge', 1)).toEqual({ kind: 'edge', slope: 'rising' })
  })

  it('costruisce fasce di ±0,5 V attorno al livello', () => {
    expect(defaultSpec('runt', 1.5)).toEqual({ kind: 'runt', polarity: 'positive', low: 1, high: 2 })
    expect(defaultSpec('window', -1)).toEqual({ kind: 'window', low: -1.5, high: -0.5, enter: true })
  })

  it('usa durate di 1 ms e larghezza maggiore', () => {
    expect(defaultSpec('pulseWidth', 0)).toEqual({
      kind: 'pulseWidth',
      polarity: 'positive',
      width: { op: 'greater', a: 1e-3, b: 2e-3 },
    })
    expect(defaultSpec('timeout', 0)).toEqual({ kind: 'timeout', slope: 'rising', seconds: 1e-3 })
    expect(defaultSpec('logic', 2)).toEqual({ kind: 'logic', op: 'or', ch1: 'rising', ch2: 'rising', otherLevel: 2 })
    expect(defaultSpec('sequence', -1)).toEqual({ kind: 'sequence', slopeA: 'rising', slopeB: 'rising', otherLevel: -1, delay: 0 })
  })
})

describe('triggerSummary', () => {
  it('mostra modo, sorgente, freccia del fronte e livello', () => {
    expect(triggerSummary(acq({}))).toBe('Auto · CH1 ↑ 1.5 V')
    expect(
      triggerSummary(acq({ mode: 'normal', triggerSource: 2, trigger: { kind: 'edge', slope: 'falling' }, triggerLevel: -0.5 })),
    ).toBe('Normal · CH2 ↓ -500 mV')
    expect(triggerSummary(acq({ trigger: { kind: 'edge', slope: 'either' }, triggerLevel: 0 }))).toBe('Auto · CH1 ↕ 0 V')
  })

  it('usa il nome del tipo al posto della freccia per gli altri trigger', () => {
    expect(
      triggerSummary(acq({ mode: 'roll', trigger: { kind: 'runt', polarity: 'positive', low: 1, high: 2 } })),
    ).toBe('Roll · CH1 Runt 1.5 V')
  })
})

describe('stepValue', () => {
  it('scorre le durate nella sequenza 1-2-5', () => {
    expect(stepValue('time', 1e-4, 1)).toBe(2e-4)
    expect(stepValue('time', 1e-4, -1)).toBe(5e-5)
    expect(stepValue('time', 10, 1)).toBe(10)
  })

  it('scorre le tensioni positive nella sequenza 1-2-5', () => {
    expect(stepValue('volt', 0.5, 1)).toBe(1)
    expect(stepValue('volt', 0.5, -1)).toBe(0.2)
    expect(stepValue('volt', 0.01, -1)).toBe(0.01)
  })

  it('attraversa lo zero con le tensioni negative', () => {
    expect(stepValue('volt', 0, 1)).toBe(0.01)
    expect(stepValue('volt', 0, -1)).toBe(-0.01)
    expect(stepValue('volt', -0.01, 1)).toBe(0)
    expect(stepValue('volt', -0.5, 1)).toBe(-0.2)
    expect(stepValue('volt', -0.5, -1)).toBe(-1)
  })
})

describe('stepDelay', () => {
  it('passa da zero al primo gradino e torna a zero', () => {
    expect(stepDelay(0, 1)).toBe(TIME_STEPS[0])
    expect(stepDelay(TIME_STEPS[0], -1)).toBe(0)
    expect(stepDelay(0, -1)).toBe(0)
    expect(stepDelay(TIME_STEPS[0], 1)).toBe(TIME_STEPS[1])
  })
})
