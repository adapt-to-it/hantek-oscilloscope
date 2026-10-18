<script lang="ts">
  import { onMount } from 'svelte'
  import Scope from './lib/display/Scope.svelte'
  import { decodeDisplay } from './lib/display/frame'
  import * as api from './lib/api'
  import type {
    AcquisitionSettings,
    ChannelSettings,
    DisplayFrame,
    DecodeView,
    DecoderSettings,
    RecordingView,
    DisplayOptions,
    CursorPositions,
    MathOp,
    ToolSettings,
    MeasureSnapshot,
    SpectrumView as SpectrumData,
    Status,
    ViewMode,
    ViewSettings,
    XyView as XyData,
  } from './lib/types'
  import SpectrumView from './lib/spectrum/SpectrumView.svelte'
  import XyView from './lib/xy/XyView.svelte'
  import DecoderStrip from './lib/decoder/DecoderStrip.svelte'
  import { channelLabel, defaultDecoder } from './lib/shell/controls/decoder'
  import MeasureGrid from './lib/measures/MeasureGrid.svelte'
  import { MeasureHistory } from './lib/measures/history'
  import { formatHz } from './lib/shell/format'
  import { tr } from './lib/i18n/msg/display'
  import TopBar from './lib/shell/TopBar.svelte'
  import ControlPanel from './lib/shell/ControlPanel.svelte'
  import Footer from './lib/shell/Footer.svelte'
  import HelpLayer from './lib/help/HelpLayer.svelte'
  import { nextMode, shortcutFor, toggleCommand, type RunCommand } from './lib/shell/keys'

  const STATUS_INTERVAL_MS = 500
  const ERROR_MS = 5000
  const MEASURE_INTERVAL_MS = 250
  const ANALYSIS_INTERVAL_MS = 100
  const DECODE_INTERVAL_MS = 200

  let view = $state<ViewSettings | null>(null)
  let status = $state<Status | null>(null)
  let frame = $state<DisplayFrame | null>(null)
  let acq = $state<AcquisitionSettings | null>(null)
  let measures = $state<MeasureSnapshot | null>(null)
  let display = $state<DisplayOptions>({ mode: 'time', window: 'flatTop', logFrequency: false, persistence: false, cursors: 'off', cursorChannel: 0 })
  let spectrum = $state<SpectrumData | null>(null)
  let xy = $state<XyData | null>(null)
  let decoder = $state<DecoderSettings>(defaultDecoder())
  let recording = $state<RecordingView | null>(null)
  let decoded = $state<DecodeView | null>(null)
  let decodeError = $state<string | null>(null)
  const triggerLevel = $derived(acq?.triggerLevel ?? 0)
  const info = $derived(status ? tr('info', { freq: formatHz(status.sampleRateHz), record: status.recordLength }) : undefined)
  let tools = $state<ToolSettings>({ math: 'off', filter: { channel: 0, kind: 'none', cutoffHz: 10e3 } })
  let cursors = $state<CursorPositions>({ t1: -1e-4, t2: 1e-4, v1: 1, v2: -1 })
  let explain = $state(false)
  const history = new MeasureHistory()
  let historyVersion = $state(0)
  /** Ultima operazione scelta: il tasto M la riattiva dopo averla spenta. */
  let lastMath: MathOp = 'add'
  let fps = $state(0)
  let errorMsg = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let noticeTimer: ReturnType<typeof setTimeout> | undefined

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
      showError(tr('viewNotApplied', { error: String(e) }))
    }
  }

  async function send(command: RunCommand) {
    try {
      await api.runCommand(command)
    } catch (e) {
      showError(tr('commandFailed', { command, error: String(e) }))
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
    if (command === 'view') setMode(nextMode(display.mode))
    else if (command === 'explain') explain = !explain
    else if (command === 'math') applyTools({ ...tools, math: tools.math === 'off' ? lastMath : 'off' })
    else if (command === 'screenshot') saveScreenshot()
    else send(command)
  }

  function onColumns(columns: number) {
    api.setColumns(columns).catch((e) => showError(tr('columnsError', { error: String(e) })))
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
      showError(tr('acqNotApplied', { error: String(e) }))
    }
  }

  function onTriggerLevel(volts: number) {
    if (acq) applyAcq({ ...acq, triggerLevel: volts })
  }

  function setMode(mode: ViewMode) {
    display = { ...display, mode }
    refreshAnalysis()
  }

  /** Chiede al motore lo spettro o i punti XY, solo per la vista visibile. */
  async function refreshAnalysis() {
    try {
      if (display.mode === 'spectrum') spectrum = await api.getSpectrum(display.window)
      else if (display.mode === 'xy') xy = await api.getXy()
    } catch (e) {
      showError(tr('analysisUnavailable', { error: String(e) }))
    }
  }

  /** Decodifica il frame più recente se il decoder è attivo; gli errori restano nella striscia. */
  async function refreshDecode() {
    if (!decoder.enabled) return
    try {
      decoded = await api.decode(decoder.params)
      decodeError = null
    } catch (e) {
      decodeError = String(e)
    }
  }

  function applyDecoder(next: DecoderSettings) {
    decoder = next
    if (!next.enabled) {
      decoded = null
      decodeError = null
    }
    refreshDecode()
  }

  /** Applica gli strumenti; se il motore li rifiuta, ripristina quelli precedenti. */
  async function applyTools(next: ToolSettings) {
    const previous = tools
    tools = next
    if (next.math !== 'off') lastMath = next.math
    try {
      await api.setTools(next)
    } catch (e) {
      tools = previous
      showError(tr('toolsNotApplied', { error: String(e) }))
    }
  }

  function showNotice(message: string) {
    notice = message
    clearTimeout(noticeTimer)
    noticeTimer = setTimeout(() => (notice = null), ERROR_MS)
  }

  /** Salva il canvas del display in PNG nella cartella delle registrazioni. */
  async function saveScreenshot() {
    const canvas = document.querySelector<HTMLCanvasElement>('main canvas')
    if (!canvas) return showError(tr('noDisplayToSave'))
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
    if (!blob) return showError(tr('imageNotCreated'))
    try {
      const path = await api.savePng(new Uint8Array(await blob.arrayBuffer()))
      showNotice(tr('imageSaved', { path }))
    } catch (e) {
      showError(tr('imageNotSaved', { error: String(e) }))
    }
  }

  async function refreshMeasures() {
    try {
      measures = await api.getMeasurements()
      if (measures && status?.runState !== 'stopped') {
        history.push(measures, Date.now())
        historyVersion++
      }
    } catch (e) {
      showError(tr('measuresUnavailable', { error: String(e) }))
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
      showError(tr('engineStatusUnavailable', { error: String(e) }))
    }
  }

  onMount(() => {
    api
      .getView()
      .then(async (loaded) => {
        view = loaded
        acq = await api.getAcquisition()
        tools = await api.getTools()
        const mode = await api.getInitialMode()
        if (mode) setMode(mode)
        await api.subscribeDisplay((buf) => {
          frame = decodeDisplay(buf)
        })
      })
      .catch((e) => showError(tr('startFailed', { error: String(e) })))

    refreshStatus()
    const timer = setInterval(refreshStatus, STATUS_INTERVAL_MS)
    const measureTimer = setInterval(refreshMeasures, MEASURE_INTERVAL_MS)
    const analysisTimer = setInterval(refreshAnalysis, ANALYSIS_INTERVAL_MS)
    const decodeTimer = setInterval(refreshDecode, DECODE_INTERVAL_MS)
    return () => {
      clearInterval(timer)
      clearInterval(measureTimer)
      clearInterval(analysisTimer)
      clearInterval(decodeTimer)
      clearTimeout(errorTimer)
      clearTimeout(noticeTimer)
    }
  })
