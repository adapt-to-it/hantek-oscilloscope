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

// Specchio di crates/h6022-view/src/acquisition.rs.
export type ModeName = 'auto' | 'normal' | 'single' | 'roll'
export type SlopeName = 'rising' | 'falling' | 'either'
export type PolarityName = 'positive' | 'negative'
export type LevelName = 'high' | 'low' | 'any'
export interface WidthSpec {
  op: 'less' | 'greater' | 'within' | 'outside'
  a: number
  b: number
}
export type TriggerSpec =
  | { kind: 'edge'; slope: SlopeName }
  | { kind: 'pulseWidth'; polarity: PolarityName; width: WidthSpec }
  | { kind: 'runt'; polarity: PolarityName; low: number; high: number }
  | { kind: 'window'; low: number; high: number; enter: boolean }
  | { kind: 'timeout'; slope: SlopeName; seconds: number }
  | { kind: 'pattern'; ch1: LevelName; ch2: LevelName }
export interface AcquisitionSettings {
  mode: ModeName
  probe: [number, number]
  ac: [boolean, boolean]
  pretrigger: number
  triggerSource: 1 | 2
  trigger: TriggerSpec
  triggerLevel: number
  hysteresis: number
  holdoff: number
}

// Specchio di crates/h6022-view/src/measures.rs.
export interface ChannelMeasures {
  vmax: number
  vmin: number
  vpp: number
  vmean: number
  vrms: number
  vacRms: number
  vtop: number
  vbase: number
  vamp: number
  frequency: number | null
  period: number | null
  dutyPositive: number | null
  pulseWidthPositive: number | null
  pulseWidthNegative: number | null
  riseTime: number | null
  fallTime: number | null
  overshootPct: number | null
  preshootPct: number | null
  risingEdges: number
  fallingEdges: number
}
export interface MeasureSnapshot {
  sequence: number
  channels: [ChannelMeasures | null, ChannelMeasures | null]
  phaseDeg: number | null
  delayS: number | null
}
