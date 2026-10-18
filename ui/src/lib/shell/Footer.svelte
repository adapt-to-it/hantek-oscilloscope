<script lang="ts">
  // Piè di pagina: motore, frame al secondo e campionamento nominale e misurato.
  import type { Status } from '../types'
  import { formatHz } from './format'
  import { tr } from '../i18n/msg/shell'

  interface Props {
    status: Status | null
    /** Frame al secondo calcolati dal genitore dal contatore `frames`. */
    fps: number
  }

  let { status, fps }: Props = $props()
</script>

<footer class="mono" style="display:flex;flex-wrap:wrap;justify-content:space-between;gap:4px 20px;padding:7px 16px;background:#0f1216;border-top:1px solid #20262e;font-size:12px;color:#9aa3af">
  <span data-help="engine">{tr('footerEngine', { sorgente: status?.source ?? '—', fps: Math.round(fps) })}</span>
  <span data-help="sampleRate">{tr('footerSampling', { nominale: status ? formatHz(status.sampleRateHz) : '—', misurato: status ? formatHz(status.sampleRateMeasuredHz) : '—' })}</span>
</footer>
