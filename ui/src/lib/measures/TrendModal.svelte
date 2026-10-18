<script lang="ts">
  // Andamento di una misura: le ultime letture di CH1 e CH2 in un SVG 1000×400. Nessuna chiamata di rete.
  import type { ChannelMeasures } from '../types'
  import { CH_COLORS } from '../types'
  import { HISTORY_SIZE, type MeasureHistory } from './history'
  import { formatValue } from './rows'

  interface Props {
    history: MeasureHistory
    version: number
    field: keyof ChannelMeasures
    label: string
    onclose: () => void
  }

  let { history, version, field, label, onclose }: Props = $props()

  interface Series {
    times: number[]
    values: [(number | null)[], (number | null)[]]
  }

  const channels = [0, 1] as const

  // Ricalcolata a ogni push: `version` cresce a ogni frame.
  const live = $derived.by((): Series => {
    void version
    return {
      times: history.times(),
      values: [history.values(field, 0), history.values(field, 1)],
    }
  })

  // In pausa resta la copia congelata al momento del clic.
  let frozen = $state<Series | null>(null)
  const paused = $derived(frozen !== null)
  const shown = $derived(frozen ?? live)
  const count = $derived(shown.times.length)

  function togglePause() {
    frozen = paused
      ? null
      : { times: [...live.times], values: [[...live.values[0]], [...live.values[1]]] }
  }

  /** Estremi dell'asse Y con il 15 % di margine sopra e sotto; null se non ci sono valori. */
  const range = $derived.by(() => {
    let lo = Infinity
    let hi = -Infinity
    for (const series of shown.values) {
      for (const value of series) {
        if (value === null) continue
        lo = Math.min(lo, value)
        hi = Math.max(hi, value)
      }
    }
    if (lo === Infinity) return null
    const pad = (hi - lo) * 0.15 || Math.abs(hi) * 0.15 || 0.01
    return { lo: lo - pad, hi: hi + pad }
  })

  /** Percorso SVG; le letture mancanti spezzano la linea. Ultima lettura sul bordo destro. */
  function path(values: (number | null)[]): string {
    if (!range) return ''
    const { lo, hi } = range
    const n = values.length
    let d = ''
    let drawing = false
    values.forEach((value, i) => {
      if (value === null) {
        drawing = false
        return
      }
      const x = ((HISTORY_SIZE - n + i) / (HISTORY_SIZE - 1)) * 1000
      const y = ((hi - value) / (hi - lo)) * 400
      d += `${drawing ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`
      drawing = true
    })
    return d
  }

  const paths = $derived([path(shown.values[0]), path(shown.values[1])])

  const yLabels = $derived(
    range
      ? [range.hi, (range.hi + range.lo) / 2, range.lo].map((value) => formatValue(field, value))
      : ['—', '—', '—'],
  )

  /** "−N s" rispetto all'ultima lettura, calcolato dai tempi reali; "ora" per l'ultima. */
  function ago(t: number): string {
    const secs = Math.round((shown.times[count - 1] - t) / 1000)
    return secs <= 0 ? 'ora' : `−${secs} s`
  }

  const legend = $derived(
    channels.map((ch) => {
      const values = shown.values[ch]
      const last = values.length ? values[values.length - 1] : null
      return { ch, color: CH_COLORS[ch], text: last === null ? '—' : formatValue(field, last) }
    }),
  )

  const stats = $derived(
    channels.map((ch) => {
      const s = history.stats(shown.values[ch])
      return {
        ch,
        color: CH_COLORS[ch],
        min: s ? formatValue(field, s.min) : '—',
        max: s ? formatValue(field, s.max) : '—',
        avg: s ? formatValue(field, s.avg) : '—',
      }
    }),
  )

  // Frazione 0..1 della larghezza del grafico sotto il puntatore.
  let hover = $state<number | null>(null)

  function onMove(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    hover = rect.width > 0 ? Math.min(Math.max((event.clientX - rect.left) / rect.width, 0), 1) : null
  }

  const reading = $derived.by(() => {
    if (hover === null || count === 0) return null
    const slot = Math.round(hover * (HISTORY_SIZE - 1))
    const i = slot - (HISTORY_SIZE - count)
    if (i < 0 || i >= count) return null
    return {
      left: `${((slot / (HISTORY_SIZE - 1)) * 100).toFixed(2)}%`,
      flip: hover > 0.7,
      time: ago(shown.times[i]),
      values: channels.map((ch) => {
        const value = shown.values[ch][i]
        return { ch, color: CH_COLORS[ch], text: value === null ? '—' : formatValue(field, value) }
      }),
    }
  })
</script>

<svelte:window onkeydown={(event) => { if (event.key === 'Escape') onclose() }} />

<div
  role="presentation"
  onclick={(event) => { if (event.target === event.currentTarget) onclose() }}
  style="position:fixed;inset:0;z-index:60;background:#05070ad9;display:flex;align-items:center;justify-content:center;padding:20px"
