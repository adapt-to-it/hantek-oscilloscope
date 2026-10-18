<script lang="ts">
  // Avvolge un controllo: con la classe `exp` su un antenato, il passaggio del puntatore mostra la spiegazione.
  import type { Snippet } from 'svelte'
  import { HELP, type HelpId } from './texts'

  interface Props {
    id: HelpId
    /** Mostra la spiegazione sopra il controllo invece che sotto. */
    up?: boolean
    children: Snippet
  }

  let { id, up = false, children }: Props = $props()
  const help = $derived(HELP[id])
</script>

<div class="tipw">
  {@render children()}
  {#if help}<div class="tip {up ? 'up' : ''}" role="tooltip"><b>{help.title}</b>{help.text}</div>{/if}
</div>
