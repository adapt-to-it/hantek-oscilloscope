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
  /** Versione del firmware (es. 0x0210); null con il simulatore. */
  firmware: number | null
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
  /** Traccia matematica M (bit 2 della maschera), in volt sulla scala di CH1. */
  math: ChannelTrace | null
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
export type LogicOpName = 'and' | 'or'
export type CondName = 'rising' | 'falling' | 'either' | 'high' | 'low' | 'ignore'
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
  | { kind: 'logic'; op: LogicOpName; ch1: CondName; ch2: CondName; otherLevel: number }
  | { kind: 'sequence'; slopeA: SlopeName; slopeB: SlopeName; otherLevel: number; delay: number }
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

// Specchio di crates/h6022-view/src/analysis.rs.
export type WindowName = 'rectangular' | 'hann' | 'hamming' | 'blackmanHarris' | 'flatTop'
export interface PeakView {
  frequency: number
  dbv: number
}
export interface ChannelSpectrum {
  dbv: number[]
  peaks: PeakView[]
  thdPct: number | null
}
export interface SpectrumView {
  sequence: number
  binHz: number
  nyquistHz: number
  channels: [ChannelSpectrum | null, ChannelSpectrum | null]
}
export interface XyView {
  sequence: number
  x: number[]
  y: number[]
}
export type ViewMode = 'time' | 'spectrum' | 'xy'
export interface DisplayOptions {
  mode: ViewMode
  window: WindowName
  logFrequency: boolean
  /** Ultime schermate del display Tempo mostrate in trasparenza. */
  persistence: boolean
  cursors: CursorMode
  /** Canale a cui si riferiscono i cursori di tensione: 0 = CH1, 1 = CH2. */
  cursorChannel: 0 | 1
}
export type CursorMode = 'off' | 'time' | 'voltage' | 'both'
/** Posizione dei cursori: tempi in secondi relativi al trigger, tensioni in volt del canale scelto. */
export interface CursorPositions {
  t1: number
  t2: number
  v1: number
  v2: number
}

// Decoder: parametri e risposta dello strumento MCP `decode` (nomi snake_case come in Rust).
export type ProtocolName = 'uart' | 'i2c' | 'can' | 'onewire' | 'manchester' | 'pwm' | 'servo' | 'ir_nec' | 'ir_rc5'
export type ParityName = 'none' | 'even' | 'odd'
export interface DecodeParams {
  protocol: ProtocolName
  channel: 1 | 2
  threshold_v: number | null
  baud: number | null
  data_bits: number | null
  parity: ParityName | null
  stop_bits: number | null
  inverted: boolean | null
  bitrate: number | null
  bit_period_s: number | null
  ieee: boolean | null
  max_annotations: number | null
}
export interface AnnotationView {
  start_s: number
  end_s: number
  /** data, address, id, ack, nack, start, stop, command, crc, reset, error e altre. */
  kind: string
  text: string
  value: number | null
  error: boolean
}
export interface DecodeView {
  protocol: string
  channel: number
  annotations: AnnotationView[]
  truncated: boolean
  summary: string
}
/** Stato del pannello Decoder. `params` va al backend solo se `enabled`. */
export interface DecoderSettings {
  enabled: boolean
  params: DecodeParams
}

// Registrazione ed export: tipi di crates/h6022-server/src/api.rs e api_decode.rs (campi snake_case).
export type RecordFormat = 'csv' | 'npy' | 'wav'
export type ExportFormat = 'vcd' | 'sr'
/** `path` relativo: il file va in ~/h6022-recordings. */
export interface RecordParams {
  path: string
  format: RecordFormat | null
  max_frames: number | null
  max_seconds: number | null
}
export interface RecordingView {
  path: string
  format: RecordFormat
  frames: number
  running: boolean
  error: string | null
}
export interface RecordSummaryView {
  path: string
  format: RecordFormat
  frames: number
  channels: number
  sample_rate_hz: number
  samples_per_channel: number
}
export interface ExportParams {
  path: string
  format: ExportFormat
  decode: DecodeParams | null
}
export interface ExportView {
  path: string
  bytes: number
}

// Strumenti: tipi di crates/h6022-view/src/tools.rs (camelCase).
export type MathOp = 'off' | 'add' | 'sub' | 'mul' | 'derivative' | 'integral'
export type FilterType = 'none' | 'lowPass' | 'highPass' | 'bandPass'
export interface FilterSettings {
  channel: 0 | 1
  kind: FilterType
  cutoffHz: number
}
export interface ToolSettings {
  math: MathOp
  filter: FilterSettings
}
/** Risposta del comando `classify` (snake_case, da crates/h6022-server/src/api_decode.rs). */
export interface ClassificationView {
  /** dc, noise, analog, clock, pwm, serial, burst, other. */
  class: string
  confidence: number
  summary: string
  baud_estimate: number | null
  suggested_decoders: string[]
}
