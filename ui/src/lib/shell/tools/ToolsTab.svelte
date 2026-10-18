<script lang="ts">
  // Scheda Strumenti: Registrazione, Classificazione, Matematica e Filtro digitale. Ogni modifica produce un oggetto nuovo.
  import type { ClassificationView, DecoderSettings, FilterType, MathOp, RecordingView, ToolSettings, ViewMode, ViewSettings } from '../../types'
  import { CH_COLORS } from '../../types'
  import { classify } from '../../api'
  import { formatHz } from '../format'
  import Tip from '../../help/Tip.svelte'
  import FileSection from '../controls/FileSection.svelte'
  import { tr } from '../../i18n/msg/measures'
  import { CHIPS, chipLabel, classHighlight, mathNote, stepCutoff } from './tools'

  interface Props {
    view: ViewSettings
    decoder: DecoderSettings
    tools: ToolSettings
    ontools: (t: ToolSettings) => void
    onrecording: (r: RecordingView | null) => void
    /** Vista attiva: la traccia M esiste solo nella vista Tempo. */
    mode?: ViewMode
  }

  let { view, decoder, tools, ontools, onrecording, mode = 'time' }: Props = $props()

  // Le etichette sono funzioni: leggono la lingua corrente al render.
  const MATH_OPTIONS: readonly { value: MathOp; label: () => string }[] = [
    { value: 'off', label: () => tr('mathOff') },
    { value: 'add', label: () => 'CH1+CH2' },
    { value: 'sub', label: () => 'CH1−CH2' },
    { value: 'mul', label: () => 'CH1×CH2' },
    { value: 'derivative', label: () => 'd/dt CH1' },
    { value: 'integral', label: () => '∫ CH1' },
  ]
  const FILTER_CHANNELS: readonly { value: 0 | 1; label: string }[] = [
    { value: 0, label: 'CH1' },
    { value: 1, label: 'CH2' },
  ]
  const FILTER_TYPES: readonly { value: FilterType; label: () => string }[] = [
    { value: 'none', label: () => tr('filterNone') },
    { value: 'lowPass', label: () => tr('filterLow') },
    { value: 'highPass', label: () => tr('filterHigh') },
    { value: 'bandPass', label: () => tr('filterBand') },
  ]

  // Risultati della classificazione per indice di canale (0 = CH1).
  let classes = $state<Record<number, ClassificationView | null>>({})
  let classifying = $state(false)
  let classError = $state('')

  const activeChannels = $derived(([0, 1] as const).filter((index) => view.channels[index].enabled))

  async function classifyActive() {
    classError = ''
    classifying = true
    try {
      const next: Record<number, ClassificationView | null> = {}
      for (const index of activeChannels) {
        next[index] = await classify((index + 1) as 1 | 2)
      }
      classes = next
    } catch (e) {
      classError = String(e)
    } finally {
      classifying = false
    }
  }

  function pickMath(op: MathOp) {
    ontools({ ...tools, math: op })
  }

  function pickFilterChannel(channel: 0 | 1) {
    ontools({ ...tools, filter: { ...tools.filter, channel } })
  }

  function pickFilterType(kind: FilterType) {
    ontools({ ...tools, filter: { ...tools.filter, kind } })
  }

  function moveCutoff(dir: 1 | -1) {
    ontools({ ...tools, filter: { ...tools.filter, cutoffHz: stepCutoff(tools.filter.cutoffHz, dir) } })
  }

  function chipStyle(on: boolean): string {
    return on
      ? 'background:#2c3443;color:#fff;border:1px solid #4a5568'
      : 'background:transparent;color:#8a94a1;border:1px solid #232932'
  }
</script>

