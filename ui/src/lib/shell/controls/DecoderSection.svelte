<script lang="ts">
  // Corpo della sezione Decoder: attivazione, protocollo, canale, soglia e parametri del protocollo scelto.
  import type { DecodeParams, DecoderSettings, ParityName, ProtocolName } from '../../types'
  import { formatVolts } from '../format'
  import { BAUDS, PROTOCOLS, channelLabel, stepBaud, withProtocol } from './decoder'

  interface Props {
    decoder: DecoderSettings
    ondecoder: (d: DecoderSettings) => void
  }

  let { decoder, ondecoder }: Props = $props()

  /** Valori mostrati quando un automatismo viene spento. */
  const DEFAULT_THRESHOLD_V = 1.5
  const DEFAULT_BAUD = 9600
  const THRESHOLD_STEP_V = 0.1
  const DATA_BITS_MIN = 5
  const DATA_BITS_MAX = 9

  const SELECT_STYLE =
    'height:34px;border-radius:8px;border:1px solid #2a313b;background:#181c22;color:#e8ebef;padding:0 8px;font-size:13px'

  const PARITY_OPTIONS: readonly { value: ParityName; label: string }[] = [
    { value: 'none', label: 'Nessuna' },
    { value: 'even', label: 'Pari' },
    { value: 'odd', label: 'Dispari' },
  ]
  const STOP_OPTIONS: readonly { value: string; label: string }[] = [
    { value: '1', label: '1' },
    { value: '1.5', label: '1,5' },
    { value: '2', label: '2' },
  ]
  const CAN_BITRATE_OPTIONS: readonly { value: string; label: string }[] = [
    { value: '125000', label: '125k' },
    { value: '250000', label: '250k' },
    { value: '500000', label: '500k' },
    { value: '1000000', label: '1M' },
  ]
  const LEGEND: readonly { label: string; color: string }[] = [
    { label: 'Dati', color: '#6fc4f0' },
    { label: 'Indirizzo', color: '#ff9f43' },
    { label: 'ID', color: '#b794f6' },
    { label: 'ACK', color: '#5fd68d' },
    { label: 'NACK', color: '#ff9a8a' },
    { label: 'Start e stop', color: '#8a94a1' },
    { label: 'Comando', color: '#4fd6c4' },
    { label: 'CRC', color: '#f5d742' },
    { label: 'Reset', color: '#b08968' },
    { label: 'Errore', color: '#e5392b' },
  ]

  const params = $derived(decoder.params)
  const protocol = $derived(params.protocol)
  const thresholdAuto = $derived(params.threshold_v === null)
  const threshold = $derived(params.threshold_v ?? DEFAULT_THRESHOLD_V)
  const baudAuto = $derived(params.baud === null)
  const baud = $derived(params.baud ?? DEFAULT_BAUD)
  const dataBits = $derived(params.data_bits ?? 8)
  const parity = $derived(params.parity ?? 'none')
  const stopBits = $derived(params.stop_bits ?? 1)
  const inverted = $derived(params.inverted ?? false)
  const bitrate = $derived(params.bitrate ?? 500000)
  const ieee = $derived(params.ieee ?? true)

  function withParams(patch: Partial<DecodeParams>) {
    ondecoder({ ...decoder, params: { ...params, ...patch } })
  }

  function setEnabled(enabled: boolean) {
    ondecoder({ ...decoder, enabled })
  }

  function pickProtocol(value: ProtocolName) {
    ondecoder(withProtocol(decoder, value))
  }

  function toggleThreshold() {
    withParams({ threshold_v: thresholdAuto ? DEFAULT_THRESHOLD_V : null })
  }

  function stepThreshold(dir: 1 | -1) {
    withParams({ threshold_v: Math.round((threshold + dir * THRESHOLD_STEP_V) * 10) / 10 })
  }

  function toggleBaudAuto() {
    withParams({ baud: baudAuto ? DEFAULT_BAUD : null })
  }

  function stepDataBits(dir: 1 | -1) {
    withParams({ data_bits: Math.min(DATA_BITS_MAX, Math.max(DATA_BITS_MIN, dataBits + dir)) })
  }
