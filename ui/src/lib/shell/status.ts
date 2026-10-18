// Testi e colori della barra superiore: pillola di stato, chip MCP e versione del firmware.
import type { Status } from '../types'

export interface Badge {
  text: string
  color: string
}

const MCP_ADDRESS_PREFIX = 'MCP http://'
const MCP_ERROR_PREFIX = 'MCP non avviato: '

/** Stato dell'acquisizione mostrato nella pillola accanto a Run e Singolo. */
export function runPill(status: Status | null, triggered: boolean | null): Badge {
  if (!status) return { text: 'In avvio', color: '#8a94a1' }
  if (status.runState === 'stopped') {
    if (status.triggerMode === 'single') return { text: 'Singolo acquisito', color: '#ff9f43' }
    return { text: 'Acquisizione ferma', color: '#9aa3af' }
  }
  if (status.triggerMode === 'roll') return { text: 'Roll · scorrimento continuo', color: '#3fd68a' }
  if (triggered === true) return { text: 'Agganciato al trigger', color: '#3fd68a' }
  if (status.triggerMode === 'auto') return { text: 'Auto · senza trigger', color: '#ff9f43' }
  return { text: 'In attesa di trigger', color: '#ff9f43' }
}

/** Stato del server MCP. Il campo `server` arriva dal backend come testo già formattato. */
export function mcpChip(server: string): Badge {
  if (server.startsWith(MCP_ADDRESS_PREFIX)) {
    return { text: `MCP · ${server.slice(MCP_ADDRESS_PREFIX.length)}`, color: '#3fd68a' }
  }
  if (server === 'MCP disattivato') return { text: 'MCP spento', color: '#9aa3af' }
  const reason = server.startsWith(MCP_ERROR_PREFIX) ? server.slice(MCP_ERROR_PREFIX.length) : server
  return { text: `MCP: ${reason}`, color: '#ff7a6b' }
}

/** Versione firmware da 0x0210 a "2.10": il byte alto è la major, il basso la minor in due cifre. */
export function firmwareLabel(firmware: number | null): string {
  if (firmware === null) return '—'
  const major = firmware >> 8
  const minor = firmware & 0xff
  return `${major.toString(16)}.${minor.toString(16).padStart(2, '0')}`
}
