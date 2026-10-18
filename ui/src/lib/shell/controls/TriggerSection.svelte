<script lang="ts">
  // Corpo della sezione Trigger: modo, sorgente, tipo con i suoi parametri, livello e holdoff.
  import type { HelpId } from '../../help/texts'
  import { tr } from '../../i18n/msg/trigger'
  import type {
    AcquisitionSettings,
    CondName,
    LevelName,
    LogicOpName,
    ModeName,
    PolarityName,
    SlopeName,
    TriggerSpec,
    WidthSpec,
  } from '../../types'
  import { formatTime, formatVolts } from '../format'
  import {
    LOGIC_OP_OPTIONS,
    SOURCE_OPTIONS,
    condOptions,
    defaultSpec,
    enterOptions,
    levelOptions,
    modeOptions,
    polarityOptions,
    slopeOptions,
    stepDelay,
    stepValue,
    triggerKinds,
    widthOptions,
  } from './trigger'
  import type { TriggerKind } from './trigger'

  interface Props {
    acq: AcquisitionSettings
    onacq: (acq: AcquisitionSettings) => void
  }

  let { acq, onacq }: Props = $props()

  const trigger = $derived(acq.trigger)
  /** Logico e A → B usano entrambi i canali: la sorgente ha `triggerLevel`, l'altro canale un livello proprio. */
  const dual = $derived(trigger.kind === 'logic' || trigger.kind === 'sequence')
  const otherCh = $derived(acq.triggerSource === 1 ? 2 : 1)

  function withAcq(patch: Partial<AcquisitionSettings>) {
    onacq({ ...acq, ...patch })
  }

  function setTrigger(next: TriggerSpec) {
    onacq({ ...acq, trigger: next })
  }

  /** Cambiare tipo riparte dai valori predefiniti attorno al livello attuale. */
  function pickKind(kind: TriggerKind) {
    if (kind !== trigger.kind) setTrigger(defaultSpec(kind, acq.triggerLevel))
  }

  const KIND_HELP: Record<TriggerSpec['kind'], HelpId> = {
    edge: 'triggerSlope',
    pulseWidth: 'pulseWidth',
    runt: 'runt',
    window: 'windowTrigger',
    timeout: 'timeoutTrigger',
    pattern: 'patternTrigger',
    logic: 'logicTrigger',
    sequence: 'sequenceTrigger',
  }
</script>

