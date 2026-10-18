// Righe della griglia misure: testi già formattati per CH1 e CH2. Specchio di measure_panel.rs.
import type { ChannelMeasures, MeasureSnapshot } from '../types'
import { formatHz, formatTime, formatVolts } from '../shell/format'

export interface MeasureRow {
  label: string
  ch1: string
  ch2: string
  /** Campo numerico da cui deriva la riga (per l'andamento nel tempo). */
  field: keyof ChannelMeasures
}

export interface MeasureGroup {
  title: string
  rows: MeasureRow[]
}

const MISSING = '—'

function finite(value: number | null | undefined): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function volts(value: number | null): string {
  return finite(value) ? formatVolts(value) : MISSING
}

function time(value: number | null): string {
  return finite(value) ? formatTime(value) : MISSING
}

function hz(value: number | null): string {
  return finite(value) ? formatHz(value) : MISSING
}

/** Valore già in percento, con una cifra decimale: "12.3 %". */
function percent(value: number | null): string {
  if (!finite(value)) return MISSING
  const text = value.toFixed(1)
  return `${text === '-0.0' ? '0.0' : text} %`
}

/** Frazione 0..1 del periodo, mostrata in percento. */
function fraction(value: number | null): string {
  return finite(value) ? percent(value * 100) : MISSING
}

interface RowSpec {
  label: string
  field: keyof ChannelMeasures
  read: (m: ChannelMeasures) => string
}

const GROUPS: { title: string; rows: RowSpec[] }[] = [
  {
    title: 'Tensione',
    rows: [
      { label: 'Vpp', field: 'vpp', read: (m) => volts(m.vpp) },
      { label: 'Vmax', field: 'vmax', read: (m) => volts(m.vmax) },
      { label: 'Vmin', field: 'vmin', read: (m) => volts(m.vmin) },
      { label: 'Vmedia', field: 'vmean', read: (m) => volts(m.vmean) },
      { label: 'Vrms', field: 'vrms', read: (m) => volts(m.vrms) },
      { label: 'Vac rms', field: 'vacRms', read: (m) => volts(m.vacRms) },
    ],
  },
  {
    title: 'Livelli e fronti',
    rows: [
      { label: 'Vtop', field: 'vtop', read: (m) => volts(m.vtop) },
      { label: 'Vbase', field: 'vbase', read: (m) => volts(m.vbase) },
      { label: 'Ampiezza', field: 'vamp', read: (m) => volts(m.vamp) },
      { label: 'Salita', field: 'riseTime', read: (m) => time(m.riseTime) },
      { label: 'Discesa', field: 'fallTime', read: (m) => time(m.fallTime) },
      { label: 'Overshoot', field: 'overshootPct', read: (m) => percent(m.overshootPct) },
    ],
  },
  {
    title: 'Tempo',
    rows: [
      { label: 'Frequenza', field: 'frequency', read: (m) => hz(m.frequency) },
      { label: 'Periodo', field: 'period', read: (m) => time(m.period) },
      { label: 'Duty', field: 'dutyPositive', read: (m) => fraction(m.dutyPositive) },
      { label: 'Larghezza +', field: 'pulseWidthPositive', read: (m) => time(m.pulseWidthPositive) },
      { label: 'Larghezza −', field: 'pulseWidthNegative', read: (m) => time(m.pulseWidthNegative) },
      // Fronti non ha un campo unico: il grafico segue i fronti di salita.
      { label: 'Fronti', field: 'risingEdges', read: (m) => `${m.risingEdges} / ${m.fallingEdges}` },
    ],
  },
]

/** Una cella: "—" se il canale è spento o la misura manca nello snapshot. */
function cell(
  measures: ChannelMeasures | null | undefined,
  on: boolean,
  read: (m: ChannelMeasures) => string,
): string {
  if (!on || !measures) return MISSING
  return read(measures)
}

/** Gruppi e righe della griglia, in ordine, con i testi pronti per la vista. */
export function measureGroups(snapshot: MeasureSnapshot | null, enabled: [boolean, boolean]): MeasureGroup[] {
  return GROUPS.map((group) => ({
    title: group.title,
    rows: group.rows.map((spec) => ({
      label: spec.label,
      ch1: cell(snapshot?.channels[0], enabled[0], spec.read),
      ch2: cell(snapshot?.channels[1], enabled[1], spec.read),
      field: spec.field,
    })),
  }))
}

const FORMAT: Partial<Record<keyof ChannelMeasures, (value: number) => string>> = {
  vmax: formatVolts,
  vmin: formatVolts,
  vpp: formatVolts,
  vmean: formatVolts,
  vrms: formatVolts,
  vacRms: formatVolts,
  vtop: formatVolts,
  vbase: formatVolts,
  vamp: formatVolts,
  riseTime: formatTime,
  fallTime: formatTime,
  period: formatTime,
  pulseWidthPositive: formatTime,
  pulseWidthNegative: formatTime,
  frequency: formatHz,
  dutyPositive: (value) => percent(value * 100),
  overshootPct: percent,
  preshootPct: percent,
  risingEdges: String,
  fallingEdges: String,
}

/** Valore di un campo nella sua unità, come nella griglia: "4 V", "12.3 %", "11 kHz". */
export function formatValue(field: keyof ChannelMeasures, value: number): string {
  const format = FORMAT[field]
  return format ? format(value) : String(value)
}
