// Tipi condivisi tra backend Tauri e frontend. Speculari a crates/h6022-view/src/settings.rs
// e crates/h6022-app/src/commands.rs (serde camelCase).

export interface ChannelSettings {
  enabled: boolean
  voltsPerDiv: number
  offsetDiv: number
}

export interface ViewSettings {
  channels: [ChannelSettings, ChannelSettings]
  /** Secondi per divisione. */
  timePerDiv: number
  /** Secondi; positivo = la vista va a destra. */
  horizontalOffset: number
}

export interface Status {
  source: 'hardware' | 'simulator'
  runState: 'running' | 'armed' | 'stopped'
  triggerMode: 'auto' | 'normal' | 'single' | 'roll'
  frames: number
  sampleRateHz: number
  sampleRateMeasuredHz: number
  recordLength: number
  server: string
}

/** Un frame del display, decodificato dal formato binario di crates/h6022-view/src/trace.rs. */
export interface DisplayFrame {
  sequence: number
  tStart: number
  tEnd: number
  sampleRate: number
  triggered: boolean
  columns: number
  /** Indice 0 = CH1, 1 = CH2; null se il canale manca nel frame. NaN = colonna senza dati. */
  traces: [ChannelTrace | null, ChannelTrace | null]
}

export interface ChannelTrace {
  min: Float32Array
  max: Float32Array
}

export const DIVISIONS_X = 10
export const DIVISIONS_Y = 8
export const CH_COLORS = ['#f5c842', '#43c6f3'] as const
