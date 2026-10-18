<script lang="ts" module>
  // Opzioni della finestra FFT, esportate per l'intestazione della sezione in ControlPanel.
  import type { WindowName } from '../../types'

  export const WINDOW_OPTIONS: { window: WindowName; label: string }[] = [
    { window: 'rectangular', label: 'Rettangolare' },
    { window: 'hann', label: 'Hann' },
    { window: 'hamming', label: 'Hamming' },
    { window: 'blackmanHarris', label: 'Blackman-Harris' },
    { window: 'flatTop', label: 'Flat-top' },
  ]
</script>

<script lang="ts">
  // Corpo della sezione Display: finestra FFT e asse delle frequenze logaritmico.
  import type { DisplayOptions } from '../../types'

  interface Props {
    display: DisplayOptions
    ondisplay: (display: DisplayOptions) => void
  }

  let { display, ondisplay }: Props = $props()

  function pickWindow(window: WindowName) {
    if (window !== display.window) ondisplay({ ...display, window })
  }

  function toggleLog() {
    ondisplay({ ...display, logFrequency: !display.logFrequency })
  }
</script>

<div style="display:flex;flex-direction:column;gap:6px">
  <div style="display:flex;flex-direction:column;gap:6px;padding:4px 0">
    <span style="font-size:13px;color:#c3cad4">Finestra FFT</span>
    <div style="display:flex;flex-wrap:wrap;gap:6px">
      {#each WINDOW_OPTIONS as option (option.window)}
        <button class="opt {option.window === display.window ? 'on' : ''}"
          aria-pressed={option.window === display.window} onclick={() => pickWindow(option.window)}>{option.label}</button>
      {/each}
    </div>
  </div>

  <div class="rw"><span class="lb">Asse delle frequenze logaritmico</span>
    <button class="tgl {display.logFrequency ? 'on' : ''}" role="switch" aria-checked={display.logFrequency}
      aria-label="Asse delle frequenze logaritmico" onclick={toggleLog}></button>
  </div>
</div>
