// Testi e colori della barra superiore: pillola di stato, chip MCP e versione del firmware.
import type { Status } from '../types'
import { tr } from '../i18n/msg/shell'

export interface Badge {
  text: string
  color: string
}

// Prefissi e valori che arrivano dal backend: si confrontano così come sono, non si traducono.
const MCP_ADDRESS_PREFIX = 'MCP http://'
const MCP_ERROR_PREFIX = 'MCP non avviato: '

/** Stato dell'acquisizione mostrato nella pillola accanto a Run e Singolo. */
export function runPill(status: Status | null, triggered: boolean | null): Badge {
  if (!status) return { text: tr('starting'), color: '#8a94a1' }
  if (status.runState === 'stopped') {
    if (status.triggerMode === 'single') return { text: tr('pillSingleDone'), color: '#ff9f43' }
    return { text: tr('pillStopped'), color: '#9aa3af' }
  }
  if (status.triggerMode === 'roll') return { text: tr('pillRoll'), color: '#3fd68a' }
  if (triggered === true) return { text: tr('pillTriggered'), color: '#3fd68a' }
  if (status.triggerMode === 'auto') return { text: tr('pillAuto'), color: '#ff9f43' }
  return { text: tr('pillWaiting'), color: '#ff9f43' }
}

/** Stato del server MCP. Il campo `server` arriva dal backend come testo già formattato. */
export function mcpChip(server: string): Badge {
  if (server.startsWith(MCP_ADDRESS_PREFIX)) {
    return { text: tr('mcpAddress', { indirizzo: server.slice(MCP_ADDRESS_PREFIX.length) }), color: '#3fd68a' }
  }
  if (server === 'MCP disattivato') return { text: tr('mcpOff'), color: '#9aa3af' }
  const reason = server.startsWith(MCP_ERROR_PREFIX) ? server.slice(MCP_ERROR_PREFIX.length) : server
  return { text: tr('mcpError', { motivo: reason }), color: '#ff7a6b' }
}

/** Versione firmware da 0x0210 a "2.10": il byte alto è la major, il basso la minor in due cifre. */
export function firmwareLabel(firmware: number | null): string {
  if (firmware === null) return '—'
  const major = firmware >> 8
  const minor = firmware & 0xff
  return `${major.toString(16)}.${minor.toString(16).padStart(2, '0')}`
}
