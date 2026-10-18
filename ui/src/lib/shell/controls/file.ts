// Nomi dei file e testi di stato per la sezione File del pannello Controlli.
import type { RecordingView } from '../../types'
import { tr } from '../../i18n/msg/decoder'

export interface LimitOption {
  /** Chiave stabile, indipendente dalla lingua. */
  key: string
  label: string
  /** Durata massima in secondi; null per la registrazione manuale. */
  seconds: number | null
}

/** Durate di registrazione. Le etichette dipendono dalla lingua corrente, quindi è una funzione. */
export function limitOptions(): readonly LimitOption[] {
  return [
    { key: 'manual', label: tr('limitManual'), seconds: null },
    { key: '10', label: '10 s', seconds: 10 },
    { key: '60', label: '60 s', seconds: 60 },
  ]
}

function pad(n: number): string {
  return String(n).padStart(2, '0')
}

/** Nome di file con data e ora locali: `prefix-AAAAMMGG-hhmmss.ext`. */
export function defaultFileName(prefix: string, ext: string, d: Date): string {
  const date = `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`
  const time = `${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
  return `${prefix}-${date}-${time}.${ext}`
}

/** Parte del percorso dopo l'ultimo '/'. */
export function fileName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1)
}

/** Dimensione in base 1000: "512 B", "12,3 kB", "4,5 MB". */
export function formatBytes(n: number): string {
  if (n < 1000) return `${n} B`
  const kb = n / 1000
  if (kb < 999.95) return `${kb.toFixed(1).replace('.', ',')} kB`
  return `${(n / 1_000_000).toFixed(1).replace('.', ',')} MB`
}

export function recordingLabel(r: RecordingView | null): string {
  if (r === null) return tr('ready')
  if (r.error) return tr('recErrorText', { error: r.error })
  if (r.running) return tr('recording', { frames: r.frames })
  return tr('finished', { frames: r.frames })
}
