<script lang="ts">
  // Display del tempo su canvas. Contratto: props sotto, eventi tramite callback.
  import { CH_COLORS, DIVISIONS_X, DIVISIONS_Y } from '../types'
  import type { DisplayFrame, ViewSettings } from '../types'

  interface Props {
    frame: DisplayFrame | null
    view: ViewSettings
    triggerLevel: number
    /** Colonne disponibili in pixel CSS: il genitore le passa a `set_columns`. */
    oncolumns?: (columns: number) => void
    /** Trascinamento dei marcatori 1, 2 (offsetDiv) e T (livello in volt). */
    onoffset?: (channel: 0 | 1, offsetDiv: number) => void
    ontriggerlevel?: (volts: number) => void
  }

  let { frame, view, triggerLevel, oncolumns, onoffset, ontriggerlevel }: Props = $props()

  const NO_DATA_NOTE = "Nessun frame dal dispositivo: controlla la connessione e avvia l'acquisizione."
  const PREFIXES: [number, string][] = [
    [1e9, 'G'],
    [1e6, 'M'],
    [1e3, 'k'],
    [1, ''],
    [1e-3, 'm'],
    [1e-6, 'µ'],
    [1e-9, 'n'],
  ]

  let plot: HTMLDivElement | undefined = $state()
  let canvas: HTMLCanvasElement | undefined = $state()
  // Dimensioni del riquadro del grafico in pixel CSS.
  let width = $state(0)
  let height = $state(0)
  let lastColumns = 0
  let raf = 0

  const divH = $derived(height / DIVISIONS_Y)
  // Chiave serializzata: fa ridisegnare il canvas anche a modifiche in profondità di view.
  const viewKey = $derived(JSON.stringify(view))
  // Posizione del trigger sull'asse x: t = 0 nel frame, null senza frame.
  const tmX = $derived(
    frame && frame.tEnd > frame.tStart ? (-frame.tStart / (frame.tEnd - frame.tStart)) * width : null,
  )
  const trigY = $derived(
    height / 2 - (triggerLevel / view.channels[0].voltsPerDiv + view.channels[0].offsetDiv) * divH,
  )

  function zeroY(ch: 0 | 1): number {
    return height / 2 - view.channels[ch].offsetDiv * divH
  }

  function fmtSI(value: number, unit: string): string {
    if (value === 0) return `0 ${unit}`
    const abs = Math.abs(value)
    const [scale, prefix] = PREFIXES.find(([s]) => abs >= s) ?? [1e-9, 'n']
    return `${Number((value / scale).toPrecision(3))} ${prefix}${unit}`
  }

  // Trascinamento: il valore parte dal valore iniziale e segue lo spostamento verticale del puntatore.
  type Drag =
    | { target: 'ch'; ch: 0 | 1; startY: number; start: number; last: number }
    | { target: 'trig'; startY: number; start: number; last: number }
  let drag: Drag | null = null

  function snap(v: number, step: number): number {
    return Number((Math.round(v / step) * step).toPrecision(12))
  }

  function startDrag(e: PointerEvent, d: Drag): void {
    drag = d
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture(e.pointerId)
  }

  function startChannelDrag(e: PointerEvent, ch: 0 | 1): void {
    const start = view.channels[ch].offsetDiv
    startDrag(e, { target: 'ch', ch, startY: e.clientY, start, last: start })
  }

  function startTriggerDrag(e: PointerEvent): void {
    startDrag(e, { target: 'trig', startY: e.clientY, start: triggerLevel, last: triggerLevel })
  }

  function moveDrag(e: PointerEvent): void {
    if (!drag) return
    const dy = e.clientY - drag.startY
    if (drag.target === 'ch') {
      const next = snap(drag.start - dy / divH, 0.1)
      if (next !== drag.last) {
        drag.last = next
        onoffset?.(drag.ch, next)
      }
    } else {
      const next = snap(drag.start - (dy / divH) * view.channels[0].voltsPerDiv, 0.01)
      if (next !== drag.last) {
        drag.last = next
        ontriggerlevel?.(next)
      }
    }
  }

  function endDrag(e: PointerEvent): void {
    drag = null
    const el = e.currentTarget as HTMLElement
    if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId)
  }

  // Colonne = larghezza CSS arrotondata; le dimensioni vengono aggiornate ad ogni resize.
  $effect(() => {
    if (!plot) return
    const ro = new ResizeObserver(([entry]) => {
      const { width: w, height: h } = entry.contentRect
      width = w
      height = h
      const cols = Math.round(w)
      if (cols >= 1 && cols !== lastColumns) {
        lastColumns = cols
        oncolumns?.(cols)
      }
    })
    ro.observe(plot)
    return () => ro.disconnect()
  })

  function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    ctx.lineWidth = 1
    ctx.strokeStyle = '#1b2128'
    ctx.beginPath()
    for (let i = 0; i <= DIVISIONS_X; i++) {
      const x = Math.round((i * w) / DIVISIONS_X) + 0.5
      ctx.moveTo(x, 0)
      ctx.lineTo(x, h)
    }
    for (let j = 0; j <= DIVISIONS_Y; j++) {
      const y = Math.round((j * h) / DIVISIONS_Y) + 0.5
      ctx.moveTo(0, y)
      ctx.lineTo(w, y)
    }
    ctx.stroke()

    // Assi centrali più marcati.
    ctx.strokeStyle = '#3a4350'
    ctx.beginPath()
    const cx = Math.round(w / 2) + 0.5
    const cy = Math.round(h / 2) + 0.5
    ctx.moveTo(cx, 0)
    ctx.lineTo(cx, h)
    ctx.moveTo(0, cy)
    ctx.lineTo(w, cy)
    ctx.stroke()
  }

  function drawTraces(ctx: CanvasRenderingContext2D, f: DisplayFrame, w: number, h: number): void {
    const colW = w / f.columns
    for (const ch of [0, 1] as const) {
      const trace = f.traces[ch]
      const channel = view.channels[ch]
      if (!trace || !channel.enabled) continue
      const yOf = (volts: number): number =>
        h / 2 - (volts / channel.voltsPerDiv + channel.offsetDiv) * (h / DIVISIONS_Y)
      const len = Math.min(f.columns, trace.min.length, trace.max.length)
      ctx.strokeStyle = CH_COLORS[ch]
      ctx.lineWidth = 1.25
      ctx.beginPath()
      let prev: { x: number; y: number } | null = null
      for (let c = 0; c < len; c++) {
        const lo = trace.min[c]
        const hi = trace.max[c]
        // NaN: colonna senza dati, interrompe la traccia.
        if (Number.isNaN(lo) || Number.isNaN(hi)) {
          prev = null
          continue
        }
        const x = (c + 0.5) * colW
        const mid = yOf((lo + hi) / 2)
        ctx.moveTo(x, yOf(lo))
        ctx.lineTo(x, yOf(hi))
        if (prev) {
          ctx.moveTo(prev.x, prev.y)
          ctx.lineTo(x, mid)
        }
        prev = { x, y: mid }
      }
      ctx.stroke()
    }
  }

  function draw(): void {
    raf = 0
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
    if (frame) drawTraces(ctx, frame, w, h)
  }

  // Ridisegno con requestAnimationFrame solo quando cambiano frame, view o dimensioni.
  $effect(() => {
    void frame
    void viewKey
    void width
    void height
    if (raf === 0) raf = requestAnimationFrame(draw)
    return () => {
      if (raf !== 0) cancelAnimationFrame(raf)
      raf = 0
    }
  })
