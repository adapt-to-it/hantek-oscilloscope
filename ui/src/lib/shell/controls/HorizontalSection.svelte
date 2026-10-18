<script lang="ts">
  // Corpo della sezione Orizzontale: tempo/div, offset con Centra, pretrigger e riga di frequenza.
  import type { AcquisitionSettings, ViewSettings } from '../../types'
  import { TIME_STEPS, formatTime, step } from '../format'

  interface Props {
    view: ViewSettings
    acq: AcquisitionSettings
    onchange: (view: ViewSettings) => void
    onacq: (acq: AcquisitionSettings) => void
    /** Riga informativa, per esempio "Frequenza 48 MHz, record 4096 campioni". */
    info?: string
  }

  let { view, acq, onchange, onacq, info }: Props = $props()

  /** Passo del pretrigger, in percentuale. */
  const PRETRIGGER_STEP_PCT = 5

  const pretriggerPct = $derived(Math.round(acq.pretrigger * 100))

  function withTime(timePerDiv: number) {
    onchange({ ...view, timePerDiv })
  }

  function withHorizontal(horizontalOffset: number) {
    onchange({ ...view, horizontalOffset })
  }

  function withPretrigger(pct: number) {
    onacq({ ...acq, pretrigger: pct / 100 })
  }
</script>

<div style="display:flex;flex-direction:column;gap:4px">
  <div class="rw">
    <span class="lb">Tempo/div</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label="Riduci tempo per divisione"
        onclick={() => withTime(step(TIME_STEPS, view.timePerDiv, -1))}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{formatTime(view.timePerDiv)}</span>
      <button class="step" aria-label="Aumenta tempo per divisione"
        onclick={() => withTime(step(TIME_STEPS, view.timePerDiv, 1))}>+</button>
    </div>
  </div>
  <div class="rw">
    <span class="lb">Offset</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label="Sposta a sinistra"
        onclick={() => withHorizontal(view.horizontalOffset - view.timePerDiv)}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:13px">{formatTime(view.horizontalOffset)}</span>
      <button class="step" aria-label="Sposta a destra"
        onclick={() => withHorizontal(view.horizontalOffset + view.timePerDiv)}>+</button>
      <button class="lnk" onclick={() => withHorizontal(0)}>Centra</button>
    </div>
  </div>
  <div class="rw">
    <span class="lb">Pretrigger</span>
    <div style="display:flex;align-items:center;gap:10px;flex:1;justify-content:flex-end">
      <input type="range" min="0" max="100" step={PRETRIGGER_STEP_PCT} value={pretriggerPct}
        oninput={(event) => withPretrigger(Number(event.currentTarget.value))}
        aria-label="Pretrigger" style="width:120px;accent-color:#ff9f43">
      <span class="mono" style="min-width:42px;text-align:right;font-size:13px">{pretriggerPct} %</span>
    </div>
  </div>
  {#if info}
    <div class="mono" style="font-size:12px;color:#8a94a1;padding-top:4px">{info}</div>
  {/if}
</div>
