<script lang="ts">
  // Spiegazione unica a posizione fissa: con `active`, segue l'elemento più vicino con `data-help`
  // sotto il puntatore. Si apre sotto l'elemento, o sopra se sotto manca spazio, e resta nella finestra.
  import { HELP, type HelpId } from './texts'

  interface Props {
    active: boolean
  }

  let { active }: Props = $props()

  const WIDTH = 300
  const GAP = 6
  const MARGIN = 8
  /** Oltre questa altezza l'elemento è un'area, come il display: la spiegazione va al suo interno, in basso a destra. */
  const AREA_HEIGHT = 160

  let anchor = $state<HTMLElement | null>(null)
  let box = $state<HTMLDivElement | null>(null)
  let pos = $state({ left: 0, top: 0, ready: false })

  const help = $derived(anchor ? HELP[anchor.dataset.help as HelpId] : undefined)

  function onOver(event: PointerEvent) {
    const target = event.target instanceof Element ? event.target.closest<HTMLElement>('[data-help]') : null
    if (target !== anchor) {
      anchor = target && HELP[target.dataset.help as HelpId] ? target : null
      pos = { ...pos, ready: false }
    }
  }

  function onLeave(event: PointerEvent) {
    if (!event.relatedTarget) anchor = null
  }

  function place() {
    if (!anchor || !box) return
    const r = anchor.getBoundingClientRect()
    const w = box.offsetWidth
    const h = box.offsetHeight
    const clampX = (x: number) => Math.min(Math.max(MARGIN, x), window.innerWidth - w - MARGIN)
    if (r.height > AREA_HEIGHT) {
      pos = { left: clampX(r.right - w - 3 * MARGIN), top: Math.max(MARGIN, Math.min(r.bottom, window.innerHeight) - h - 4 * MARGIN), ready: true }
      return
    }
    const left = clampX(r.left)
    const below = r.bottom + GAP
    const top = below + h <= window.innerHeight - MARGIN ? below : Math.max(MARGIN, r.top - GAP - h)
    pos = { left, top, ready: true }
  }

  $effect(() => {
    if (!active) {
      anchor = null
      return
    }
    document.addEventListener('pointerover', onOver)
    document.addEventListener('pointerout', onLeave)
    window.addEventListener('scroll', place, true)
    return () => {
      document.removeEventListener('pointerover', onOver)
      document.removeEventListener('pointerout', onLeave)
      window.removeEventListener('scroll', place, true)
    }
  })

  $effect(() => {
    if (help && box && !pos.ready) place()
  })
</script>

{#if active && help}
  <div bind:this={box} class="tip" role="tooltip"
    style="display:block;position:fixed;left:{pos.left}px;top:{pos.top}px;right:auto;width:{WIDTH}px;max-width:calc(100vw - {2 * MARGIN}px);z-index:1000;visibility:{pos.ready ? 'visible' : 'hidden'}">
    <b>{help.title}</b>{help.text}
  </div>
{/if}
