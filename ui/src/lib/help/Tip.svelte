<script lang="ts">
  // Avvolge un blocco e gli assegna una spiegazione: HelpLayer la mostra al passaggio del puntatore.
  import type { Snippet } from 'svelte'
  import type { HelpId } from './texts'

  interface Props {
    id: HelpId
    children: Snippet
  }

  let { id, children }: Props = $props()
</script>

<div data-help={id}>
  {@render children()}
</div>
