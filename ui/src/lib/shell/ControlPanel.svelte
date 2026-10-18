<script lang="ts">
  // Contenuto dell'aside, con le schede Controlli, Decoder e Strumenti. Controlli: fisarmonica con Orizzontale, un canale per volta, Trigger e Display.
  // Una sola sezione aperta alla volta. Ogni modifica produce un oggetto nuovo; l'applicazione al motore la fa il genitore.
  import type { AcquisitionSettings, ChannelSettings, DecoderSettings, DisplayOptions, RecordingView, ToolSettings, ViewSettings } from '../types'
  import { CH_COLORS } from '../types'
  import { formatTime, formatVolts } from './format'
  import { triggerSummary } from './controls/trigger'
  import HorizontalSection from './controls/HorizontalSection.svelte'
  import ChannelSection from './controls/ChannelSection.svelte'
  import TriggerSection from './controls/TriggerSection.svelte'
  import DisplaySection, { WINDOW_OPTIONS } from './controls/DisplaySection.svelte'
  import DecoderSection from './controls/DecoderSection.svelte'
  import { decoderSummary } from './controls/decoder'
  import ToolsTab from './tools/ToolsTab.svelte'
  import { tr } from '../i18n/msg/shell'

  interface Props {
    view: ViewSettings
    acq: AcquisitionSettings
    display: DisplayOptions
    decoder: DecoderSettings
    tools: ToolSettings
    onchange: (view: ViewSettings) => void
    onacq: (acq: AcquisitionSettings) => void
    ondisplay: (display: DisplayOptions) => void
    ondecoder: (decoder: DecoderSettings) => void
    ontools: (t: ToolSettings) => void
    /** Riga informativa della sezione Orizzontale, per esempio la frequenza di campionamento. */
    info?: string
    /** Stato della registrazione: il badge di Strumenti segnala una registrazione in corso. */
    recording: RecordingView | null
    onrecording: (r: RecordingView | null) => void
  }

  let { view, acq, display, decoder, tools, onchange, onacq, ondisplay, ondecoder, ontools, info, recording, onrecording }: Props = $props()

  type TabId = 'controls' | 'decoder' | 'tools'

  const TABS = [
    { id: 'controls', label: 'tabControls' },
    { id: 'decoder', label: 'tabDecoder' },
    { id: 'tools', label: 'tabTools' },
  ] as const

  let tab = $state<TabId>('controls')

  /** Pallino di stato accanto all'etichetta: verde con il decoder attivo, rosso con una registrazione in corso. */
  const badges = $derived<Record<TabId, string | null>>({
    controls: null,
    decoder: decoder.enabled ? '#3fd68a' : null,
    tools: recording?.running === true ? '#ff7a6b' : null,
  })

  type SectionId = 'horizontal' | 'ch1' | 'ch2' | 'trigger' | 'display'

  interface Section {
    id: SectionId
    name: string
    color: string
    summary: string
    /** Indice del canale per le sezioni di canale, altrimenti null. */
    channel: 0 | 1 | null
    /** Canale spento: l'intestazione si attenua come nel design. */
    dim: boolean
  }

  let openSection = $state<SectionId | null>('horizontal')

  const windowLabel = $derived(WINDOW_OPTIONS.find((option) => option.window === display.window)?.label ?? display.window)

  const sections: Section[] = $derived([
    {
      id: 'horizontal',
      name: tr('sectionHorizontal'),
      color: '#e8ebef',
      summary: `${formatTime(view.timePerDiv)}/div`,
      channel: null,
      dim: false,
    },
    {
      id: 'ch1',
      name: 'CH1',
      color: CH_COLORS[0],
      summary: `${formatVolts(view.channels[0].voltsPerDiv)}/div`,
      channel: 0,
      dim: !view.channels[0].enabled,
    },
    {
      id: 'ch2',
      name: 'CH2',
      color: CH_COLORS[1],
      summary: `${formatVolts(view.channels[1].voltsPerDiv)}/div`,
      channel: 1,
      dim: !view.channels[1].enabled,
    },
    {
      id: 'trigger',
      name: tr('sectionTrigger'),
      color: '#ff9f43',
      summary: triggerSummary(acq),
      channel: null,
      dim: false,
    },
    {
      id: 'display',
      name: tr('sectionDisplay'),
      color: '#e8ebef',
      summary: windowLabel,
      channel: null,
      dim: false,
    },
  ])

  function toggle(id: SectionId) {
    openSection = openSection === id ? null : id
  }

  function toggleChannel(index: 0 | 1) {
    const channels = [...view.channels] as [ChannelSettings, ChannelSettings]
    channels[index] = { ...channels[index], enabled: !channels[index].enabled }
    onchange({ ...view, channels })
  }
</script>

<div style="display:flex;flex-direction:column">
  <div style="position:sticky;top:0;z-index:5;background:#12151a;padding:10px 12px;border-bottom:1px solid #20262e">
    <div class="segrow" role="group" aria-label={tr('panelAria')} data-help="panelTabs" style="display:flex;width:100%">
      {#each TABS as option (option.id)}
        <button class="seg {tab === option.id ? 'on' : ''}" style="flex:1" aria-pressed={tab === option.id}
          onclick={() => (tab = option.id)}>{tr(option.label)}{#if badges[option.id]}<span style="margin-left:6px;width:7px;height:7px;border-radius:50%;background:{badges[option.id]};display:inline-block"></span>{/if}</button>
      {/each}
    </div>
  </div>

  {#if tab === 'controls'}
    {#each sections as section (section.id)}
      <div style="border-bottom:1px solid #20262e;opacity:{section.dim ? 0.55 : 1}">
        <div class="acc-h">
          <button class="acc-t" aria-expanded={openSection === section.id} onclick={() => toggle(section.id)}>
            <span style="color:{section.color}">{section.name}</span>
            <span class="mono" style="font-weight:500;font-size:12px;color:#9aa3af;margin-left:auto">{section.summary}</span>
          </button>
          {#if section.channel !== null}
            {@const index = section.channel}
            <button data-help="channelEnable" class="tgl {index === 0 ? 'c1' : 'c2'} {view.channels[index].enabled ? 'on' : ''}"
              role="switch" aria-checked={view.channels[index].enabled} aria-label={tr('channelActive', { nome: section.name })}
              onclick={() => toggleChannel(index)}></button>
          {/if}
        </div>
        {#if openSection === section.id}
          <div style="padding:0 14px 14px">
            {#if section.id === 'horizontal'}
              <HorizontalSection {view} {acq} {onchange} {onacq} {info} />
            {:else if section.id === 'trigger'}
              <TriggerSection {acq} {onacq} />
            {:else if section.id === 'display'}
              <DisplaySection {display} {ondisplay} />
            {:else if section.channel !== null}
              <ChannelSection index={section.channel} {view} {acq} {onchange} {onacq} />
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {:else if tab === 'decoder'}
    <div style="padding:14px" data-help="decoder">
      <DecoderSection {decoder} {ondecoder} />
    </div>
  {:else}
    <ToolsTab {view} {decoder} {tools} {ontools} {onrecording} mode={display.mode} />
  {/if}
</div>