{#snippet segRow(label: string, options: readonly { value: string; label: string }[], current: string, pick: (value: string) => void, help?: HelpId)}
  <div class="rw" data-help={help}>
    <span class="lb">{label}</span>
    <div class="segrow" role="group" aria-label={label}>
      {#each options as option (option.value)}
        <button class="seg {option.value === current ? 'on' : ''}" style="padding:0 9px"
          aria-pressed={option.value === current} onclick={() => pick(option.value)}>{option.label}</button>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet stepRow(label: string, text: string, dec: () => void, inc: () => void, help?: HelpId)}
  <div class="rw" data-help={help}>
    <span class="lb">{label}</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label={tr('decreaseLabel', { etichetta: label })} onclick={dec}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{text}</span>
      <button class="step" aria-label={tr('increaseLabel', { etichetta: label })} onclick={inc}>+</button>
    </div>
  </div>
{/snippet}

<div style="display:flex;flex-direction:column;gap:6px">
  {@render segRow(tr('labelMode'), modeOptions(), acq.mode, (value) => withAcq({ mode: value as ModeName }), 'triggerMode')}
  {@render segRow(tr('labelSource'), SOURCE_OPTIONS, String(acq.triggerSource), (value) =>
    withAcq({ triggerSource: Number(value) as 1 | 2 }), 'triggerSource',
  )}

  <div data-help="triggerKind" style="display:flex;flex-direction:column;gap:6px;padding:4px 0">
    <span class="lb" style="font-size:13px;color:#c3cad4">{tr('labelType')}</span>
    <div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px">
      {#each triggerKinds() as option (option.kind)}
        <button class="opt {option.kind === trigger.kind ? 'on' : ''}" style="padding:0 4px"
          aria-pressed={option.kind === trigger.kind} onclick={() => pickKind(option.kind)}>{option.label}</button>
      {/each}
    </div>
  </div>

  <div data-help={KIND_HELP[trigger.kind]} style="display:flex;flex-direction:column;gap:6px">
  {#if trigger.kind === 'edge'}
    {@const spec = trigger}
    {@render segRow(tr('labelSlope'), slopeOptions(), spec.slope, (value) =>
      setTrigger({ kind: 'edge', slope: value as SlopeName }),
    )}
  {:else if trigger.kind === 'pulseWidth'}
    {@const spec = trigger}
    {@const bounded = spec.width.op === 'within' || spec.width.op === 'outside'}
    {@render segRow(tr('labelPolarity'), polarityOptions(), spec.polarity, (value) =>
      setTrigger({ ...spec, polarity: value as PolarityName }),
    )}
    {@render segRow(tr('labelCondition'), widthOptions(), spec.width.op, (value) =>
      setTrigger({ ...spec, width: { ...spec.width, op: value as WidthSpec['op'] } }),
    )}
    {@render stepRow(bounded ? tr('labelMinDuration') : tr('labelDuration'), formatTime(spec.width.a),
      () => setTrigger({ ...spec, width: { ...spec.width, a: stepValue('time', spec.width.a, -1) } }),
      () => setTrigger({ ...spec, width: { ...spec.width, a: stepValue('time', spec.width.a, 1) } }),
    )}
    {#if bounded}
      {@render stepRow(tr('labelMaxDuration'), formatTime(spec.width.b),
        () => setTrigger({ ...spec, width: { ...spec.width, b: stepValue('time', spec.width.b, -1) } }),
        () => setTrigger({ ...spec, width: { ...spec.width, b: stepValue('time', spec.width.b, 1) } }),
      )}
    {/if}
  {:else if trigger.kind === 'runt'}
    {@const spec = trigger}
    {@render segRow(tr('labelPolarity'), polarityOptions(), spec.polarity, (value) =>
      setTrigger({ ...spec, polarity: value as PolarityName }),
    )}
    {@render stepRow(tr('labelLowThreshold'), formatVolts(spec.low),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, -1) }),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, 1) }),
    )}
    {@render stepRow(tr('labelHighThreshold'), formatVolts(spec.high),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, -1) }),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, 1) }),
    )}
  {:else if trigger.kind === 'window'}
    {@const spec = trigger}
    {@render stepRow(tr('labelLowThreshold'), formatVolts(spec.low),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, -1) }),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, 1) }),
    )}
    {@render stepRow(tr('labelHighThreshold'), formatVolts(spec.high),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, -1) }),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, 1) }),
    )}
    {@render segRow(tr('labelTransition'), enterOptions(), spec.enter ? 'enter' : 'exit', (value) =>
      setTrigger({ ...spec, enter: value === 'enter' }),
    )}
  {:else if trigger.kind === 'timeout'}
    {@const spec = trigger}
    {@render segRow(tr('labelSlope'), slopeOptions(), spec.slope, (value) =>
      setTrigger({ ...spec, slope: value as SlopeName }),
    )}
    {@render stepRow(tr('kindTimeout'), formatTime(spec.seconds),
      () => setTrigger({ ...spec, seconds: stepValue('time', spec.seconds, -1) }),
      () => setTrigger({ ...spec, seconds: stepValue('time', spec.seconds, 1) }),
    )}
  {:else if trigger.kind === 'pattern'}
    {@const spec = trigger}
    {@render segRow('CH1', levelOptions(), spec.ch1, (value) =>
      setTrigger({ ...spec, ch1: value as LevelName }),
    )}
    {@render segRow('CH2', levelOptions(), spec.ch2, (value) =>
      setTrigger({ ...spec, ch2: value as LevelName }),
    )}
  {:else if trigger.kind === 'logic'}
    {@const spec = trigger}
    {@render segRow(tr('labelOperator'), LOGIC_OP_OPTIONS, spec.op, (value) =>
      setTrigger({ ...spec, op: value as LogicOpName }),
    )}
    {@render segRow('CH1', condOptions(), spec.ch1, (value) => setTrigger({ ...spec, ch1: value as CondName }))}
    {@render segRow('CH2', condOptions(), spec.ch2, (value) => setTrigger({ ...spec, ch2: value as CondName }))}
    {@render stepRow(tr('labelLevelCh', { canale: otherCh }), formatVolts(spec.otherLevel),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, -1) }),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, 1) }), 'otherLevel',
    )}
  {:else if trigger.kind === 'sequence'}
    {@const spec = trigger}
    {@render segRow(`A · CH${acq.triggerSource}`, slopeOptions(), spec.slopeA, (value) =>
      setTrigger({ ...spec, slopeA: value as SlopeName }),
    )}
    {@render segRow(`B · CH${otherCh}`, slopeOptions(), spec.slopeB, (value) =>
      setTrigger({ ...spec, slopeB: value as SlopeName }),
    )}
    {@render stepRow(tr('labelLevelCh', { canale: otherCh }), formatVolts(spec.otherLevel),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, -1) }),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, 1) }), 'otherLevel',
    )}
    {@render stepRow(tr('labelDelayB'), formatTime(spec.delay),
      () => setTrigger({ ...spec, delay: stepDelay(spec.delay, -1) }),
      () => setTrigger({ ...spec, delay: stepDelay(spec.delay, 1) }), 'sequenceDelay',
    )}
  {/if}
  </div>

  {#if dual}
    <span style="font-size:12px;color:#9aa3af">{tr('dualNote')}</span>
  {/if}

  {@render stepRow(dual ? tr('labelLevelCh', { canale: acq.triggerSource }) : tr('labelLevel'), formatVolts(acq.triggerLevel),
    () => withAcq({ triggerLevel: stepValue('volt', acq.triggerLevel, -1) }),
    () => withAcq({ triggerLevel: stepValue('volt', acq.triggerLevel, 1) }), 'triggerLevel',
  )}
  {@render stepRow('Holdoff', formatTime(acq.holdoff),
    () => withAcq({ holdoff: stepValue('time', acq.holdoff, -1) }),
    () => withAcq({ holdoff: stepValue('time', acq.holdoff, 1) }), 'holdoff',
  )}
</div>
