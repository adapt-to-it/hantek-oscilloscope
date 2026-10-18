<script lang="ts">
  // Griglia delle misure: una card per gruppo, poi Fase e Ritardo CH1–CH2. Nessuna chiamata di rete.
  import type { ChannelMeasures, MeasureSnapshot } from '../types'
  import { CH_COLORS } from '../types'
  import { formatTime } from '../shell/format'
  import type { MeasureHistory } from './history'
  import { measureGroups, type MeasureRow } from './rows'
  import TrendModal from './TrendModal.svelte'
  import type { HelpId } from '../help/texts'
  import { tr } from '../i18n/msg/measures'

  // Spiegazione di ogni riga, per campo misurato.
  const ROW_HELP: Partial<Record<keyof ChannelMeasures, HelpId>> = {
    vpp: 'vpp', vmax: 'vmax', vmin: 'vmin', vmean: 'vmean', vrms: 'vrms', vacRms: 'vacRms',
    vtop: 'vtop', vbase: 'vbase', vamp: 'vamp', riseTime: 'riseTime', fallTime: 'fallTime',
    overshootPct: 'overshoot', frequency: 'frequency', period: 'period', dutyPositive: 'duty',
    pulseWidthPositive: 'pulseWidthMeasure', pulseWidthNegative: 'pulseWidthMeasure', risingEdges: 'edges',
  }

  interface Props {
    snapshot: MeasureSnapshot | null
    enabled: [boolean, boolean]
    history: MeasureHistory
    /** Cresce a ogni push nello storico. */
    version: number
    ontrend?: (field: keyof ChannelMeasures, label: string) => void
  }

  let { snapshot, enabled, history, version, ontrend }: Props = $props()

  const groups = $derived(measureGroups(snapshot, enabled))

  let trend = $state<{ field: keyof ChannelMeasures; label: string } | null>(null)

  /** Il pulsante grafico compare solo se almeno un canale attivo ha un valore numerico. */
  function hasNumber(row: MeasureRow): boolean {
    return enabled.some((on, ch) => on && Number.isFinite(snapshot?.channels[ch]?.[row.field]))
  }

  function openTrend(row: MeasureRow) {
    trend = { field: row.field, label: row.label }
    ontrend?.(row.field, row.label)
  }

  function phaseText(deg: number | null | undefined): string {
    return deg != null && Number.isFinite(deg) ? `${deg.toFixed(1)} °` : '—'
  }

  function delayText(seconds: number | null | undefined): string {
    return seconds != null && Number.isFinite(seconds) ? formatTime(seconds) : '—'
  }

  function cellColor(text: string, channel: 0 | 1): string {
    return text === '—' ? '#8a94a1' : CH_COLORS[channel]
  }
</script>

<section data-help="measurements" style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(260px,100%),1fr));gap:12px">
  {#each groups as group (group.title)}
    <div class="card" style="padding:8px 10px 10px;display:flex;flex-direction:column;gap:2px">
      <div class="mrow" style="min-height:30px;border-bottom:1px solid #20262e;border-radius:0">
        <span style="font-size:12px;font-weight:700;color:#e8ebef;letter-spacing:.02em;white-space:nowrap">{group.title}</span>
        <span style="font-size:12px;font-weight:700;color:{CH_COLORS[0]};text-align:right">CH1</span>
        <span style="font-size:12px;font-weight:700;color:{CH_COLORS[1]};text-align:right">CH2</span>
        <span></span>
      </div>
      {#each group.rows as row (row.label)}
        <div class="mrow" data-help={ROW_HELP[row.field]}>
          <span style="white-space:nowrap">{row.label}</span>
          <span class="mono" style="text-align:right;color:{cellColor(row.ch1, 0)}">{row.ch1}</span>
          <span class="mono" style="text-align:right;color:{cellColor(row.ch2, 1)}">{row.ch2}</span>
          {#if hasNumber(row)}
            <button class="ibtn" onclick={() => openTrend(row)} aria-label={tr('trendAria', { label: row.label })} title={tr('trendTitle')}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 4v16h18"></path><path d="M7 15l4-5 3 3 5-7"></path></svg>
            </button>
          {:else}
            <span></span>
          {/if}
        </div>
      {/each}
    </div>
  {/each}
</section>
<div style="display:flex;flex-wrap:wrap;gap:6px 24px;font-size:13px;color:#c3cad4;padding:0 4px">
  <span data-help="phase">{tr('phaseLabel')} <span class="mono" style="color:#8a94a1;margin-left:6px">{phaseText(snapshot?.phaseDeg)}</span></span>
  <span data-help="delay">{tr('delayLabel')} <span class="mono" style="color:#8a94a1;margin-left:6px">{delayText(snapshot?.delayS)}</span></span>
  <span style="color:#8a94a1">{tr('notComputable')}</span>
</div>

{#if trend}
  <TrendModal
    {history}
    {version}
    field={trend.field}
    label={trend.label}
    onclose={() => (trend = null)}
  />
{/if}
