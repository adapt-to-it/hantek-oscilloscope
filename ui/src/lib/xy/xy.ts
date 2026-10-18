import type { ChannelSettings } from '../types'

/** Divisioni per lato della griglia XY: la vista è quadrata, 8 × 8. */
export const XY_DIVISIONS = 8

function assertScale(ch: ChannelSettings, size: number): void {
  if (!Number.isFinite(ch.voltsPerDiv) || ch.voltsPerDiv <= 0) {
    throw new RangeError(`voltsPerDiv non valido: ${ch.voltsPerDiv}`)
  }
  if (!Number.isFinite(size) || size <= 0) {
    throw new RangeError(`dimensione non valida: ${size}`)
  }
}

/**
 * Converte una tensione in una coordinata lungo un asse di `size` pixel CSS.
 * Lo zero sta al centro dell'asse, spostato dall'offset del canale; la coordinata cresce
 * verso destra. Per l'asse verticale del canvas (y cresce in basso) si usa `size - voltsToPx(...)`.
 */
export function voltsToPx(v: number, ch: ChannelSettings, size: number): number {
  assertScale(ch, size)
  const divPx = size / XY_DIVISIONS
  return size / 2 + (v / ch.voltsPerDiv + ch.offsetDiv) * divPx
}

/** Inversa di `voltsToPx`: coordinata lungo l'asse di `size` pixel CSS, in volt. */
export function pxToVolts(px: number, ch: ChannelSettings, size: number): number {
  assertScale(ch, size)
  const divPx = size / XY_DIVISIONS
  return ((px - size / 2) / divPx - ch.offsetDiv) * ch.voltsPerDiv
}