<div style="display:flex;flex-direction:column">
  <div style="padding:14px;border-bottom:1px solid #20262e">
    <Tip id="recording">
      <FileSection {decoder} {onrecording} />
    </Tip>
  </div>

  <div style="padding:14px;display:flex;flex-direction:column;gap:10px;border-bottom:1px solid #20262e">
    <Tip id="classify">
      <div style="display:flex;flex-direction:column;gap:10px">
        <div style="display:flex;align-items:center;justify-content:space-between">
          <span style="font-size:13px;font-weight:700">{tr('classTitle')}</span>
          <button class="btn" style="height:32px" disabled={classifying || activeChannels.length === 0}
            onclick={classifyActive}>{tr('classify')}</button>
        </div>
        {#if activeChannels.length === 0}
          <span style="font-size:12px;color:#8a94a1;line-height:1.45">{tr('classNeedChannel')}</span>
        {/if}
        {#if classError}
          <div style="font-size:13px;color:#ff6b6b">{classError}</div>
        {/if}
        {#each activeChannels as index (index)}
          {@const result = classes[index]}
          <div style="display:flex;flex-direction:column;gap:6px;padding:10px 12px;border-radius:10px;background:#0d1013;border:1px solid #232932">
            <span style="font-weight:700;color:{CH_COLORS[index]}">CH{index + 1}</span>
            {#if result}
              {@const highlight = classHighlight(result.class)}
              <div style="display:flex;flex-wrap:wrap;gap:5px">
                {#each CHIPS as chip (chip)}
                  <span style="padding:3px 8px;border-radius:999px;font-size:12px;font-weight:600;{chipStyle(chip === highlight)}">{chipLabel(chip)}</span>
                {/each}
              </div>
              <span class="mono" style="font-size:12px;color:#c3cad4">{result.summary}</span>
            {:else if result === null}
              <span style="font-size:12px;color:#8a94a1">{tr('classNoFrames')}</span>
            {:else}
              <span style="font-size:12px;color:#8a94a1">{tr('classPressHint')}</span>
            {/if}
          </div>
        {/each}
      </div>
    </Tip>
  </div>

  <div style="padding:14px;display:flex;flex-direction:column;gap:10px;border-bottom:1px solid #20262e">
    <Tip id="math">
      <div style="display:flex;flex-direction:column;gap:10px">
        <span style="font-size:13px;font-weight:700;color:#b794f6">{tr('mathTitle')}</span>
        <div style="display:flex;flex-wrap:wrap;gap:6px" role="group" aria-label={tr('mathGroupAria')}>
          {#each MATH_OPTIONS as option (option.value)}
            <button class="opt {option.value === tools.math ? 'on' : ''}" aria-pressed={option.value === tools.math}
              onclick={() => pickMath(option.value)}>{option.label()}</button>
          {/each}
        </div>
        <span style="font-size:12px;color:#8a94a1;line-height:1.45">{mathNote(tools.math, mode)}</span>
      </div>
    </Tip>
  </div>

  <div style="padding:14px 14px 18px;display:flex;flex-direction:column;gap:10px">
    <Tip id="filter">
      <div style="display:flex;flex-direction:column;gap:10px">
        <span style="font-size:13px;font-weight:700">{tr('filterTitle')}</span>
        <div class="rw">
          <span class="lb">{tr('filterApplyTo')}</span>
          <div class="segrow" role="group" aria-label={tr('filterChannelAria')}>
            {#each FILTER_CHANNELS as option (option.value)}
              <button class="seg {option.value === tools.filter.channel ? 'on' : ''}" aria-pressed={option.value === tools.filter.channel}
                onclick={() => pickFilterChannel(option.value)}>{option.label}</button>
            {/each}
          </div>
        </div>
        <div style="display:flex;flex-wrap:wrap;gap:6px" role="group" aria-label={tr('filterTypeAria')}>
          {#each FILTER_TYPES as option (option.value)}
            <button class="opt {option.value === tools.filter.kind ? 'on' : ''}" aria-pressed={option.value === tools.filter.kind}
              onclick={() => pickFilterType(option.value)}>{option.label()}</button>
          {/each}
        </div>
        <div class="rw">
          <span class="lb">{tr('cutoffLabel')}</span>
          <div style="display:flex;align-items:center;gap:6px">
            <button class="step" onclick={() => moveCutoff(-1)} aria-label={tr('cutoffDown')}>−</button>
            <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{formatHz(tools.filter.cutoffHz)}</span>
            <button class="step" onclick={() => moveCutoff(1)} aria-label={tr('cutoffUp')}>+</button>
          </div>
        </div>
      </div>
    </Tip>
  </div>
</div>
