<script lang="ts">
  // Pannello di destra: canali (attivo, V/div, offset) e asse orizzontale (tempo/div, offset).
  // Ogni modifica produce una nuova vista; l'applicazione al motore la fa il genitore.
  import type { ChannelSettings, ViewSettings } from '../types'
  import { CH_COLORS } from '../types'
  import { TIME_STEPS, VOLT_STEPS, formatDiv, formatTime, formatVolts, step } from './format'

  interface Props {
    view: ViewSettings
    onchange: (view: ViewSettings) => void
  }

  let { view, onchange }: Props = $props()

  /** Passo di spostamento verticale, in divisioni. */
  const OFFSET_STEP_DIV = 0.5

  function withChannel(index: 0 | 1, patch: Partial<ChannelSettings>) {
    const channels = [...view.channels] as [ChannelSettings, ChannelSettings]
    channels[index] = { ...channels[index], ...patch }
    onchange({ ...view, channels })
  }

  function withTime(timePerDiv: number) {
    onchange({ ...view, timePerDiv })
  }

  function withHorizontal(horizontalOffset: number) {
    onchange({ ...view, horizontalOffset })
  }
</script>

<div style="display:flex;flex-direction:column">
  <section style="border-bottom:1px solid #20262e;padding:12px 14px;display:flex;flex-direction:column;gap:4px">
    <h3 style="font-size:14px;font-weight:700;margin-bottom:6px">Canali</h3>
    {#each view.channels as channel, i (i)}
      {@const index = i as 0 | 1}
      <div style="display:flex;flex-direction:column;gap:4px;padding-bottom:10px;opacity:{channel.enabled ? 1 : 0.55}">
        <div class="rw">
          <span style="font-size:14px;font-weight:700;color:{CH_COLORS[index]}">CH{i + 1}</span>
          <button
            class="tgl {index === 0 ? 'c1' : 'c2'} {channel.enabled ? 'on' : ''}"
            role="switch"
            aria-checked={channel.enabled}
            aria-label="CH{i + 1} attivo"
            onclick={() => withChannel(index, { enabled: !channel.enabled })}
          ></button>
        </div>
        <div class="rw">
          <span class="lb">V/div</span>
          <div style="display:flex;align-items:center;gap:6px">
            <button class="step" aria-label="Riduci volt per divisione CH{i + 1}"
              onclick={() => withChannel(index, { voltsPerDiv: step(VOLT_STEPS, channel.voltsPerDiv, -1) })}>−</button>
            <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{formatVolts(channel.voltsPerDiv)}</span>
            <button class="step" aria-label="Aumenta volt per divisione CH{i + 1}"
              onclick={() => withChannel(index, { voltsPerDiv: step(VOLT_STEPS, channel.voltsPerDiv, 1) })}>+</button>
          </div>
        </div>
        <div class="rw">
          <span class="lb">Offset</span>
          <div style="display:flex;align-items:center;gap:6px">
            <button class="step" aria-label="Sposta in basso CH{i + 1}"
              onclick={() => withChannel(index, { offsetDiv: channel.offsetDiv - OFFSET_STEP_DIV })}>−</button>
            <span class="mono" style="min-width:74px;text-align:center;font-size:13px">{formatDiv(channel.offsetDiv)}</span>
            <button class="step" aria-label="Sposta in alto CH{i + 1}"
              onclick={() => withChannel(index, { offsetDiv: channel.offsetDiv + OFFSET_STEP_DIV })}>+</button>
          </div>
        </div>
      </div>
    {/each}
  </section>

  <section style="padding:12px 14px;display:flex;flex-direction:column;gap:4px">
    <h3 style="font-size:14px;font-weight:700;margin-bottom:6px">Orizzontale</h3>
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
      </div>
    </div>
  </section>
</div>