</script>

<svelte:window onkeydown={onKey} />

<div class="app {explain ? 'exp' : ''}" style="display:flex;flex-direction:column;height:100vh;min-height:720px;background:#0b0d10;position:relative">
  <TopBar {status} {recording} mode={display.mode} triggered={frame?.triggered ?? null} {explain} onexplain={() => (explain = !explain)} onrun={onRun} onsingle={() => send('single')} onmode={setMode} />

  {#if notice}
    <div role="status" style="padding:9px 16px;background:#14261c;border-bottom:1px solid #295a3c;color:#9fe3bb;font-size:13px;font-weight:500">{notice}</div>
  {/if}
  {#if errorMsg}
    <div role="alert" style="display:flex;align-items:center;gap:10px;padding:9px 16px;background:#3a1714;border-bottom:1px solid #6a3029;color:#ffb3a6;font-size:13px;font-weight:500">
      {errorMsg}
    </div>
  {/if}

  <main style="flex:1;min-height:0;display:flex;flex-wrap:wrap;gap:14px;padding:14px 16px;overflow:auto;align-items:stretch">
    <div style="flex:1 1 640px;min-width:0;display:flex;flex-direction:column;gap:12px">
      {#if view}
        {#if display.mode === 'spectrum'}
          <SpectrumView data={spectrum} logFrequency={display.logFrequency} />
        {:else if display.mode === 'xy'}
          <XyView data={xy} {view} />
        {:else}
          <Scope {frame} {view} {display} {cursors} oncursors={(c) => (cursors = c)} {triggerLevel} frequencies={[measures?.channels[0]?.frequency ?? null, measures?.channels[1]?.frequency ?? null]} oncolumns={onColumns} onoffset={onOffset} ontriggerlevel={onTriggerLevel} />
        {/if}
        {#if decoder.enabled && display.mode === 'time'}
          <DecoderStrip
            view={decoded}
            error={decodeError}
            tStart={frame?.tStart ?? 0}
            tEnd={frame?.tEnd ?? 0}
            channelLabel={channelLabel(decoder.params)}
          />
        {/if}
        <MeasureGrid snapshot={measures} enabled={[view.channels[0].enabled, view.channels[1].enabled]} {history} version={historyVersion} />
      {/if}
    </div>

    <!-- Il contenuto del pannello non allunga la riga: scorre dentro la sidebar. -->
    <aside class="card" style="flex:0 1 330px;min-width:300px;min-height:420px;position:relative;overflow:hidden">
      <div style="position:absolute;inset:0;display:flex;flex-direction:column;overflow:auto">
      {#if view && acq}
        <ControlPanel
          {view}
          {acq}
          {display}
          {decoder}
          {tools}
          ontools={applyTools}
          {info}
          {recording}
          onrecording={(r) => (recording = r)}
          onchange={applyView}
          onacq={applyAcq}
          ondecoder={applyDecoder}
          ondisplay={(next) => {
            display = next
            refreshAnalysis()
          }}
        />
      {/if}
      </div>
    </aside>
  </main>

  <Footer {status} {fps} />
  <HelpLayer active={explain} />
</div>
