<script lang="ts">
  // Piè di pagina: motore, frame al secondo e campionamento nominale e misurato.
  import type { Status } from '../types'
  import { formatHz } from './format'

  interface Props {
    status: Status | null
    /** Frame al secondo calcolati dal genitore dal contatore `frames`. */
    fps: number
  }

  let { status, fps }: Props = $props()
</script>

<footer class="mono" style="display:flex;flex-wrap:wrap;justify-content:space-between;gap:4px 20px;padding:7px 16px;background:#0f1216;border-top:1px solid #20262e;font-size:12px;color:#9aa3af">
  <span>Motore: {status?.source ?? '—'} · {Math.round(fps)} frame/s</span>
  <span>Campionamento {status ? formatHz(status.sampleRateHz) : '—'} · misurato {status ? formatHz(status.sampleRateMeasuredHz) : '—'}</span>
</footer>