</script>

{#snippet toggleRow(label: string, on: boolean, toggle: () => void)}
  <div class="rw">
    <span class="lb">{label}</span>
    <button class="tgl {on ? 'on' : ''}" role="switch" aria-checked={on} aria-label={label} onclick={toggle}></button>
  </div>
{/snippet}

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

<div style="display:flex;flex-direction:column;gap:8px">
  {@render toggleRow('Attiva decoder', decoder.enabled, () => setEnabled(!decoder.enabled))}

  <div class="rw" data-help="decoderProtocol">
    <span class="lb">Protocollo</span>
    <select style={SELECT_STYLE} aria-label="Protocollo" value={protocol}
      onchange={(e) => pickProtocol(e.currentTarget.value as ProtocolName)}>
      {#each PROTOCOLS as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </div>

  <div class="rw" data-help="decoderChannel">
    <span class="lb">Canale</span>
    <select style={SELECT_STYLE} aria-label="Canale da decodificare" value={String(params.channel)}
      onchange={(e) => withParams({ channel: Number(e.currentTarget.value) as 1 | 2 })}>
      <option value="1">{channelLabel({ ...params, channel: 1 })}</option>
      <option value="2">{channelLabel({ ...params, channel: 2 })}</option>
    </select>
  </div>

  <div data-help="decoderThreshold" style="display:flex;flex-direction:column;gap:8px">
  {@render toggleRow('Soglia automatica', thresholdAuto, toggleThreshold)}
  {#if !thresholdAuto}
    {@render stepRow('Soglia', formatVolts(threshold),
      () => stepThreshold(-1),
      () => stepThreshold(1),
    )}
  {/if}
  </div>

  {#if protocol === 'uart'}
    <div style="display:flex;flex-direction:column;gap:6px;padding-top:6px;border-top:1px solid #20262e">
      {@render toggleRow('Baud automatico', baudAuto, toggleBaudAuto)}
      {#if !baudAuto}
        {@render stepRow('Velocità', `${baud} baud`,
          () => withParams({ baud: stepBaud(baud, -1) }),
          () => withParams({ baud: stepBaud(baud, 1) }),
        )}
      {/if}
      {@render stepRow('Bit di dati', String(dataBits),
        () => stepDataBits(-1),
        () => stepDataBits(1),
      )}
      {@render segRow('Parità', PARITY_OPTIONS, parity, (value) => withParams({ parity: value as ParityName }))}
      {@render segRow('Bit di stop', STOP_OPTIONS, String(stopBits), (value) => withParams({ stop_bits: Number(value) }))}
      {@render toggleRow('Linea invertita', inverted, () => withParams({ inverted: !inverted }))}
    </div>
  {:else if protocol === 'can'}
    <div style="display:flex;flex-direction:column;gap:6px;padding-top:6px;border-top:1px solid #20262e">
      {@render segRow('Bitrate', CAN_BITRATE_OPTIONS, String(bitrate), (value) => withParams({ bitrate: Number(value) }))}
      {@render toggleRow('Linea invertita', inverted, () => withParams({ inverted: !inverted }))}
    </div>
  {:else if protocol === 'manchester'}
    <div style="display:flex;flex-direction:column;gap:6px;padding-top:6px;border-top:1px solid #20262e">
      {@render toggleRow('IEEE', ieee, () => withParams({ ieee: !ieee }))}
    </div>
  {:else}
    <div style="padding-top:8px;border-top:1px solid #20262e;font-size:13px;color:#8a94a1">Questo protocollo non ha parametri.</div>
  {/if}

  <div style="display:flex;flex-direction:column;gap:8px;padding-top:12px;border-top:1px solid #20262e">
    <span style="font-size:13px;font-weight:700">Legenda della striscia</span>
    <div style="display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:6px 10px">
      {#each LEGEND as entry (entry.label)}
        <div style="display:flex;align-items:center;gap:8px;font-size:12px;color:#c3cad4">
          <span style="width:26px;height:14px;border-radius:4px;background:{entry.color};flex:none"></span>{entry.label}
        </div>
      {/each}
    </div>
  </div>
</div>
