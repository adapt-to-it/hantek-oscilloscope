import { describe, it, expect } from 'vitest'
import { HELP, type HelpId } from './texts'

// Record su HelpId: TypeScript segnala una chiave mancante o in più rispetto all'unione.
const EXPECTED: Record<HelpId, true> = {
  run: true, stop: true, single: true, triggerMode: true, triggerSource: true, triggerKind: true,
  triggerLevel: true, triggerSlope: true, hysteresis: true, holdoff: true, pretrigger: true,
  timePerDiv: true, horizontalOffset: true, channelEnable: true, voltsPerDiv: true,
  verticalOffset: true, probe: true, coupling: true, cursors: true, view: true, fftWindow: true,
  persistence: true, measurements: true, vpp: true, vrms: true, vacRms: true, frequency: true,
  duty: true, riseTime: true, thd: true, phase: true, sampleRate: true, recordLength: true,
  math: true, filter: true, classify: true, recording: true, decoder: true, mcp: true,
  device: true, trend: true,
  explain: true,
  status: true,
  scope: true,
  panelTabs: true,
  logFrequency: true,
  pulsePolarity: true,
  pulseWidth: true,
  runt: true,
  windowTrigger: true,
  timeoutTrigger: true,
  patternTrigger: true,
  logicTrigger: true,
  otherLevel: true,
  sequenceTrigger: true,
  sequenceDelay: true,
  vmax: true,
  vmin: true,
  vmean: true,
  vtop: true,
  vbase: true,
  vamp: true,
  fallTime: true,
  overshoot: true,
  period: true,
  pulseWidthMeasure: true,
  edges: true,
  delay: true,
  decoderProtocol: true,
  decoderChannel: true,
  decoderThreshold: true,
  engine: true,
}

const IDS = Object.keys(EXPECTED) as HelpId[]

describe('HELP', () => {
  it.each(IDS)('%s ha titolo e testo non vuoti', (id) => {
    expect(HELP[id].title.trim()).not.toBe('')
    expect(HELP[id].text.trim()).not.toBe('')
  })

  it.each(IDS)('%s ha un testo di al massimo 250 caratteri', (id) => {
    expect(HELP[id].text.length).toBeLessThanOrEqual(250)
  })

  it('non ha chiavi oltre a quelle di HelpId', () => {
    expect(Object.keys(HELP).sort()).toEqual([...IDS].sort())
  })
})
