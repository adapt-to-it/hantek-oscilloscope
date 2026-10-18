<script lang="ts">
  // Barra superiore: dispositivo, Run/Stop, Singolo, stato, selettore di vista, registrazione, Spiegazioni, lingua e server MCP.
  // Screenshot è fuori da questa unità.
  import type { RecordingView, Status, ViewMode } from '../types'
  import Tip from '../help/Tip.svelte'
  import { toggleCommand } from './keys'
  import { firmwareLabel, mcpChip, runPill } from './status'
  import { tr } from '../i18n/msg/shell'
  import { LOCALES, LOCALE_NAMES, getLocale, setLocale, type Locale } from '../i18n/i18n.svelte'

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
  const DEVICE_LABEL_KEY = { hardware: 'hardware', simulator: 'simulator' } as const
  const DEVICE_NOTE_KEY = { hardware: 'noteHardware', simulator: 'noteSimulator' } as const
  const VIEWS = [
    { mode: 'time', label: 'viewTime' },
    { mode: 'spectrum', label: 'viewSpectrum' },
    { mode: 'xy', label: 'viewXy' },
  ] as const

  let devOpen = $state(false)
  let devColor = $derived(status ? DEVICE_COLOR[status.source] : '#8a94a1')
  let devLabel = $derived(status ? tr(DEVICE_LABEL_KEY[status.source]) : tr('starting'))
  let devNote = $derived(status ? tr(DEVICE_NOTE_KEY[status.source]) : tr('noteMissing'))
  let calibration = $derived(status?.source === 'hardware' ? tr('calibrationRead') : '—')
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
      <button class="chip" onclick={() => (devOpen = !devOpen)} aria-expanded={devOpen} aria-label={tr('deviceAria', { stato: devLabel })}>
        <span style="width:8px;height:8px;border-radius:50%;background:{devColor};display:inline-block"></span>
        <span>Hantek 6022BE</span><span style="color:#9aa3af;font-weight:500">{devLabel}</span>
      </button>
      {#if devOpen}
        <div class="card" style="position:absolute;left:0;top:42px;width:320px;padding:14px 16px;display:flex;flex-direction:column;gap:10px;background:#0b0d10;border-color:#3a4350;box-shadow:0 12px 32px #000b">
          <div style="font-size:14px;font-weight:700">{tr('devicePanel')}</div>
          <div class="rw" style="min-height:26px"><span class="lb">{tr('deviceState')}</span><span class="mono" style="font-size:13px;color:{devColor}">{devLabel}</span></div>
          <div class="rw" style="min-height:26px"><span class="lb">{tr('firmware')}</span><span class="mono" style="font-size:13px;color:#9aa3af">{firmware}</span></div>
          <div class="rw" style="min-height:26px"><span class="lb">{tr('calibration')}</span><span class="mono" style="font-size:13px;color:#9aa3af">{calibration}</span></div>
          <div style="font-size:13px;line-height:1.45;color:#9aa3af;text-wrap:pretty;border-top:1px solid #20262e;padding-top:10px">{devNote}</div>
        </div>
      {/if}
    </div>
  </Tip>

  <div style="display:flex;gap:8px;align-items:center">
    <Tip id="run">
      <button class="btn {runCls}" style="min-width:112px" onclick={onrun}>
        {runLabel} <span class="kbd">{tr('kbdSpace')}</span>
      </button>
    </Tip>
    <Tip id="single">
      <button class="btn" onclick={onsingle}>{tr('single')} <span class="kbd">S</span></button>
    </Tip>
  </div>

  <div class="chip" role="status" data-help="status" style="gap:8px"><span style="width:8px;height:8px;border-radius:50%;background:{pill.color};display:inline-block"></span>{pill.text}</div>

  <div style="flex:1 1 10px"></div>

  <div style="display:flex;align-items:center;gap:6px">
    <Tip id="view">
      <div class="segrow" role="group" aria-label={tr('viewGroup')}>
        {#each VIEWS as view (view.mode)}
          <button class="seg {view.mode === mode ? 'on' : ''}" aria-pressed={view.mode === mode}
            onclick={() => onmode(view.mode)}>{tr(view.label)}</button>
        {/each}
      </div>
    </Tip>
    <span class="kbd" title={tr('viewKeyTitle')}>F</span>
  </div>

  <button data-help="explain" class="btn {explain ? 'on' : ''}" aria-pressed={explain} onclick={onexplain}>{tr('explain')} <span class="kbd">i</span></button>

  <select class="chip" style="cursor:pointer;color:#e8ebef;font-size:13px;font-weight:600" aria-label={tr('langLabel')} value={getLocale()}
    onchange={(event) => setLocale(event.currentTarget.value as Locale)}>
    {#each LOCALES as locale (locale)}
      <option value={locale}>{LOCALE_NAMES[locale]}</option>
    {/each}
  </select>

  {#if recording?.running}
    <div class="chip mono" role="status" style="font-size:12px;color:#ff6b6b" title={recording.path}>
      <span style="width:8px;height:8px;border-radius:50%;background:#ff6b6b;display:inline-block"></span>{tr('recFrames', { n: recording.frames })}
    </div>
  {/if}

  <Tip id="mcp">
    <div class="chip mono" style="font-size:12px;font-weight:500;color:{mcp.color}" title={tr('mcpServerTitle')}>
      <span style="width:8px;height:8px;border-radius:50%;background:{mcp.color};display:inline-block"></span>{mcp.text}
    </div>
  </Tip>
</header>