</script>

<section class="card" style="flex:1;min-height:300px;display:flex;flex-direction:column;overflow:hidden;background:#06080a">
  <div bind:this={plot} style="position:relative;flex:1;min-height:240px;overflow:hidden">
    <canvas bind:this={canvas} style="position:absolute;left:0;top:0;width:100%;height:100%;display:block"></canvas>

    {#if frame}
      <div class="mono" role="slider" aria-label="Offset CH1" aria-valuenow={view.channels[0].offsetDiv} tabindex="-1" onpointerdown={(e) => startChannelDrag(e, 0)} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={endDrag} style="position:absolute;left:0;top:{zeroY(0)}px;transform:translateY(-50%);opacity:{view.channels[0].enabled ? 1 : 0.35};width:26px;height:20px;padding-right:6px;background:{CH_COLORS[0]};color:#1a1403;font-size:12px;font-weight:700;display:flex;align-items:center;justify-content:center;clip-path:polygon(0 0,70% 0,100% 50%,70% 100%,0 100%);cursor:ns-resize;touch-action:none" title="Trascina per regolare l'offset di CH1">1</div>
      <div class="mono" role="slider" aria-label="Offset CH2" aria-valuenow={view.channels[1].offsetDiv} tabindex="-1" onpointerdown={(e) => startChannelDrag(e, 1)} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={endDrag} style="position:absolute;left:0;top:{zeroY(1)}px;transform:translateY(-50%);opacity:{view.channels[1].enabled ? 1 : 0.35};width:26px;height:20px;padding-right:6px;background:{CH_COLORS[1]};color:#021820;font-size:12px;font-weight:700;display:flex;align-items:center;justify-content:center;clip-path:polygon(0 0,70% 0,100% 50%,70% 100%,0 100%);cursor:ns-resize;touch-action:none" title="Trascina per regolare l'offset di CH2">2</div>
      <div class="mono" role="slider" aria-label="Livello di trigger" aria-valuenow={triggerLevel} tabindex="-1" onpointerdown={startTriggerDrag} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={endDrag} style="position:absolute;right:0;top:{trigY}px;transform:translateY(-50%);height:22px;padding:0 8px 0 14px;background:{CH_COLORS[0]};color:#1a1403;font-size:12px;font-weight:700;display:flex;align-items:center;gap:6px;clip-path:polygon(0 50%,10px 0,100% 0,100% 100%,10px 100%);cursor:ns-resize;touch-action:none" title="Trascina per cambiare il livello di trigger">T <span style="font-weight:500">{fmtSI(triggerLevel, 'V')}</span></div>
      {#if tmX !== null}
        <div style="position:absolute;top:0;left:{tmX}px;transform:translateX(-50%);width:0;height:0;border-left:7px solid transparent;border-right:7px solid transparent;border-top:11px solid {CH_COLORS[0]}" title="Istante del trigger (t = 0)"></div>
      {/if}
      <div style="position:absolute;left:36px;bottom:10px;font-size:12px;color:#7f8996">Trascina 1, 2 e T per regolarli</div>
    {:else}
      <div style="position:absolute;left:0;top:0;right:0;bottom:0;background:#06080af2;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;text-align:center;padding:24px">
        <div style="font-size:18px;font-weight:600">In attesa di dati</div>
        <div style="font-size:13px;color:#9aa3af;max-width:340px;line-height:1.45;text-wrap:pretty">{NO_DATA_NOTE}</div>
      </div>
    {/if}
  </div>

  <div class="mono" style="display:flex;flex-wrap:wrap;align-items:center;gap:4px 16px;padding:7px 14px;border-top:1px solid #1b2128;background:#0a0c0f;font-size:12px">
    <span style="color:{CH_COLORS[0]}">CH1 {fmtSI(view.channels[0].voltsPerDiv, 'V')}/div</span>
    <span style="color:{CH_COLORS[1]}">CH2 {fmtSI(view.channels[1].voltsPerDiv, 'V')}/div</span>
    <span>{fmtSI(view.timePerDiv, 's')}/div</span>
    <span>fs {frame ? fmtSI(frame.sampleRate, 'Sa/s') : '—'}</span>
    <span style="color:{frame?.triggered ? '#ff9f43' : '#9aa3af'};font-weight:600">{frame ? (frame.triggered ? 'Triggerato' : 'Non triggerato') : '—'}</span>
  </div>
</section>
