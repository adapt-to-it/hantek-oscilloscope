import type { CursorMode, CursorPositions } from '../types'
import { formatHz, formatTime, formatVolts } from '../shell/format'

/**
 * Righe del riquadro letture dei cursori, filtrate per modo.
 * Tempo: t1, t2, Δt, 1/Δt. Tensione: V1, V2, ΔV. Modo 'off': nessuna riga.
 */
export function cursorReadout(mode: CursorMode, pos: CursorPositions): { k: string; v: string }[] {
  const rows: { k: string; v: string }[] = []
  if (mode === 'off') return rows

  if (mode === 'time' || mode === 'both') {
    const dt = pos.t2 - pos.t1
    // Con Δt = 0 la frequenza non è definita.
    const inverse = dt === 0 ? '—' : formatHz(1 / Math.abs(dt))
    rows.push(
      { k: 't1', v: formatTime(pos.t1) },
      { k: 't2', v: formatTime(pos.t2) },
      { k: 'Δt', v: formatTime(dt) },
      { k: '1/Δt', v: inverse },
    )
  }

  if (mode === 'voltage' || mode === 'both') {
    rows.push(
      { k: 'V1', v: formatVolts(pos.v1) },
      { k: 'V2', v: formatVolts(pos.v2) },
      { k: 'ΔV', v: formatVolts(pos.v2 - pos.v1) },
    )
  }

  return rows
}
