import { Channel, invoke } from '@tauri-apps/api/core'
import type { AcquisitionSettings, DecodeParams, DecodeView, MeasureSnapshot, SpectrumView, Status, ViewSettings, ViewMode, WindowName, XyView } from './types'
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

export function getAcquisition(): Promise<AcquisitionSettings> {
  return invoke('get_acquisition')
}

export function setAcquisition(settings: AcquisitionSettings): Promise<void> {
  return invoke('set_acquisition', { settings })
}

export function getMeasurements(): Promise<MeasureSnapshot | null> {
  return invoke('measurements')
}

export function getSpectrum(window: WindowName): Promise<SpectrumView | null> {
  return invoke('spectrum', { window })
}

export function getXy(): Promise<XyView | null> {
  return invoke('xy')
}

/** Vista iniziale richiesta con H6022_MODE, se presente. */
export function getInitialMode(): Promise<ViewMode | null> {
  return invoke<ViewMode | null>('initial_mode')
}

/** Decodifica il frame più recente; null se non c'è ancora un frame. Rifiutata se i parametri non sono validi. */
export function decode(params: DecodeParams): Promise<DecodeView | null> {
  return invoke<DecodeView | null>('decode', { params })
}
