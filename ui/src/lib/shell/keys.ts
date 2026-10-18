import type { Status, ViewMode } from '../types'

/** Comandi inviati a `run_command`. */
export type RunCommand = 'run' | 'stop' | 'single'

/** Azioni da tastiera: i comandi del motore più il cambio di vista, che non passa da `run_command`. */
/** I spiegazioni, M traccia matematica, P immagine del display. */
export type ShortcutCommand = RunCommand | 'view' | 'explain' | 'math' | 'screenshot'

/** Il comando che il pulsante Run/Stop invia nello stato attuale: Stop se il motore gira o è armato. */
export function toggleCommand(runState: Status['runState']): RunCommand {
  return runState === 'stopped' ? 'run' : 'stop'
}

/** Vista successiva nel ciclo del tasto F: Tempo, Spettro, XY, poi di nuovo Tempo. */
export function nextMode(mode: ViewMode): ViewMode {
  if (mode === 'time') return 'spectrum'
  if (mode === 'spectrum') return 'xy'
  return 'time'
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
 * Azione associata al tasto: Spazio alterna run/stop secondo `runState`, S avvia il singolo, F cambia vista,
 * I attiva le spiegazioni, M la traccia matematica, P salva l'immagine del display.
 * Ritorna null se il tasto non è una scorciatoia o se il focus è su un campo di input.
 */
export function shortcutFor(input: KeyInput, runState: Status['runState']): ShortcutCommand | null {
  if (input.ctrlKey || input.metaKey || input.altKey) return null
  if (input.target) {
    if (input.target.isContentEditable) return null
    if (input.target.tagName && FIELD_TAGS.has(input.target.tagName.toUpperCase())) return null
  }
  if (input.key === ' ') return toggleCommand(runState)
  if (input.key === 's' || input.key === 'S') return 'single'
  if (input.key === 'f' || input.key === 'F') return 'view'
  if (input.key === 'i' || input.key === 'I') return 'explain'
  if (input.key === 'm' || input.key === 'M') return 'math'
  if (input.key === 'p' || input.key === 'P') return 'screenshot'
  return null
}
