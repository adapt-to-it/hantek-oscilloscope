<script lang="ts">
  // Corpo della sezione File: registrazione dei frame su disco ed export del logic analyzer.
  import type { DecoderSettings, ExportFormat, RecordFormat, RecordingView } from '../../types'
  import { exportLogic, recordingStatus, startRecording, stopRecording } from '../../api'
  import { tr } from '../../i18n/msg/decoder'
  import { defaultFileName, fileName, formatBytes, limitOptions, recordingLabel } from './file'

  interface Props {
    decoder: DecoderSettings
    onrecording?: (r: RecordingView | null) => void
  }

  let { decoder, onrecording }: Props = $props()

  const POLL_MS = 500
  const RECORD_FORMATS: readonly { value: RecordFormat; label: string }[] = [
    { value: 'csv', label: 'CSV' },
    { value: 'npy', label: 'NPY' },
    { value: 'wav', label: 'WAV' },
  ]
  const EXPORT_FORMATS: readonly { value: ExportFormat; label: string }[] = [
    { value: 'vcd', label: 'VCD' },
    { value: 'sr', label: 'Sigrok .sr' },
  ]
  const LIMIT_OPTIONS = $derived(limitOptions().map((l) => ({ value: l.key, label: l.label })))

  let recFormat = $state<RecordFormat>('csv')
  let limit = $state('manual')
  let status = $state<RecordingView | null>(null)
  let recError = $state('')
  let recSaved = $state('')
  let polling = false
  let stopping = false

  let exportFormat = $state<ExportFormat>('vcd')
  let includeDecode = $state(false)
  let exportError = $state('')
  let exportSaved = $state('')

  const running = $derived(status?.running === true)
  const maxSeconds = $derived(limitOptions().find((l) => l.key === limit)?.seconds ?? null)
  const decodeAvailable = $derived(exportFormat === 'vcd' && decoder.enabled)
  const decodeIncluded = $derived(decodeAvailable && includeDecode)

  $effect(() => {
    if (!running) return
    const id = setInterval(poll, POLL_MS)
    return () => clearInterval(id)
  })

  function setStatus(r: RecordingView | null) {
    status = r
    onrecording?.(r)
  }

  async function start() {
    recError = ''
    recSaved = ''
    try {
      const view = await startRecording({
        path: defaultFileName('h6022', recFormat, new Date()),
        format: recFormat,
        max_frames: null,
        max_seconds: maxSeconds,
      })
      setStatus(view)
    } catch (e) {
      recError = String(e)
    }
  }

  async function finish(current: RecordingView) {
    if (stopping) return
    stopping = true
    try {
      const summary = await stopRecording()
      setStatus({ ...current, running: false, frames: summary.frames })
      recSaved = tr('savedRecording', { name: fileName(summary.path), frames: summary.frames })
    } catch (e) {
      recError = String(e)
    } finally {
      stopping = false
    }
  }

  async function stop() {
    if (status) await finish(status)
  }

  async function poll() {
    if (polling) return
    polling = true
    try {
      const s = await recordingStatus()
      setStatus(s)
      if (s !== null && !s.running) await finish(s)
    } catch (e) {
      recError = String(e)
    } finally {
      polling = false
    }
  }

  async function exportFrames() {
    exportError = ''
    exportSaved = ''
    try {
      const view = await exportLogic({
        path: defaultFileName('h6022-logic', exportFormat, new Date()),
        format: exportFormat,
        decode: decodeIncluded ? decoder.params : null,
      })
      exportSaved = tr('savedExport', { name: fileName(view.path), size: formatBytes(view.bytes) })
    } catch (e) {
      exportError = String(e)
    }
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

<div style="display:flex;flex-direction:column;gap:8px">
  <span style="font-size:13px;font-weight:700">{tr('recordingTitle')}</span>
  {@render segRow(tr('format'), RECORD_FORMATS, recFormat, (value) => (recFormat = value as RecordFormat))}
  {@render segRow(tr('duration'), LIMIT_OPTIONS, limit, (value) => (limit = value))}

  <div class="rw">
    {#if running}
      <button class="btn stop" onclick={stop}>{tr('stop')}</button>
    {:else}
      <button class="btn run" onclick={start}>{tr('record')}</button>
    {/if}
    <span class="mono" style="font-size:13px;color:#c3cad4">{recordingLabel(status)}</span>
  </div>
  {#if recError}
    <div style="font-size:13px;color:#ff6b6b">{recError}</div>
  {/if}
  {#if recSaved}
    <div style="font-size:13px;color:#c3cad4">{recSaved}</div>
  {/if}

  <div style="display:flex;flex-direction:column;gap:8px;padding-top:12px;border-top:1px solid #20262e">
    <span style="font-size:13px;font-weight:700">{tr('exportTitle')}</span>
    {@render segRow(tr('format'), EXPORT_FORMATS, exportFormat, (value) => (exportFormat = value as ExportFormat))}

    <div class="rw">
      <span class="lb">{tr('includeDecoder')}</span>
      <button class="tgl {decodeIncluded ? 'on' : ''}" role="switch" aria-checked={decodeIncluded}
        aria-label={tr('includeDecoder')} disabled={!decodeAvailable}
        style={decodeAvailable ? '' : 'opacity:0.4'}
        onclick={() => (includeDecode = !includeDecode)}></button>
    </div>

    <button class="btn" onclick={exportFrames}>{tr('exportFrames')}</button>
    {#if exportError}
      <div style="font-size:13px;color:#ff6b6b">{exportError}</div>
    {/if}
    {#if exportSaved}
      <div style="font-size:13px;color:#c3cad4">{exportSaved}</div>
    {/if}
  </div>

  <div style="font-size:12px;color:#8a94a1">{tr('filesIn', { dir: '~/h6022-recordings' })}</div>
</div>
