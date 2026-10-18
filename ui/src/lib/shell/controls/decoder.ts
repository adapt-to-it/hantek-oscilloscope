// Protocolli, parametri e riepilogo del decoder per il pannello Controlli. Specchio di protocol_of in crates/h6022-server/src/decode_tools.rs.
import type { DecodeParams, DecoderSettings, ProtocolName } from '../../types'

export const PROTOCOLS: readonly { value: ProtocolName; label: string }[] = [
  { value: 'uart', label: 'UART' },
  { value: 'i2c', label: 'I2C' },
  { value: 'can', label: 'CAN' },
  { value: 'onewire', label: '1-Wire' },
  { value: 'manchester', label: 'Manchester' },
  { value: 'pwm', label: 'PWM' },
  { value: 'servo', label: 'Servo' },
  { value: 'ir_nec', label: 'IR NEC' },
  { value: 'ir_rc5', label: 'IR RC5' },
]

/** Velocità selezionabili con lo stepper, in baud. */
export const BAUDS: readonly number[] = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200]

/** Campi di DecodeParams che ciascun protocollo legge, oltre a protocollo, canale, soglia e max_annotations. */
const OWN_FIELDS: Record<ProtocolName, readonly (keyof DecodeParams)[]> = {
  uart: ['baud', 'data_bits', 'parity', 'stop_bits', 'inverted'],
  i2c: [],
  can: ['bitrate', 'inverted'],
  onewire: [],
  manchester: ['bit_period_s', 'ieee'],
  pwm: [],
  servo: [],
  ir_nec: [],
  ir_rc5: [],
}

const CAN_DEFAULT_BITRATE = 500000

export function defaultDecoder(): DecoderSettings {
  return {
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
  }
}

/** Passa alla velocità successiva o precedente in lista, partendo da quella più vicina a `current`. */
export function stepBaud(current: number, dir: 1 | -1): number {
  let nearest = 0
  for (let i = 1; i < BAUDS.length; i++) {
    if (Math.abs(BAUDS[i] - current) < Math.abs(BAUDS[nearest] - current)) nearest = i
  }
  const next = Math.min(BAUDS.length - 1, Math.max(0, nearest + dir))
  return BAUDS[next]
}

/** Cambia protocollo: azzera i campi che il nuovo protocollo non usa; il CAN parte da 500 kbit/s. */
export function withProtocol(s: DecoderSettings, protocol: ProtocolName): DecoderSettings {
  const uses = new Set<keyof DecodeParams>(OWN_FIELDS[protocol])
  const pick = <K extends keyof DecodeParams>(key: K): DecodeParams[K] | null =>
    uses.has(key) ? s.params[key] : null
  const params: DecodeParams = {
    protocol,
    channel: s.params.channel,
    threshold_v: s.params.threshold_v,
    max_annotations: s.params.max_annotations,
    baud: pick('baud'),
    data_bits: pick('data_bits'),
    parity: pick('parity'),
    stop_bits: pick('stop_bits'),
    inverted: pick('inverted'),
    bitrate: pick('bitrate'),
    bit_period_s: pick('bit_period_s'),
    ieee: pick('ieee'),
  }
  if (protocol === 'can' && params.bitrate === null) params.bitrate = CAN_DEFAULT_BITRATE
  return { ...s, params }
}

export function decoderSummary(s: DecoderSettings): string {
  if (!s.enabled) return 'Spento'
  const label = PROTOCOLS.find((p) => p.value === s.params.protocol)?.label ?? s.params.protocol
  return `${label} · CH${s.params.channel}`
}

/** Per l'I2C il canale scelto è SCL e l'altro è SDA; per gli altri protocolli è il solo canale. */
export function channelLabel(params: DecodeParams): string {
  if (params.protocol !== 'i2c') return `CH${params.channel}`
  const sda = params.channel === 1 ? 2 : 1
  return `CH${params.channel} = SCL, CH${sda} = SDA`
}
