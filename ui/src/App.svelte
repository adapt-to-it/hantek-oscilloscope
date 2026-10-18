<script lang="ts">
  import { onMount } from 'svelte'
  import Scope from './lib/display/Scope.svelte'
  import { decodeDisplay } from './lib/display/frame'
  import * as api from './lib/api'
  import type { AcquisitionSettings, ChannelSettings, DisplayFrame, MeasureSnapshot, Status, ViewSettings } from './lib/types'
  import MeasureGrid from './lib/measures/MeasureGrid.svelte'
  import { formatHz } from './lib/shell/format'
  import TopBar from './lib/shell/TopBar.svelte'
  import ControlPanel from './lib/shell/ControlPanel.svelte'
  import Footer from './lib/shell/Footer.svelte'
  import { shortcutFor, toggleCommand, type RunCommand } from './lib/shell/keys'

  const STATUS_INTERVAL_MS = 500
  const ERROR_MS = 5000
  const MEASURE_INTERVAL_MS = 250

  let view = $state<ViewSettings | null>(null)
  let status = $state<Status | null>(null)
  let frame = $state<DisplayFrame | null>(null)
  let acq = $state<AcquisitionSettings | null>(null)
  let measures = $state<MeasureSnapshot | null>(null)
  const triggerLevel = $derived(acq?.triggerLevel ?? 0)
  const info = $derived(status ? `Frequenza ${formatHz(status.sampleRateHz)}, record ${status.recordLength} campioni` : undefined)
  let fps = $state(0)
  let errorMsg = $state<string | null>(null)

  let errorTimer: ReturnType<typeof setTimeout> | undefined
  let lastPoll = 0

  function showError(message: string) {
    errorMsg = message
    clearTimeout(errorTimer)
    errorTimer = setTimeout(() => (errorMsg = null), ERROR_MS)
  }

  /** Applica la vista; se il motore la rifiuta, ripristina quella precedente. */
  async function applyView(next: ViewSettings) {
    if (!view) return
    const previous = view
    view = next
    try {
      await api.setView(next)
    } catch (e) {
      view = previous
      showError(`Vista non applicata: ${e}`)
    }
  }

  async function send(command: RunCommand) {
    try {
      await api.runCommand(command)
    } catch (e) {
      showError(`Comando «${command}» non riuscito: ${e}`)
    }
  }

  function onRun() {
    send(toggleCommand(status?.runState ?? 'stopped'))
  }

  function onKey(event: KeyboardEvent) {
    const target = event.target instanceof HTMLElement ? event.target : null
    const command = shortcutFor(
      { key: event.key, ctrlKey: event.ctrlKey, metaKey: event.metaKey, altKey: event.altKey, target },
      status?.runState ?? 'stopped',
    )
    if (!command) return
    event.preventDefault()
    send(command)
  }

  function onColumns(columns: number) {
    api.setColumns(columns).catch((e) => showError(`Colonne del display: ${e}`))
  }

  function onOffset(channel: 0 | 1, offsetDiv: number) {
    if (!view) return
    const channels = [...view.channels] as [ChannelSettings, ChannelSettings]
    channels[channel] = { ...channels[channel], offsetDiv }
    applyView({ ...view, channels })
  }

  /** Applica trigger e acquisizione; se il motore li rifiuta, ripristina quelli precedenti. */
  async function applyAcq(next: AcquisitionSettings) {
    if (!acq) return
    const previous = acq
    acq = next
    try {
      await api.setAcquisition(next)
    } catch (e) {
      acq = previous
      showError(`Impostazioni non applicate: ${e}`)
    }
  }

  function onTriggerLevel(volts: number) {
    if (acq) applyAcq({ ...acq, triggerLevel: volts })
  }

  async function refreshMeasures() {
    try {
      measures = await api.getMeasurements()
    } catch (e) {
      showError(`Misure non disponibili: ${e}`)
    }
  }

  async function refreshStatus() {
    try {
      const next = await api.getStatus()
      const now = performance.now()
      if (status && now > lastPoll) {
        fps = Math.max(0, next.frames - status.frames) * (1000 / (now - lastPoll))
      }
      lastPoll = now
      status = next
    } catch (e) {
      showError(`Stato del motore non disponibile: ${e}`)
    }
  }

  onMount(() => {
    api
      .getView()
      .then(async (loaded) => {
        view = loaded
        acq = await api.getAcquisition()
        await api.subscribeDisplay((buf) => {
          frame = decodeDisplay(buf)
        })
      })
      .catch((e) => showError(`Avvio non riuscito: ${e}`))

    refreshStatus()
    const timer = setInterval(refreshStatus, STATUS_INTERVAL_MS)
    const measureTimer = setInterval(refreshMeasures, MEASURE_INTERVAL_MS)
    return () => {
      clearInterval(timer)
      clearInterval(measureTimer)
      clearTimeout(errorTimer)
    }
  })
</script>

<svelte:window onkeydown={onKey} />

<div class="app" style="display:flex;flex-direction:column;height:100vh;min-height:720px;background:#0b0d10;position:relative">
  <TopBar {status} onrun={onRun} onsingle={() => send('single')} />

  {#if errorMsg}
    <div role="alert" style="display:flex;align-items:center;gap:10px;padding:9px 16px;background:#3a1714;border-bottom:1px solid #6a3029;color:#ffb3a6;font-size:13px;font-weight:500">
      {errorMsg}
    </div>
  {/if}

  <main style="flex:1;min-height:0;display:flex;flex-wrap:wrap;gap:14px;padding:14px 16px;overflow:auto;align-items:stretch">
    <div style="flex:1 1 640px;min-width:0;display:flex;flex-direction:column;gap:12px">
      {#if view}
        <Scope {frame} {view} {triggerLevel} oncolumns={onColumns} onoffset={onOffset} ontriggerlevel={onTriggerLevel} />
        <MeasureGrid snapshot={measures} enabled={[view.channels[0].enabled, view.channels[1].enabled]} />
      {/if}
    </div>

    <aside class="card" style="flex:0 1 330px;min-width:300px;display:flex;flex-direction:column;min-height:420px;overflow:auto">
      {#if view && acq}
        <ControlPanel {view} {acq} {info} onchange={applyView} onacq={applyAcq} />
      {/if}
    </aside>
  </main>

  <Footer {status} {fps} />
</div>
