<script lang="ts">
  // Display dello spettro su canvas. Contratto: props sotto, nessuna chiamata di rete.
  import { CH_COLORS, DIVISIONS_X } from '../types'
  import type { SpectrumView } from '../types'
  import { formatHz } from '../shell/format'
  import { DB_MAX, DB_MIN, DB_STEP, decadeTicks, dbToY, formatDb, freqToX } from './scale'

  interface Props {
    /** Spettro della vista corrente; null senza dati dal dispositivo. */
    data: SpectrumView | null
    /** Asse x logaritmico con righe a ogni decade. */
    logFrequency: boolean
  }

  let { data, logFrequency }: Props = $props()

  const NO_DATA_NOTE = "Nessun frame dal dispositivo: controlla la connessione e avvia l'acquisizione."

  let plot: HTMLDivElement | undefined = $state()
  let canvas: HTMLCanvasElement | undefined = $state()
  // Dimensioni del riquadro del grafico in pixel CSS.
  let width = $state(0)
  let height = $state(0)

  // Il bin 0 (DC) non ha posto in scala log: la scala parte da max(binHz, 1).
  const minHz = $derived(data ? Math.max(data.binHz, 1) : 1)

  // Righe ed etichette dell'asse y: ogni DB_STEP da DB_MIN a DB_MAX.
  const dbTicks = $derived(
    Array.from({ length: Math.round((DB_MAX - DB_MIN) / DB_STEP) + 1 }, (_, i) => {
      const db = DB_MIN + i * DB_STEP
      return { db, top: dbToY(db, height), text: formatDb(db, 0, false) }
    }),
  )

  // Frequenze delle righe verticali: decadi in log, divisioni uniformi in lineare.
  function gridFreqs(s: SpectrumView): number[] {
    if (logFrequency) return decadeTicks(minHz, s.nyquistHz)
    return Array.from({ length: DIVISIONS_X + 1 }, (_, i) => (i / DIVISIONS_X) * s.nyquistHz)
  }

  const fTicks = $derived.by(() => {
    if (!data) return []
    const nyq = data.nyquistHz
    return gridFreqs(data).map((f) => ({
      left: freqToX(f, width, nyq, logFrequency, minHz),
      text: formatHz(f),
    }))
  })

  // Riquadro dei picchi: per ogni canale presente, i primi 3 picchi e la THD.
  const peakGroups = $derived(
    data
      ? data.channels.flatMap((ch, i) => {
          if (!ch) return []
          return [
            {
              name: `CH${i + 1}`,
              color: CH_COLORS[i],
              peaks: ch.peaks.slice(0, 3).map((p) => `${formatHz(p.frequency)} · ${formatDb(p.dbv)}`),
              thd: ch.thdPct === null ? null : `THD ${ch.thdPct.toFixed(2)} %`,
            },
          ]
        })
      : [],
  )

  // Dimensioni in pixel CSS: ad ogni resize il canvas viene ridisegnato con il dpr corrente.
  $effect(() => {
    if (!plot) return
    const ro = new ResizeObserver(([entry]) => {
      width = entry.contentRect.width
      height = entry.contentRect.height
    })
    ro.observe(plot)
    return () => ro.disconnect()
  })

  function clampDb(db: number): number {
    return Math.min(DB_MAX, Math.max(DB_MIN, db))
  }

  function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    ctx.lineWidth = 1
    ctx.strokeStyle = '#1b2128'
    ctx.beginPath()
    for (let i = 0; i <= Math.round((DB_MAX - DB_MIN) / DB_STEP); i++) {
      const y = Math.round(dbToY(DB_MIN + i * DB_STEP, h)) + 0.5
      ctx.moveTo(0, y)
      ctx.lineTo(w, y)
    }
    // Le colonne seguono le stesse frequenze delle etichette; senza dati, divisioni uniformi.
    const nyq = data?.nyquistHz ?? 0
    const xs = data
      ? gridFreqs(data).map((f) => freqToX(f, w, nyq, logFrequency, minHz))
      : Array.from({ length: DIVISIONS_X + 1 }, (_, i) => (i * w) / DIVISIONS_X)
    for (const x of xs) {
      const px = Math.round(x) + 0.5
      ctx.moveTo(px, 0)
      ctx.lineTo(px, h)
    }
    ctx.stroke()

    // Linea dello 0 dBV più marcata.
    ctx.strokeStyle = '#3a4350'
    ctx.beginPath()
    const zy = Math.round(dbToY(0, h)) + 0.5
    ctx.moveTo(0, zy)
    ctx.lineTo(w, zy)
    ctx.stroke()
  }

  function drawTraces(ctx: CanvasRenderingContext2D, s: SpectrumView, w: number, h: number): void {
    for (const ch of [0, 1] as const) {
      const spec = s.channels[ch]
      if (!spec) continue
      ctx.strokeStyle = CH_COLORS[ch]
      ctx.lineWidth = 1.25
      ctx.beginPath()
      let started = false
      spec.dbv.forEach((db, bin) => {
        // In log il bin 0 cade a f = 0 e non ha posizione: si salta.
        if (logFrequency && bin === 0) return
        // Valore non finito: interrompe la traccia.
        if (!Number.isFinite(db)) {
          started = false
          return
        }
        const x = freqToX(bin * s.binHz, w, s.nyquistHz, logFrequency, minHz)
        const y = dbToY(clampDb(db), h)
        if (started) {
          ctx.lineTo(x, y)
        } else {
          ctx.moveTo(x, y)
          started = true
        }
      })
      ctx.stroke()
    }
  }

  function draw(): void {
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const dpr = window.devicePixelRatio || 1
    const w = width
    const h = height
    canvas.width = Math.round(w * dpr)
    canvas.height = Math.round(h * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.fillStyle = '#06080a'
    ctx.fillRect(0, 0, w, h)
    drawGrid(ctx, w, h)
    if (data) drawTraces(ctx, data, w, h)
  }

  // Ridisegno solo quando cambiano dati, scala o dimensioni.
  $effect(() => {
    void data
    void logFrequency
    void width
    void height
    draw()
  })
</script>

<section class="card" style="flex:1;min-height:300px;display:flex;flex-direction:column;overflow:hidden;background:#06080a">
  <!-- Il grafico è rientrato dai bordi: a sinistra le etichette dB, sotto quelle delle frequenze. -->
  <div style="position:relative;flex:1;min-height:240px">
  <div bind:this={plot} style="position:absolute;top:14px;right:34px;bottom:26px;left:48px">
    <canvas bind:this={canvas} style="position:absolute;left:0;top:0;width:100%;height:100%;display:block"></canvas>

    {#each dbTicks as d (d.db)}
      <div class="mono" style="position:absolute;right:calc(100% + 6px);top:{d.top}px;transform:translateY(-50%);font-size:11px;color:#9aa3af;white-space:nowrap">{d.text}</div>
    {/each}

    {#each fTicks as t, i (i)}
      <div class="mono" style="position:absolute;top:calc(100% + 6px);left:{t.left}px;transform:translateX(-50%);font-size:11px;color:#9aa3af;white-space:nowrap">{t.text}</div>
    {/each}

    {#if data}
      <div style="position:absolute;right:12px;top:12px;min-width:210px;padding:10px 12px;border-radius:10px;background:#0b0d10ee;border:1px solid #333b47;display:flex;flex-direction:column;gap:5px">
        <div style="font-size:12px;font-weight:700;color:#c3cad4">Picchi</div>
        {#each peakGroups as g (g.name)}
          <div class="mono" style="font-size:12px;font-weight:700;color:{g.color}">{g.name}</div>
          {#each g.peaks as p, n (n)}
            <div class="mono" style="display:flex;align-items:center;gap:8px;font-size:12px;color:#dde2e8"><span style="width:18px;height:18px;border-radius:50%;border:2px solid {g.color};color:{g.color};font-size:10px;font-weight:700;display:inline-flex;align-items:center;justify-content:center">{n + 1}</span><span>{p}</span></div>
          {/each}
          {#if g.thd}
            <div class="mono" style="font-size:12px;color:#9aa3af">{g.thd}</div>
          {/if}
        {/each}
      </div>
    {:else}
      <div style="position:absolute;left:0;top:0;right:0;bottom:0;background:#06080af2;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;text-align:center;padding:24px">
        <div style="font-size:18px;font-weight:600">In attesa di dati</div>
        <div style="font-size:13px;color:#9aa3af;max-width:340px;line-height:1.45;text-wrap:pretty">{NO_DATA_NOTE}</div>
      </div>
    {/if}
  </div>
  </div>

  <div class="mono" style="display:flex;flex-wrap:wrap;align-items:center;gap:4px 16px;padding:7px 14px;border-top:1px solid #1b2128;background:#0a0c0f;font-size:12px">
    <span>Δf {data ? formatHz(data.binHz) : '—'}</span>
    <span>Nyquist {data ? formatHz(data.nyquistHz) : '—'}</span>
  </div>
</section>
