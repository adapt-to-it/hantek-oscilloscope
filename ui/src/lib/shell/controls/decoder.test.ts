import { describe, expect, it } from 'vitest'
import type { DecoderSettings } from '../../types'
import { BAUDS, PROTOCOLS, channelLabel, decoderSummary, defaultDecoder, stepBaud, withProtocol } from './decoder'

function settings(patch: Partial<DecoderSettings['params']>, enabled = true): DecoderSettings {
  const base = defaultDecoder()
  return { ...base, enabled, params: { ...base.params, ...patch } }
}

describe('PROTOCOLS', () => {
  it('elenca i nove protocolli nell\'ordine del pannello', () => {
    expect(PROTOCOLS.map((p) => p.label)).toEqual([
      'UART',
      'I2C',
      'CAN',
      '1-Wire',
      'Manchester',
      'PWM',
      'Servo',
      'IR NEC',
      'IR RC5',
    ])
    expect(PROTOCOLS.map((p) => p.value)[0]).toBe('uart')
  })
})

describe('defaultDecoder', () => {
  it('è spento, UART sul canale 1, con 500 annotazioni massime e il resto non impostato', () => {
    expect(defaultDecoder()).toEqual({
      enabled: false,
      params: {
        protocol: 'uart',
        channel: 1,
        threshold_v: null,
        baud: null,
        data_bits: null,
        parity: null,
        stop_bits: null,
        inverted: null,
        bitrate: null,
        bit_period_s: null,
        ieee: null,
        max_annotations: 500,
      },
    })
  })

  it('restituisce un oggetto nuovo a ogni chiamata', () => {
    const a = defaultDecoder()
    const b = defaultDecoder()
    expect(a).not.toBe(b)
    expect(a.params).not.toBe(b.params)
  })
})

describe('BAUDS e stepBaud', () => {
  it('elenca le velocità standard in ordine crescente', () => {
    expect(BAUDS).toEqual([1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200])
  })

  it('scorre alla velocità successiva e precedente', () => {
    expect(stepBaud(9600, 1)).toBe(19200)
    expect(stepBaud(9600, -1)).toBe(4800)
  })

  it('parte dalla velocità più vicina se il valore non è in lista', () => {
    expect(stepBaud(10000, 1)).toBe(19200)
    expect(stepBaud(10000, -1)).toBe(4800)
  })

  it('si ferma agli estremi della lista', () => {
    expect(stepBaud(115200, 1)).toBe(115200)
    expect(stepBaud(1200, -1)).toBe(1200)
    expect(stepBaud(1, -1)).toBe(1200)
  })
})

describe('withProtocol', () => {
  it('cambia il protocollo senza modificare il canale, la soglia e max_annotations', () => {
    const s = settings({ channel: 2, threshold_v: 0.8, max_annotations: 120 })
    const next = withProtocol(s, 'i2c')
    expect(next.params.protocol).toBe('i2c')
    expect(next.params.channel).toBe(2)
    expect(next.params.threshold_v).toBe(0.8)
    expect(next.params.max_annotations).toBe(120)
  })

  it('mantiene lo stato di attivazione', () => {
    expect(withProtocol(settings({}, true), 'pwm').enabled).toBe(true)
    expect(withProtocol(settings({}, false), 'pwm').enabled).toBe(false)
  })

  it('azzera i parametri UART uscendo dall\'UART', () => {
    const s = settings({ protocol: 'uart', baud: 9600, data_bits: 7, parity: 'even', stop_bits: 2, inverted: true })
    const next = withProtocol(s, 'servo')
    expect(next.params.baud).toBeNull()
    expect(next.params.data_bits).toBeNull()
    expect(next.params.parity).toBeNull()
    expect(next.params.stop_bits).toBeNull()
    expect(next.params.inverted).toBeNull()
  })

  it('conserva inverted passando da UART a CAN, perché il campo serve a entrambi', () => {
    const s = settings({ protocol: 'uart', baud: 9600, inverted: true })
    const next = withProtocol(s, 'can')
    expect(next.params.inverted).toBe(true)
    expect(next.params.baud).toBeNull()
  })

  it('imposta 500 kbit/s al CAN se il bitrate è vuoto', () => {
    expect(withProtocol(settings({}), 'can').params.bitrate).toBe(500000)
  })

  it('conserva il bitrate CAN già scelto', () => {
    const s = settings({ protocol: 'can', bitrate: 250000 })
    expect(withProtocol(s, 'can').params.bitrate).toBe(250000)
  })

  it('azzera il bitrate uscendo dal CAN e lo riempie di nuovo tornando al CAN', () => {
    const left = withProtocol(settings({ protocol: 'can', bitrate: 1000000 }), 'uart')
    expect(left.params.bitrate).toBeNull()
    expect(withProtocol(left, 'can').params.bitrate).toBe(500000)
  })

  it('non tocca la soglia automatica', () => {
    expect(withProtocol(settings({ threshold_v: null }), 'manchester').params.threshold_v).toBeNull()
  })

  it('non modifica l\'oggetto di partenza', () => {
    const s = settings({ protocol: 'uart', baud: 9600 })
    withProtocol(s, 'can')
    expect(s.params.protocol).toBe('uart')
    expect(s.params.baud).toBe(9600)
  })
})

describe('decoderSummary', () => {
  it('dice Spento quando il decoder è disattivato', () => {
    expect(decoderSummary(defaultDecoder())).toBe('Spento')
  })

  it('mostra protocollo e canale quando è attivo', () => {
    expect(decoderSummary(settings({ protocol: 'ir_rc5', channel: 2 }))).toBe('IR RC5 · CH2')
    expect(decoderSummary(settings({ protocol: 'uart', channel: 1 }))).toBe('UART · CH1')
  })
})

describe('channelLabel', () => {
  it('mostra solo il canale per i protocolli diversi dall\'I2C', () => {
    expect(channelLabel(settings({ protocol: 'uart', channel: 1 }).params)).toBe('CH1')
    expect(channelLabel(settings({ protocol: 'can', channel: 2 }).params)).toBe('CH2')
  })

  it('per l\'I2C indica quale canale è SCL e quale SDA', () => {
    expect(channelLabel(settings({ protocol: 'i2c', channel: 1 }).params)).toBe('CH1 = SCL, CH2 = SDA')
    expect(channelLabel(settings({ protocol: 'i2c', channel: 2 }).params)).toBe('CH2 = SCL, CH1 = SDA')
  })
})
