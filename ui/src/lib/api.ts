import { Channel, invoke } from '@tauri-apps/api/core'
import type { Status, ViewSettings } from './types'
import type { RunCommand } from './shell/keys'

// Wrapper tipizzati dei comandi definiti in crates/h6022-app/src/commands.rs.
// Tauri converte gli argomenti snake_case in camelCase: qui i nomi sono già camelCase.

export function getStatus(): Promise<Status> {
  return invoke<Status>('status')
}

export function getView(): Promise<ViewSettings> {
  return invoke<ViewSettings>('get_view')
}

/** Rifiutata dal motore: la vista precedente resta in vigore. */
export function setView(view: ViewSettings): Promise<void> {
  return invoke<void>('set_view', { view })
}

export function runCommand(command: RunCommand): Promise<void> {
  return invoke<void>('run_command', { command })
}

export function setColumns(columns: number): Promise<void> {
  return invoke<void>('set_columns', { columns })
}

/** Riceve ogni frame del display codificato in binario (vedi crates/h6022-view/src/trace.rs). */
export function subscribeDisplay(cb: (buf: ArrayBuffer) => void): Promise<void> {
  const channel = new Channel<ArrayBuffer>()
  channel.onmessage = cb
  return invoke<void>('subscribe_display', { channel })
}

export function getTriggerLevel(): Promise<number> {
  return invoke<number>('get_trigger_level')
}

export function setTriggerLevel(volts: number): Promise<void> {
  return invoke<void>('set_trigger_level', { volts })
}
