<script lang="ts">
  // Griglia delle misure: una card per gruppo, poi Fase e Ritardo CH1–CH2. Nessuna chiamata di rete.
  import type { MeasureSnapshot } from '../types'
  import { CH_COLORS } from '../types'
  import { formatTime } from '../shell/format'
  import { measureGroups } from './rows'

  interface Props {
    snapshot: MeasureSnapshot | null
    enabled: [boolean, boolean]
  }

  let { snapshot, enabled }: Props = $props()

  const groups = $derived(measureGroups(snapshot, enabled))

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

<section style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(260px,100%),1fr));gap:12px">
  {#each groups as group (group.title)}
    <div class="card" style="padding:8px 10px 10px;display:flex;flex-direction:column;gap:2px">
      <div class="mrow" style="min-height:30px;border-bottom:1px solid #20262e;border-radius:0">
        <span style="font-size:12px;font-weight:700;color:#e8ebef;letter-spacing:.02em;white-space:nowrap">{group.title}</span>
        <span style="font-size:12px;font-weight:700;color:{CH_COLORS[0]};text-align:right">CH1</span>
        <span style="font-size:12px;font-weight:700;color:{CH_COLORS[1]};text-align:right">CH2</span>
        <span></span>
      </div>
      {#each group.rows as row (row.label)}
        <div class="mrow">
          <span style="white-space:nowrap">{row.label}</span>
          <span class="mono" style="text-align:right;color:{cellColor(row.ch1, 0)}">{row.ch1}</span>
          <span class="mono" style="text-align:right;color:{cellColor(row.ch2, 1)}">{row.ch2}</span>
          <span></span>
        </div>
      {/each}
    </div>
  {/each}
</section>
<div style="display:flex;flex-wrap:wrap;gap:6px 24px;font-size:13px;color:#c3cad4;padding:0 4px">
  <span>Fase CH1–CH2 <span class="mono" style="color:#8a94a1;margin-left:6px">{phaseText(snapshot?.phaseDeg)}</span></span>
  <span>Ritardo CH1–CH2 <span class="mono" style="color:#8a94a1;margin-left:6px">{delayText(snapshot?.delayS)}</span></span>
  <span style="color:#8a94a1">— = non calcolabile nella finestra attuale</span>
</div>