>
  <div class="card" role="dialog" aria-modal="true" aria-label="Andamento della misura {label}" style="width:min(900px,100%);max-height:100%;overflow:auto;display:flex;flex-direction:column;gap:14px;padding:16px 18px;background:#0e1115;border-color:#3a4350;box-shadow:0 20px 60px #000c">
    <div style="display:flex;align-items:flex-start;gap:12px">
      <div style="flex:1;display:flex;flex-direction:column;gap:2px">
        <h2 style="font-size:18px;font-weight:700">Andamento di {label}</h2>
        <span style="font-size:13px;color:#9aa3af">Ultime {HISTORY_SIZE} letture · una misura per frame</span>
      </div>
      <button class="btn" onclick={onclose} aria-label="Chiudi" style="height:34px;padding:0 10px">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M2 2l10 10M12 2 2 12"></path></svg>
        <span class="kbd">Esc</span>
      </button>
    </div>

    <div style="display:flex;flex-wrap:wrap;align-items:center;gap:10px 14px">
      <button class="btn" aria-pressed={paused ? 'true' : 'false'} onclick={togglePause} style="min-width:120px">
        {#if paused}
          <svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true"><path d="M3 1.5v11l9-5.5z" fill="currentColor"></path></svg>
          Riprendi
        {:else}
          <svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true"><rect x="2.5" y="2" width="3.2" height="10" rx="1" fill="currentColor"></rect><rect x="8.3" y="2" width="3.2" height="10" rx="1" fill="currentColor"></rect></svg>
          Pausa
        {/if}
      </button>
      <div class="chip" role="status" style="height:32px">
        <span style="width:8px;height:8px;border-radius:50%;background:{paused ? '#8a94a1' : '#5fd68d'};display:inline-block"></span>
        {paused ? 'In pausa' : 'In diretta'}
      </div>
      <div style="flex:1"></div>
      {#each legend as item (item.ch)}
        <div class="mono" style="display:flex;align-items:center;gap:8px;font-size:13px">
          <span style="width:12px;height:3px;border-radius:2px;background:{item.color}"></span>
          <span style="color:{item.color};font-weight:600">CH{item.ch + 1}</span>
          <span>{item.text}</span>
        </div>
      {/each}
    </div>

    <div
      role="presentation"
      onmousemove={onMove}
      onmouseleave={() => (hover = null)}
      style="position:relative;height:300px;border:1px solid #232932;border-radius:10px;background:#06080a;overflow:hidden;cursor:crosshair"
    >
      <svg viewBox="0 0 1000 400" preserveAspectRatio="none" role="img" aria-label="Andamento nel tempo della misura {label}" style="position:absolute;left:0;top:0;width:100%;height:100%">
        <path d="M0 100H1000M0 200H1000M0 300H1000M200 0V400M400 0V400M600 0V400M800 0V400" stroke="#1b2128" stroke-width="1" fill="none" vector-effect="non-scaling-stroke"></path>
        <path d={paths[1]} stroke={CH_COLORS[1]} stroke-width="2" fill="none" stroke-linejoin="round" vector-effect="non-scaling-stroke"></path>
        <path d={paths[0]} stroke={CH_COLORS[0]} stroke-width="2" fill="none" stroke-linejoin="round" vector-effect="non-scaling-stroke"></path>
      </svg>
      {#if !range}
        <div class="mono" style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;font-size:13px;color:#6b7585">In attesa di misure</div>
      {/if}
      <div class="mono" style="position:absolute;left:8px;top:6px;padding:0 4px;background:#06080a;font-size:11px;color:#9aa3af">{yLabels[0]}</div>
      <div class="mono" style="position:absolute;left:8px;top:50%;transform:translateY(-50%);padding:0 4px;background:#06080a;font-size:11px;color:#9aa3af">{yLabels[1]}</div>
      <div class="mono" style="position:absolute;left:8px;bottom:6px;padding:0 4px;background:#06080a;font-size:11px;color:#9aa3af">{yLabels[2]}</div>
      {#if reading}
        <div style="position:absolute;top:0;bottom:0;left:{reading.left};width:1px;background:#cfd5dd99"></div>
        <div class="mono" style="position:absolute;top:10px;left:{reading.left};transform:{reading.flip ? 'translateX(calc(-100% - 10px))' : 'translateX(10px)'};padding:7px 10px;border-radius:8px;background:#0b0d10;border:1px solid #4a5568;font-size:12px;line-height:1.5;white-space:nowrap">
          <div style="color:#fff;font-weight:600">{reading.time}</div>
          {#each reading.values as v (v.ch)}
            <div style="color:{v.color}">CH{v.ch + 1} {v.text}</div>
          {/each}
        </div>
      {/if}
    </div>
    <div class="mono" style="display:flex;justify-content:space-between;font-size:11px;color:#9aa3af;margin-top:-8px;padding:0 2px">
      <span>{count > 0 ? ago(shown.times[0]) : '—'}</span><span>ora</span>
    </div>

    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(220px,100%),1fr));gap:10px">
      {#each stats as s (s.ch)}
        <div style="padding:10px 12px;border-radius:10px;background:#12151a;border:1px solid #232932;display:flex;flex-direction:column;gap:6px">
          <span style="font-weight:700;color:{s.color}">CH{s.ch + 1}</span>
          <div class="mono" style="display:flex;justify-content:space-between;font-size:12px;color:#c3cad4"><span>min {s.min}</span><span>media {s.avg}</span><span>max {s.max}</span></div>
        </div>
      {/each}
    </div>
  </div>
</div>
