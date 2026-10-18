// Righe della griglia misure: testi già formattati per CH1 e CH2. Specchio di measure_panel.rs.
import type { ChannelMeasures, MeasureSnapshot } from '../types'
import { formatHz, formatTime, formatVolts } from '../shell/format'

export interface MeasureRow {
  label: string
  ch1: string
  ch2: string
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
  read: (m: ChannelMeasures) => string
}

const GROUPS: { title: string; rows: RowSpec[] }[] = [
  {
    title: 'Tensione',
    rows: [
      { label: 'Vpp', read: (m) => volts(m.vpp) },
      { label: 'Vmax', read: (m) => volts(m.vmax) },
      { label: 'Vmin', read: (m) => volts(m.vmin) },
      { label: 'Vmedia', read: (m) => volts(m.vmean) },
      { label: 'Vrms', read: (m) => volts(m.vrms) },
      { label: 'Vac rms', read: (m) => volts(m.vacRms) },
    ],
  },
  {
    title: 'Livelli e fronti',
    rows: [
      { label: 'Vtop', read: (m) => volts(m.vtop) },
      { label: 'Vbase', read: (m) => volts(m.vbase) },
      { label: 'Ampiezza', read: (m) => volts(m.vamp) },
      { label: 'Salita', read: (m) => time(m.riseTime) },
      { label: 'Discesa', read: (m) => time(m.fallTime) },
      { label: 'Overshoot', read: (m) => percent(m.overshootPct) },
    ],
  },
  {
    title: 'Tempo',
    rows: [
      { label: 'Frequenza', read: (m) => hz(m.frequency) },
      { label: 'Periodo', read: (m) => time(m.period) },
      { label: 'Duty', read: (m) => fraction(m.dutyPositive) },
      { label: 'Larghezza +', read: (m) => time(m.pulseWidthPositive) },
      { label: 'Larghezza −', read: (m) => time(m.pulseWidthNegative) },
      { label: 'Fronti', read: (m) => `${m.risingEdges} / ${m.fallingEdges}` },
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
    })),
  }))
}
