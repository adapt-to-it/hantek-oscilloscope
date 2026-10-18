import type { ChannelTrace, DisplayFrame } from '../types'

/** Lunghezza dell'header in byte, come in crates/h6022-view/src/trace.rs. */
const HEADER_LEN = 36
/** Bit della maschera per la traccia matematica M (dopo CH1 = bit 0 e CH2 = bit 1). */
const MATH_BIT = 2

/**
 * Decodifica un frame del display prodotto da `encode_display` (little endian).
 * Lancia Error se il buffer è più corto di quanto dichiarano header e maschera canali.
 */
export function decodeDisplay(buf: ArrayBuffer): DisplayFrame {
  if (buf.byteLength < HEADER_LEN) {
    throw new Error(`frame display troncato: ${buf.byteLength} byte, header di ${HEADER_LEN}`)
  }
  const view = new DataView(buf)
  const mask = view.getUint8(33)
  const columns = view.getUint16(34, true)
  const present: Array<0 | 1 | 2> = ([0, 1, MATH_BIT] as const).filter((ch) => (mask & (1 << ch)) !== 0)
  const expected = HEADER_LEN + present.length * columns * 8
  if (buf.byteLength < expected) {
    throw new Error(`frame display troncato: ${buf.byteLength} byte, attesi ${expected}`)
  }

  const traces: [ChannelTrace | null, ChannelTrace | null] = [null, null]
  let math: ChannelTrace | null = null
  let offset = HEADER_LEN
  for (const ch of present) {
    const min = new Float32Array(columns)
    const max = new Float32Array(columns)
    for (let c = 0; c < columns; c++) min[c] = view.getFloat32(offset + c * 4, true)
    offset += columns * 4
    for (let c = 0; c < columns; c++) max[c] = view.getFloat32(offset + c * 4, true)
    offset += columns * 4
    if (ch === MATH_BIT) math = { min, max }
    else traces[ch] = { min, max }
  }

  return {
    sequence: Number(view.getBigUint64(0, true)),
    tStart: view.getFloat64(8, true),
    tEnd: view.getFloat64(16, true),
    sampleRate: view.getFloat64(24, true),
    triggered: (view.getUint8(32) & 1) !== 0,
    columns,
    traces,
    math,
  }
}
