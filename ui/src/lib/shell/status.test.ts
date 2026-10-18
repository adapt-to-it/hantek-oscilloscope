import { describe, expect, it } from 'vitest'
import type { Status } from '../types'
import { firmwareLabel, mcpChip, runPill } from './status'

function status(over: Partial<Status> = {}): Status {
  return {
    source: 'simulator',
    firmware: null,
    runState: 'running',
    triggerMode: 'normal',
    frames: 0,
    sampleRateHz: 48e6,
    sampleRateMeasuredHz: 48e6,
    recordLength: 4096,
    server: 'MCP http://127.0.0.1:7878',
    ...over,
  }
}

describe('runPill', () => {
  it('mostra l\'avvio finché non arriva lo stato', () => {
    expect(runPill(null, null)).toEqual({ text: 'In avvio', color: '#8a94a1' })
  })

  it('distingue il singolo completato dalla acquisizione ferma', () => {
    expect(runPill(status({ runState: 'stopped', triggerMode: 'single' }), null))
      .toEqual({ text: 'Singolo acquisito', color: '#ff9f43' })
    expect(runPill(status({ runState: 'stopped' }), true))
      .toEqual({ text: 'Acquisizione ferma', color: '#9aa3af' })
  })

  it('mostra roll, trigger agganciato e auto quando è in esecuzione', () => {
    expect(runPill(status({ triggerMode: 'roll' }), false).text).toBe('Roll · scorrimento continuo')
    expect(runPill(status({ triggerMode: 'normal' }), true))
      .toEqual({ text: 'Agganciato al trigger', color: '#3fd68a' })
    expect(runPill(status({ triggerMode: 'auto' }), false))
      .toEqual({ text: 'Auto · senza trigger', color: '#ff9f43' })
  })

  it('attende il trigger se non è agganciato o non si sa', () => {
    expect(runPill(status({ triggerMode: 'normal' }), false))
      .toEqual({ text: 'In attesa di trigger', color: '#ff9f43' })
    expect(runPill(status({ triggerMode: 'normal' }), null).text).toBe('In attesa di trigger')
  })
})

describe('mcpChip', () => {
  it('mostra l\'indirizzo senza http:// quando il server è attivo', () => {
    expect(mcpChip('MCP http://127.0.0.1:7878'))
      .toEqual({ text: 'MCP · 127.0.0.1:7878', color: '#3fd68a' })
  })

  it('distingue server spento e server non avviato', () => {
    expect(mcpChip('MCP disattivato')).toEqual({ text: 'MCP spento', color: '#9aa3af' })
    expect(mcpChip('MCP non avviato: porta occupata'))
      .toEqual({ text: 'MCP: porta occupata', color: '#ff7a6b' })
  })
})

describe('firmwareLabel', () => {
  it('legge la versione dal valore esadecimale', () => {
    expect(firmwareLabel(0x0210)).toBe('2.10')
    expect(firmwareLabel(0x0205)).toBe('2.05')
  })

  it('usa un trattino senza firmware', () => {
    expect(firmwareLabel(null)).toBe('—')
  })
})
