<script lang="ts">
  // Corpo della sezione Trigger: modo, sorgente, tipo con i suoi parametri, livello e holdoff.
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
    COND_OPTIONS,
    ENTER_OPTIONS,
    LEVEL_OPTIONS,
    LOGIC_OP_OPTIONS,
    MODE_OPTIONS,
    POLARITY_OPTIONS,
    SLOPE_OPTIONS,
    SOURCE_OPTIONS,
    TRIGGER_KINDS,
    WIDTH_OPTIONS,
    defaultSpec,
    stepDelay,
    stepValue,
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
</script>

{#snippet segRow(label: string, options: readonly { value: string; label: string }[], current: string, pick: (value: string) => void)}
  <div class="rw">
    <span class="lb">{label}</span>
    <div class="segrow" role="group" aria-label={label}>
      {#each options as option (option.value)}
        <button class="seg {option.value === current ? 'on' : ''}" style="padding:0 9px"
          aria-pressed={option.value === current} onclick={() => pick(option.value)}>{option.label}</button>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet stepRow(label: string, text: string, dec: () => void, inc: () => void)}
  <div class="rw">
    <span class="lb">{label}</span>
    <div style="display:flex;align-items:center;gap:6px">
      <button class="step" aria-label="Riduci {label}" onclick={dec}>−</button>
      <span class="mono" style="min-width:74px;text-align:center;font-size:14px;font-weight:600">{text}</span>
      <button class="step" aria-label="Aumenta {label}" onclick={inc}>+</button>
    </div>
  </div>
{/snippet}

<div style="display:flex;flex-direction:column;gap:6px">
  {@render segRow('Modo', MODE_OPTIONS, acq.mode, (value) => withAcq({ mode: value as ModeName }))}
  {@render segRow('Sorgente', SOURCE_OPTIONS, String(acq.triggerSource), (value) =>
    withAcq({ triggerSource: Number(value) as 1 | 2 }),
  )}

  <div style="display:flex;flex-direction:column;gap:6px;padding:4px 0">
    <span class="lb" style="font-size:13px;color:#c3cad4">Tipo</span>
    <div style="display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px">
      {#each TRIGGER_KINDS as option (option.kind)}
        <button class="opt {option.kind === trigger.kind ? 'on' : ''}" style="padding:0 4px"
          aria-pressed={option.kind === trigger.kind} onclick={() => pickKind(option.kind)}>{option.label}</button>
      {/each}
    </div>
  </div>

  {#if trigger.kind === 'edge'}
    {@const spec = trigger}
    {@render segRow('Pendenza', SLOPE_OPTIONS, spec.slope, (value) =>
      setTrigger({ kind: 'edge', slope: value as SlopeName }),
    )}
  {:else if trigger.kind === 'pulseWidth'}
    {@const spec = trigger}
    {@const bounded = spec.width.op === 'within' || spec.width.op === 'outside'}
    {@render segRow('Polarità', POLARITY_OPTIONS, spec.polarity, (value) =>
      setTrigger({ ...spec, polarity: value as PolarityName }),
    )}
    {@render segRow('Condizione', WIDTH_OPTIONS, spec.width.op, (value) =>
      setTrigger({ ...spec, width: { ...spec.width, op: value as WidthSpec['op'] } }),
    )}
    {@render stepRow(bounded ? 'Durata min' : 'Durata', formatTime(spec.width.a),
      () => setTrigger({ ...spec, width: { ...spec.width, a: stepValue('time', spec.width.a, -1) } }),
      () => setTrigger({ ...spec, width: { ...spec.width, a: stepValue('time', spec.width.a, 1) } }),
    )}
    {#if bounded}
      {@render stepRow('Durata max', formatTime(spec.width.b),
        () => setTrigger({ ...spec, width: { ...spec.width, b: stepValue('time', spec.width.b, -1) } }),
        () => setTrigger({ ...spec, width: { ...spec.width, b: stepValue('time', spec.width.b, 1) } }),
      )}
    {/if}
  {:else if trigger.kind === 'runt'}
    {@const spec = trigger}
    {@render segRow('Polarità', POLARITY_OPTIONS, spec.polarity, (value) =>
      setTrigger({ ...spec, polarity: value as PolarityName }),
    )}
    {@render stepRow('Soglia bassa', formatVolts(spec.low),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, -1) }),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, 1) }),
    )}
    {@render stepRow('Soglia alta', formatVolts(spec.high),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, -1) }),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, 1) }),
    )}
  {:else if trigger.kind === 'window'}
    {@const spec = trigger}
    {@render stepRow('Soglia bassa', formatVolts(spec.low),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, -1) }),
      () => setTrigger({ ...spec, low: stepValue('volt', spec.low, 1) }),
    )}
    {@render stepRow('Soglia alta', formatVolts(spec.high),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, -1) }),
      () => setTrigger({ ...spec, high: stepValue('volt', spec.high, 1) }),
    )}
    {@render segRow('Transizione', ENTER_OPTIONS, spec.enter ? 'enter' : 'exit', (value) =>
      setTrigger({ ...spec, enter: value === 'enter' }),
    )}
  {:else if trigger.kind === 'timeout'}
    {@const spec = trigger}
    {@render segRow('Pendenza', SLOPE_OPTIONS, spec.slope, (value) =>
      setTrigger({ ...spec, slope: value as SlopeName }),
    )}
    {@render stepRow('Timeout', formatTime(spec.seconds),
      () => setTrigger({ ...spec, seconds: stepValue('time', spec.seconds, -1) }),
      () => setTrigger({ ...spec, seconds: stepValue('time', spec.seconds, 1) }),
    )}
  {:else if trigger.kind === 'pattern'}
    {@const spec = trigger}
    {@render segRow('CH1', LEVEL_OPTIONS, spec.ch1, (value) =>
      setTrigger({ ...spec, ch1: value as LevelName }),
    )}
    {@render segRow('CH2', LEVEL_OPTIONS, spec.ch2, (value) =>
      setTrigger({ ...spec, ch2: value as LevelName }),
    )}
  {:else if trigger.kind === 'logic'}
    {@const spec = trigger}
    {@render segRow('Operatore', LOGIC_OP_OPTIONS, spec.op, (value) =>
      setTrigger({ ...spec, op: value as LogicOpName }),
    )}
    {@render segRow('CH1', COND_OPTIONS, spec.ch1, (value) => setTrigger({ ...spec, ch1: value as CondName }))}
    {@render segRow('CH2', COND_OPTIONS, spec.ch2, (value) => setTrigger({ ...spec, ch2: value as CondName }))}
    {@render stepRow(`Livello CH${otherCh}`, formatVolts(spec.otherLevel),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, -1) }),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, 1) }),
    )}
  {:else if trigger.kind === 'sequence'}
    {@const spec = trigger}
    {@render segRow(`A · CH${acq.triggerSource}`, SLOPE_OPTIONS, spec.slopeA, (value) =>
      setTrigger({ ...spec, slopeA: value as SlopeName }),
    )}
    {@render segRow(`B · CH${otherCh}`, SLOPE_OPTIONS, spec.slopeB, (value) =>
      setTrigger({ ...spec, slopeB: value as SlopeName }),
    )}
    {@render stepRow(`Livello CH${otherCh}`, formatVolts(spec.otherLevel),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, -1) }),
      () => setTrigger({ ...spec, otherLevel: stepValue('volt', spec.otherLevel, 1) }),
    )}
    {@render stepRow('Ritardo B', formatTime(spec.delay),
      () => setTrigger({ ...spec, delay: stepDelay(spec.delay, -1) }),
      () => setTrigger({ ...spec, delay: stepDelay(spec.delay, 1) }),
    )}
  {/if}

  {#if dual}
    <span style="font-size:12px;color:#9aa3af">Richiede CH1 e CH2 attivi. La sorgente sceglie il canale che usa il livello del marcatore T.</span>
  {/if}

  {@render stepRow(dual ? `Livello CH${acq.triggerSource}` : 'Livello', formatVolts(acq.triggerLevel),
    () => withAcq({ triggerLevel: stepValue('volt', acq.triggerLevel, -1) }),
    () => withAcq({ triggerLevel: stepValue('volt', acq.triggerLevel, 1) }),
  )}
  {@render stepRow('Holdoff', formatTime(acq.holdoff),
    () => withAcq({ holdoff: stepValue('time', acq.holdoff, -1) }),
    () => withAcq({ holdoff: stepValue('time', acq.holdoff, 1) }),
  )}
</div>
