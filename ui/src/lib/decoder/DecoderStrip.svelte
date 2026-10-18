<script lang="ts">
  import { PROTOCOLS } from '../shell/controls/decoder'
  // Striscia dei frame decodificati sulla finestra del display. Solo presentazione: nessuna chiamata di rete.
  import type { DecodeView } from '../types'
  import { stripBlocks } from './blocks'

  interface Props {
    /** Risultato del decoder per la finestra corrente; null se non c'è decodifica. */
    view: DecodeView | null
    /** Errore del decoder da mostrare sopra la striscia; null se assente. */
    error: string | null
    /** Estremi della finestra in secondi. */
    tStart: number
    tEnd: number
    /** Etichetta del canale decodificato, es. "CH1". */
    channelLabel: string
  }

  let { view, error, tStart, tEnd, channelLabel }: Props = $props()

  // Larghezza in pixel CSS del contenitore dei blocchi.
  let strip: HTMLDivElement | undefined = $state()
  let width = $state(0)

  const blocks = $derived(view ? stripBlocks(view.annotations, tStart, tEnd, width) : [])

  $effect(() => {
    if (!strip) return
    const ro = new ResizeObserver(([entry]) => {
      width = entry.contentRect.width
    })
    ro.observe(strip)
    return () => ro.disconnect()
  })
</script>

<section class="card" style="padding:10px 12px;display:flex;flex-direction:column;gap:8px">
  <div style="display:flex;align-items:center;gap:10px;font-size:12px;color:#9aa3af">
    <span style="font-weight:700;color:#e8ebef">Decoder {view ? (PROTOCOLS.find((p) => p.value === view.protocol)?.label ?? view.protocol) : ''}</span><span>{channelLabel}</span>
  </div>

  <div bind:this={strip} style="position:relative;height:28px">
    {#each blocks as b, i (i)}
      <div class="blk" style="left:{b.left}%;width:{b.width}%;background:{b.bg};color:{b.fg}" title={b.title}>{b.text}</div>
    {/each}
    {#if blocks.length === 0}
      <div style="position:absolute;left:0;top:0;right:0;height:28px;border:1px dashed #333b47;border-radius:5px;display:flex;align-items:center;justify-content:center;font-size:12px;color:#8a94a1">Nessun frame decodificato nella finestra</div>
    {/if}
  </div>

  {#if error}
    <div role="alert" style="padding:6px 10px;border-radius:6px;background:#3a1714;color:#ffb3a6;font-size:13px">{error}</div>
  {/if}

  {#if view}
    <div class="mono" style="font-size:12px;color:#c3cad4">{view.summary}{view.truncated ? ' (troncato)' : ''}</div>
  {/if}
</section>
