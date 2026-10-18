// Scale del display spettro: conversioni tra frequenza, dBV e pixel, e tacche.
// Funzioni pure, senza dipendenze dal DOM.

/** Estremi dell'asse y in dBV e passo delle righe. */
export const DB_MIN = -100
export const DB_MAX = 20
export const DB_STEP = 20

/**
 * Frequenza -> ascissa in pixel. Lineare: 0..nyquist sull'intera larghezza.
 * Logaritmica: da minHz (precondizione minHz >= 1) a nyquist; sotto minHz si ferma al bordo sinistro.
 */
export function freqToX(f: number, width: number, nyquist: number, log: boolean, minHz: number): number {
  if (log) {
    const lo = Math.log10(minHz)
    const span = Math.log10(nyquist) - lo
    if (!(span > 0)) return 0
    return ((Math.log10(Math.max(f, minHz)) - lo) / span) * width
  }
  if (!(nyquist > 0)) return 0
  return (f / nyquist) * width
}

/** Inversa di freqToX. */
export function xToFreq(x: number, width: number, nyquist: number, log: boolean, minHz: number): number {
  if (!(width > 0)) return log ? minHz : 0
  if (log) {
    const lo = Math.log10(minHz)
    const span = Math.log10(nyquist) - lo
    return 10 ** (lo + (x / width) * span)
  }
  return (x / width) * nyquist
}

/** dBV -> ordinata in pixel: DB_MAX in alto, DB_MIN in basso. Nessun ritaglio: lo fa il chiamante. */
export function dbToY(db: number, height: number): number {
  return ((DB_MAX - db) / (DB_MAX - DB_MIN)) * height
}

/** Potenze di dieci in [minHz, maxHz], in ordine crescente. Vuoto se l'intervallo non è valido. */
export function decadeTicks(minHz: number, maxHz: number): number[] {
  const ticks: number[] = []
  if (!(minHz > 0) || !(maxHz >= minHz) || !Number.isFinite(maxHz)) return ticks
  // Il margine evita che errori di arrotondamento escludano 10^k quando minHz è già una decade.
  for (let k = Math.ceil(Math.log10(minHz) - 1e-9); 10 ** k <= maxHz * (1 + 1e-9); k++) {
    ticks.push(10 ** k)
  }
  return ticks
}

/** Livello in dBV con segno, es. "-12.3 dBV" o "+20". Con unit = false il suffisso è omesso. */
export function formatDb(db: number, decimals = 1, unit = true): string {
  // Number(...) || 0 converte -0 in 0, così non compare "-0.0".
  const v = Number(db.toFixed(decimals)) || 0
  const sign = v > 0 ? '+' : ''
  return `${sign}${v.toFixed(decimals)}${unit ? ' dBV' : ''}`
}
