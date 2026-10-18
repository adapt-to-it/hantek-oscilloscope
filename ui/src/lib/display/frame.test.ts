import { describe, expect, it } from 'vitest'
import { decodeDisplay } from './frame'

/** Costruisce un frame nel formato di encode_display: header di 36 byte e poi i float f32. */
function build(mask: number, columns: number, floats: number[], sequence = 7n): ArrayBuffer {
  const buf = new ArrayBuffer(36 + floats.length * 4)
  const view = new DataView(buf)
  view.setBigUint64(0, sequence, true)
  view.setFloat64(8, -0.001, true)
  view.setFloat64(16, 0.002, true)
  view.setFloat64(24, 1e6, true)
  view.setUint8(32, 1)
  view.setUint8(33, mask)
  view.setUint16(34, columns, true)
  floats.forEach((f, i) => view.setFloat32(36 + i * 4, f, true))
  return buf
}

describe('decodeDisplay', () => {
  it('decodifica due canali con header e colonne', () => {
    // CH1: min [-1, 0.5], max [1, 2]; CH2: min [3, 4], max [5, 6]
    const frame = decodeDisplay(build(3, 2, [-1, 0.5, 1, 2, 3, 4, 5, 6]))

    expect(frame.sequence).toBe(7)
    expect(frame.tStart).toBeCloseTo(-0.001)
    expect(frame.tEnd).toBeCloseTo(0.002)
    expect(frame.sampleRate).toBe(1e6)
    expect(frame.triggered).toBe(true)
    expect(frame.columns).toBe(2)

    const [ch1, ch2] = frame.traces
    expect(ch1).not.toBeNull()
    expect(Array.from(ch1!.min)).toEqual([-1, 0.5])
    expect(Array.from(ch1!.max)).toEqual([1, 2])
    expect(Array.from(ch2!.min)).toEqual([3, 4])
    expect(Array.from(ch2!.max)).toEqual([5, 6])
  })

  it('decodifica un solo canale CH1', () => {
    // CH1: min [0.5, 2], max [1, 3]
    const frame = decodeDisplay(build(1, 2, [0.5, 2, 1, 3]))
    expect(frame.traces[0]).not.toBeNull()
    expect(frame.traces[1]).toBeNull()
    expect(Array.from(frame.traces[0]!.min)).toEqual([0.5, 2])
    expect(Array.from(frame.traces[0]!.max)).toEqual([1, 3])
  })

  it('decodifica un solo canale CH2', () => {
    // CH2: min [7, 9], max [8, 10]
    const frame = decodeDisplay(build(2, 2, [7, 9, 8, 10]))
    expect(frame.traces[0]).toBeNull()
    expect(Array.from(frame.traces[1]!.min)).toEqual([7, 9])
    expect(Array.from(frame.traces[1]!.max)).toEqual([8, 10])
  })

  it('conserva NaN per le colonne senza dati', () => {
    // CH1: min [NaN, 2], max [1, NaN]
    const frame = decodeDisplay(build(1, 2, [NaN, 2, 1, NaN]))
    const ch1 = frame.traces[0]!
    expect(Number.isNaN(ch1.min[0])).toBe(true)
    expect(ch1.max[0]).toBe(1)
    expect(ch1.min[1]).toBe(2)
    expect(Number.isNaN(ch1.max[1])).toBe(true)
  })

  it('converte il sequence u64 in Number', () => {
    const frame = decodeDisplay(build(1, 1, [0, 0], 2n ** 40n + 5n))
    expect(frame.sequence).toBe(2 ** 40 + 5)
  })

  it('lancia Error se il buffer è troncato rispetto alle tracce dichiarate', () => {
    const full = build(3, 2, [-1, 0.5, 1, 2, 3, 4, 5, 6])
    const truncated = full.slice(0, full.byteLength - 1)
    expect(() => decodeDisplay(truncated)).toThrow(Error)
  })

  it('lancia Error se il buffer è più corto dell\'header', () => {
    expect(() => decodeDisplay(new ArrayBuffer(35))).toThrow(Error)
  })
})
