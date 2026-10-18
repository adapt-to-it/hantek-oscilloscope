<script lang="ts">
  // Corpo della sezione di un canale: V/div, offset con Azzera, sonda x1/x10, accoppiamento DC/AC.
  // L'attivazione del canale sta nell'intestazione, in ControlPanel.
  import { tr } from '../../i18n/msg/trigger'
  import type { AcquisitionSettings, ChannelSettings, ViewSettings } from '../../types'
  import { VOLT_STEPS, formatDiv, formatVolts, step } from '../format'

  interface Props {
    index: 0 | 1
    view: ViewSettings
    acq: AcquisitionSettings
    onchange: (view: ViewSettings) => void
    onacq: (acq: AcquisitionSettings) => void
  }

  let { index, view, acq, onchange, onacq }: Props = $props()

  /** Passo di spostamento verticale, in divisioni. */
  const OFFSET_STEP_DIV = 0.5
  const PROBE_OPTIONS = [1, 10] as const

  const channel = $derived(view.channels[index])
  const label = $derived(`CH${index + 1}`)

  function withChannel(patch: Partial<ChannelSettings>) {
    const channels = [...view.channels] as [ChannelSettings, ChannelSettings]
    channels[index] = { ...channels[index], ...patch }
    onchange({ ...view, channels })
  }

  function withProbe(probe: number) {
    const next = [...acq.probe] as [number, number]
    next[index] = probe
    onacq({ ...acq, probe: next })
  }

  function withCoupling(ac: boolean) {
    const next = [...acq.ac] as [boolean, boolean]
    next[index] = ac
    onacq({ ...acq, ac: next })
  }
</script>

<div style="display:flex;flex-direction:column;gap:4px">
  <div class="rw" data-help="voltsPerDiv">
    <span class="lb">V/div</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label={tr('decreaseVoltsPerDiv', { canale: label })}
        onclick={() => withChannel({ voltsPerDiv: step(VOLT_STEPS, channel.voltsPerDiv, -1) })}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{formatVolts(channel.voltsPerDiv)}</span>
      <button class="step" aria-label={tr('increaseVoltsPerDiv', { canale: label })}
        onclick={() => withChannel({ voltsPerDiv: step(VOLT_STEPS, channel.voltsPerDiv, 1) })}>+</button>
    </div>
  </div>
  <div class="rw" data-help="verticalOffset">
    <span class="lb">{tr('offset')}</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label={tr('moveDown', { canale: label })}
        onclick={() => withChannel({ offsetDiv: channel.offsetDiv - OFFSET_STEP_DIV })}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:13px">{formatDiv(channel.offsetDiv)}</span>
      <button class="step" aria-label={tr('moveUp', { canale: label })}
        onclick={() => withChannel({ offsetDiv: channel.offsetDiv + OFFSET_STEP_DIV })}>+</button>
      <button class="lnk" onclick={() => withChannel({ offsetDiv: 0 })}>{tr('zero')}</button>
    </div>
  </div>
  <div class="rw" data-help="probe">
    <span class="lb">{tr('probe')}</span>
    <div class="segrow" role="group" aria-label={tr('probeGroup', { canale: label })}>
      {#each PROBE_OPTIONS as probe (probe)}
        <button class="seg {acq.probe[index] === probe ? 'on' : ''}" aria-pressed={acq.probe[index] === probe}
          onclick={() => withProbe(probe)}>x{probe}</button>
      {/each}
    </div>
  </div>
  <div class="rw" data-help="coupling">
    <span class="lb">{tr('coupling')}</span>
    <div class="segrow" role="group" aria-label={tr('couplingGroup', { canale: label })}>
      <button class="seg {acq.ac[index] ? '' : 'on'}" aria-pressed={!acq.ac[index]}
        onclick={() => withCoupling(false)}>DC</button>
      <button class="seg {acq.ac[index] ? 'on' : ''}" aria-pressed={acq.ac[index]}
        onclick={() => withCoupling(true)}>AC</button>
    </div>
  </div>
</div>
