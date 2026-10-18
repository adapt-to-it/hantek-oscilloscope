<script lang="ts">
  // Barra superiore: dispositivo, Run/Stop, Singolo, stato e server MCP.
  // Spiegazioni, screenshot e segmenti di vista sono fuori da questa unità.
  import type { Status } from '../types'
  import { toggleCommand } from './keys'

  interface Props {
    status: Status | null
    onrun: () => void
    onsingle: () => void
  }

  let { status, onrun, onsingle }: Props = $props()

  const DEVICE_COLOR = { hardware: '#3fd68a', simulator: '#ff9f43' } as const
  const DEVICE_LABEL = { hardware: 'Hardware', simulator: 'Simulatore' } as const
  const STATE_TEXT = { running: 'In esecuzione', armed: 'In attesa del trigger', stopped: 'Fermo' } as const
  const STATE_COLOR = { running: '#3fd68a', armed: '#ff9f43', stopped: '#8a94a1' } as const

  let devColor = $derived(status ? DEVICE_COLOR[status.source] : '#8a94a1')
  let devLabel = $derived(status ? DEVICE_LABEL[status.source] : 'In avvio')
  let stateText = $derived(status ? STATE_TEXT[status.runState] : '—')
  let stateColor = $derived(status ? STATE_COLOR[status.runState] : '#8a94a1')
  let isStopped = $derived(!status || status.runState === 'stopped')
  let runCls = $derived(isStopped ? 'run' : 'stop')
  let runLabel = $derived(toggleCommand(status?.runState ?? 'stopped') === 'run' ? 'Run' : 'Stop')
</script>

<header style="position:relative;z-index:40;display:flex;flex-wrap:wrap;align-items:center;gap:10px 14px;padding:10px 16px;background:#0f1216;border-bottom:1px solid #20262e">
  <div class="chip" aria-label="Dispositivo: {devLabel}">
    <span style="width:8px;height:8px;border-radius:50%;background:{devColor};display:inline-block"></span>
    <span>Hantek 6022BE</span><span style="color:#9aa3af;font-weight:500">{devLabel}</span>
  </div>

  <div style="display:flex;gap:8px;align-items:center">
    <button class="btn {runCls}" style="min-width:112px" onclick={onrun}>
      {runLabel} <span class="kbd">Spazio</span>
    </button>
    <button class="btn" onclick={onsingle}>Singolo <span class="kbd">S</span></button>
  </div>

  <div class="chip" role="status" style="gap:8px"><span style="width:8px;height:8px;border-radius:50%;background:{stateColor};display:inline-block"></span>{stateText}</div>

  <div style="flex:1 1 10px"></div>

  <div class="chip mono" style="font-size:12px;font-weight:500;color:#9aa3af" title="Server MCP per Claude Code">
    <span style="width:8px;height:8px;border-radius:50%;background:#8a94a1;display:inline-block"></span>{status?.server ?? '—'}
  </div>
</header>
