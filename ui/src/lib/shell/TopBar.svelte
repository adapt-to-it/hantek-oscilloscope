<script lang="ts">
  // Barra superiore: dispositivo, Run/Stop, Singolo, stato, selettore di vista, registrazione, Spiegazioni e server MCP.
  // Screenshot è fuori da questa unità.
  import type { RecordingView, Status, ViewMode } from '../types'
  import Tip from '../help/Tip.svelte'
  import { toggleCommand } from './keys'
  import { firmwareLabel, mcpChip, runPill } from './status'

  interface Props {
    status: Status | null
    mode: ViewMode
    /** Trigger agganciato; null se non si sa. */
    triggered: boolean | null
    /** Modalità Spiegazioni attiva. */
    explain: boolean
    onrun: () => void
    onsingle: () => void
    onmode: (mode: ViewMode) => void
    onexplain: () => void
    /** Registrazione in corso: la barra mostra REC e i frame scritti. */
    recording: RecordingView | null
  }

  let { status, mode, triggered, explain, onrun, onsingle, onmode, onexplain, recording }: Props = $props()

  const DEVICE_COLOR = { hardware: '#3fd68a', simulator: '#ff9f43' } as const
  const DEVICE_LABEL = { hardware: 'Hardware', simulator: 'Simulatore' } as const
  const DEVICE_NOTE = {
    hardware: 'Dispositivo collegato via USB. Le misure vengono dal segnale reale.',
    simulator: 'Nessun 6022BE collegato: i segnali sono simulati. Collega lo strumento via USB per misurare segnali reali.',
  } as const
  const VIEWS: { mode: ViewMode; label: string }[] = [
    { mode: 'time', label: 'Tempo' },
    { mode: 'spectrum', label: 'Spettro' },
    { mode: 'xy', label: 'XY' },
  ]

  let devOpen = $state(false)
  let devColor = $derived(status ? DEVICE_COLOR[status.source] : '#8a94a1')
  let devLabel = $derived(status ? DEVICE_LABEL[status.source] : 'In avvio')
  let devNote = $derived(status ? DEVICE_NOTE[status.source] : 'Nessun dispositivo trovato. Controlla il cavo USB e riprova.')
  let calibration = $derived(status?.source === 'hardware' ? 'EEPROM letta' : '—')
  let firmware = $derived(firmwareLabel(status?.firmware ?? null))
  let pill = $derived(runPill(status, triggered))
  let mcp = $derived(status ? mcpChip(status.server) : { text: '—', color: '#8a94a1' })
  let isStopped = $derived(!status || status.runState === 'stopped')
  let runCls = $derived(isStopped ? 'run' : 'stop')
  let runLabel = $derived(toggleCommand(status?.runState ?? 'stopped') === 'run' ? 'Run' : 'Stop')
</script>

<header style="position:relative;z-index:40;display:flex;flex-wrap:wrap;align-items:center;gap:10px 14px;padding:10px 16px;background:#0f1216;border-bottom:1px solid #20262e">
  <Tip id="device">
    <div style="position:relative">
      <button class="chip" onclick={() => (devOpen = !devOpen)} aria-expanded={devOpen} aria-label="Dispositivo: {devLabel}">
        <span style="width:8px;height:8px;border-radius:50%;background:{devColor};display:inline-block"></span>
        <span>Hantek 6022BE</span><span style="color:#9aa3af;font-weight:500">{devLabel}</span>
      </button>
      {#if devOpen}
        <div class="card" style="position:absolute;left:0;top:42px;width:320px;padding:14px 16px;display:flex;flex-direction:column;gap:10px;background:#0b0d10;border-color:#3a4350;box-shadow:0 12px 32px #000b">
          <div style="font-size:14px;font-weight:700">Dispositivo</div>
          <div class="rw" style="min-height:26px"><span class="lb">Stato</span><span class="mono" style="font-size:13px;color:{devColor}">{devLabel}</span></div>
          <div class="rw" style="min-height:26px"><span class="lb">Firmware</span><span class="mono" style="font-size:13px;color:#9aa3af">{firmware}</span></div>
          <div class="rw" style="min-height:26px"><span class="lb">Calibrazione</span><span class="mono" style="font-size:13px;color:#9aa3af">{calibration}</span></div>
          <div style="font-size:13px;line-height:1.45;color:#9aa3af;text-wrap:pretty;border-top:1px solid #20262e;padding-top:10px">{devNote}</div>
        </div>
      {/if}
    </div>
  </Tip>

  <div style="display:flex;gap:8px;align-items:center">
    <Tip id="run">
      <button class="btn {runCls}" style="min-width:112px" onclick={onrun}>
        {runLabel} <span class="kbd">Spazio</span>
      </button>
    </Tip>
    <Tip id="single">
      <button class="btn" onclick={onsingle}>Singolo <span class="kbd">S</span></button>
    </Tip>
  </div>

  <div class="chip" role="status" data-help="status" style="gap:8px"><span style="width:8px;height:8px;border-radius:50%;background:{pill.color};display:inline-block"></span>{pill.text}</div>

  <div style="flex:1 1 10px"></div>

  <div style="display:flex;align-items:center;gap:6px">
    <Tip id="view">
      <div class="segrow" role="group" aria-label="Vista">
        {#each VIEWS as view (view.mode)}
          <button class="seg {view.mode === mode ? 'on' : ''}" aria-pressed={view.mode === mode}
            onclick={() => onmode(view.mode)}>{view.label}</button>
        {/each}
      </div>
    </Tip>
    <span class="kbd" title="Tasto F: cambia vista">F</span>
  </div>

  <button data-help="explain" class="btn {explain ? 'on' : ''}" aria-pressed={explain} onclick={onexplain}>Spiegazioni <span class="kbd">i</span></button>

  {#if recording?.running}
    <div class="chip mono" role="status" style="font-size:12px;color:#ff6b6b" title={recording.path}>
      <span style="width:8px;height:8px;border-radius:50%;background:#ff6b6b;display:inline-block"></span>REC · {recording.frames} frame
    </div>
  {/if}

  <Tip id="mcp">
    <div class="chip mono" style="font-size:12px;font-weight:500;color:{mcp.color}" title="Server MCP per Claude Code">
      <span style="width:8px;height:8px;border-radius:50%;background:{mcp.color};display:inline-block"></span>{mcp.text}
    </div>
  </Tip>
</header>
