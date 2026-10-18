<script lang="ts">
  // Contenuto dell'aside, tab "Controlli": fisarmonica con Orizzontale, un canale per volta, Trigger e Display.
  // Una sola sezione aperta alla volta. Ogni modifica produce un oggetto nuovo; l'applicazione al motore la fa il genitore.
  import type { AcquisitionSettings, ChannelSettings, DisplayOptions, ViewSettings } from '../types'
  import { CH_COLORS } from '../types'
  import { formatTime, formatVolts } from './format'
  import { triggerSummary } from './controls/trigger'
  import HorizontalSection from './controls/HorizontalSection.svelte'
  import ChannelSection from './controls/ChannelSection.svelte'
  import TriggerSection from './controls/TriggerSection.svelte'
  import DisplaySection, { WINDOW_OPTIONS } from './controls/DisplaySection.svelte'

  interface Props {
    view: ViewSettings
    acq: AcquisitionSettings
    display: DisplayOptions
    onchange: (view: ViewSettings) => void
    onacq: (acq: AcquisitionSettings) => void
    ondisplay: (display: DisplayOptions) => void
    /** Riga informativa della sezione Orizzontale, per esempio la frequenza di campionamento. */
    info?: string
  }

  let { view, acq, display, onchange, onacq, ondisplay, info }: Props = $props()

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
      name: 'Orizzontale',
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
      name: 'Trigger',
      color: '#ff9f43',
      summary: triggerSummary(acq),
      channel: null,
      dim: false,
    },
    {
      id: 'display',
      name: 'Display',
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
  {#each sections as section (section.id)}
    <div style="border-bottom:1px solid #20262e;opacity:{section.dim ? 0.55 : 1}">
      <div class="acc-h">
        <button class="acc-t" aria-expanded={openSection === section.id} onclick={() => toggle(section.id)}>
          <span style="color:{section.color}">{section.name}</span>
          <span class="mono" style="font-weight:500;font-size:12px;color:#9aa3af;margin-left:auto">{section.summary}</span>
        </button>
        {#if section.channel !== null}
          {@const index = section.channel}
          <button class="tgl {index === 0 ? 'c1' : 'c2'} {view.channels[index].enabled ? 'on' : ''}"
            role="switch" aria-checked={view.channels[index].enabled} aria-label="{section.name} attivo"
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
</div>
