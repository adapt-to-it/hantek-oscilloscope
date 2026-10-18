import type { AnnotationView } from '../types'

export interface StripBlock {
  /** Inizio del blocco, in percentuale della finestra (0..100). */
  left: number
  /** Larghezza del blocco, in percentuale della finestra (0..100). */
  width: number
  /** Testo mostrato nel blocco; vuoto se non ci sta in larghezza. */
  text: string
  bg: string
  fg: string
  /** Testo completo per il tooltip. */
  title: string
}

/** Blocchi più stretti di questa percentuale vengono scartati. */
const MIN_WIDTH_PCT = 0.25
/** Larghezza minima, in percentuale, di uno span nullo (start = end, es. start/stop I2C). */
const NULL_SPAN_PCT = 0.6
/** Stima della larghezza di un carattere in mono 12px, e padding del blocco, in px. */
const CHAR_PX = 7.5
const PADDING_PX = 10

/** Colori [bg, fg] per kind, come da @colori. */
const PALETTE = new Map<string, [string, string]>([
  ['data', ['#6fc4f0', '#06141c']],
  ['address', ['#ff9f43', '#241200']],
  ['id', ['#b794f6', '#1a0f33']],
  ['ack', ['#5fd68d', '#06210f']],
  ['nack', ['#ff9a8a', '#2a0e0a']],
  ['start', ['#8a94a1', '#0b0d10']],
  ['stop', ['#8a94a1', '#0b0d10']],
  ['command', ['#4fd6c4', '#032420']],
  ['crc', ['#f5d742', '#241d00']],
  ['reset', ['#b08968', '#1a0f05']],
  ['error', ['#e5392b', '#ffffff']],
])
const DEFAULT_COLORS = PALETTE.get('data')!
const ERROR_COLORS = PALETTE.get('error')!

/**
 * Converte le annotazioni nei blocchi della striscia sulla finestra [tStart, tEnd].
 * Restituisce [] se la finestra è vuota o non valida.
 */
export function stripBlocks(
  anns: AnnotationView[],
  tStart: number,
  tEnd: number,
  widthPx: number,
): StripBlock[] {
  const span = tEnd - tStart
  if (!(span > 0)) return []

  const blocks: StripBlock[] = []
  for (const a of anns) {
    const l = ((a.start_s - tStart) / span) * 100
    const nullSpan = a.end_s === a.start_s
    const r = nullSpan ? l + NULL_SPAN_PCT : ((a.end_s - tStart) / span) * 100

    // Ritaglio alla finestra: i blocchi fuori hanno larghezza <= 0 e cadono nello scarto.
    const left = Math.max(0, l)
    const right = Math.min(100, r)
    const width = right - left
    if (width < MIN_WIDTH_PCT) continue

    const isError = a.error || a.kind === 'error'
    const [bg, fg] = isError ? ERROR_COLORS : (PALETTE.get(a.kind) ?? DEFAULT_COLORS)
    const fits = (width / 100) * widthPx >= a.text.length * CHAR_PX + PADDING_PX
    const text = fits ? a.text : ''
    const title = isError ? (a.text ? `Errore: ${a.text}` : 'Errore') : a.text

    blocks.push({ left, width, text, bg, fg, title })
  }
  return blocks
}
