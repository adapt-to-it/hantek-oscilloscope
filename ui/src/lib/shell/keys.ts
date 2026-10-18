import type { Status } from '../types'

/** Comandi inviati a `run_command`. */
export type RunCommand = 'run' | 'stop' | 'single'

/** Il comando che il pulsante Run/Stop invia nello stato attuale: Stop se il motore gira o è armato. */
export function toggleCommand(runState: Status['runState']): RunCommand {
  return runState === 'stopped' ? 'run' : 'stop'
}

/** Sottoinsieme dell'evento da tastiera usato per decidere l'azione. */
export interface KeyInput {
  key: string
  ctrlKey?: boolean
  metaKey?: boolean
  altKey?: boolean
  /** Elemento con il focus; null se non c'è. */
  target: { tagName?: string; isContentEditable?: boolean } | null
}

const FIELD_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT'])

/**
 * Azione associata al tasto: Spazio alterna run/stop secondo `runState`, S avvia il singolo.
 * Ritorna null se il tasto non è una scorciatoia o se il focus è su un campo di input.
 */
export function shortcutFor(input: KeyInput, runState: Status['runState']): RunCommand | null {
  if (input.ctrlKey || input.metaKey || input.altKey) return null
  if (input.target) {
    if (input.target.isContentEditable) return null
    if (input.target.tagName && FIELD_TAGS.has(input.target.tagName.toUpperCase())) return null
  }
  if (input.key === ' ') return toggleCommand(runState)
  if (input.key === 's' || input.key === 'S') return 'single'
  return null
}
