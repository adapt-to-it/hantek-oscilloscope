<script lang="ts">
  // Display del tempo su canvas. Contratto: props sotto, eventi tramite callback.
  import { CH_COLORS, DIVISIONS_X, DIVISIONS_Y } from '../types'
  import type { ChannelTrace, CursorPositions, DisplayFrame, DisplayOptions, ViewSettings } from '../types'
  import Tip from '../help/Tip.svelte'
  import { cursorReadout } from './cursors'

  interface Props {
    frame: DisplayFrame | null
    view: ViewSettings
    display: DisplayOptions
    cursors: CursorPositions
    triggerLevel: number
    /** Colonne disponibili in pixel CSS: il genitore le passa a `set_columns`. */
    oncolumns?: (columns: number) => void
    /** Trascinamento dei marcatori 1, 2 (offsetDiv) e T (livello in volt). */
    onoffset?: (channel: 0 | 1, offsetDiv: number) => void
    ontriggerlevel?: (volts: number) => void
    /** Trascinamento dei cursori: restituisce le posizioni complete. */
    oncursors?: (c: CursorPositions) => void
  }

  let { frame, view, display, cursors, triggerLevel, oncolumns, onoffset, ontriggerlevel, oncursors }: Props = $props()

  const NO_DATA_NOTE = "Nessun frame dal dispositivo: controlla la connessione e avvia l'acquisizione."
  const MATH_COLOR = '#b794f6'
  const CURSOR_COLOR = '#cfd5dd'
  // Persistenza: ultime 8 schermate; la più vecchia in trasparenza 0.05, la più recente sotto la corrente 0.4.
  const PERSIST_DEPTH = 8
  const GHOST_ALPHA_NEAR = 0.4
  const GHOST_ALPHA_FAR = 0.05
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
  // Frame precedenti per la persistenza, dal più vecchio al più recente. Non reattivo: lo legge solo draw.
  let history: DisplayFrame[] = []

  const divH = $derived(height / DIVISIONS_Y)
  // Chiave serializzata: fa ridisegnare il canvas anche a modifiche in profondità di view.
  const viewKey = $derived(JSON.stringify(view))
  const displayKey = $derived(JSON.stringify(display))
  const cursorsKey = $derived(JSON.stringify(cursors))
  // Chiave delle scale: cambiarle svuota la persistenza.
  const scaleKey = $derived(JSON.stringify([view.timePerDiv, view.channels[0].voltsPerDiv, view.channels[1].voltsPerDiv]))
  // Posizione del trigger sull'asse x: t = 0 nel frame, null senza frame.
  const tmX = $derived(
    frame && frame.tEnd > frame.tStart ? (-frame.tStart / (frame.tEnd - frame.tStart)) * width : null,
  )
  const trigY = $derived(
    height / 2 - (triggerLevel / view.channels[0].voltsPerDiv + view.channels[0].offsetDiv) * divH,
  )

  // Intervallo dell'asse x in secondi relativi al trigger: dal frame, altrimenti dalla base dei tempi.
  const timeRange = $derived(
    frame && frame.tEnd > frame.tStart
      ? { start: frame.tStart, span: frame.tEnd - frame.tStart }
      : { start: -(DIVISIONS_X / 2) * view.timePerDiv, span: DIVISIONS_X * view.timePerDiv },
  )
  const showTime = $derived(frame !== null && (display.cursors === 'time' || display.cursors === 'both'))
  const showVolt = $derived(frame !== null && (display.cursors === 'voltage' || display.cursors === 'both'))
  const readout = $derived(cursorReadout(display.cursors, cursors))

  function zeroY(ch: 0 | 1): number {
    return height / 2 - view.channels[ch].offsetDiv * divH
  }

  function timeToX(t: number): number {
    return ((t - timeRange.start) / timeRange.span) * width
  }

  function xToTime(x: number): number {
    return timeRange.start + (x / width) * timeRange.span
  }

  function voltToY(v: number, ch: 0 | 1): number {
    return height / 2 - (v / view.channels[ch].voltsPerDiv + view.channels[ch].offsetDiv) * divH
  }

  function yToVolt(y: number, ch: 0 | 1): number {
    return ((height / 2 - y) / divH - view.channels[ch].offsetDiv) * view.channels[ch].voltsPerDiv
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

  // Trascinamento dei cursori: la posizione segue il puntatore in assoluto.
  type CursorKey = 't1' | 't2' | 'v1' | 'v2'
  let cursorDrag: CursorKey | null = null

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

  function startCursorDrag(e: PointerEvent, key: CursorKey): void {
    cursorDrag = key
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture(e.pointerId)
  }

  function moveCursorDrag(e: PointerEvent): void {
    if (!cursorDrag || !plot) return
    const box = plot.getBoundingClientRect()
    const x = e.clientX - box.left
    const y = e.clientY - box.top
    const value = cursorDrag === 't1' || cursorDrag === 't2' ? xToTime(x) : yToVolt(y, display.cursorChannel)
    oncursors?.({ ...cursors, [cursorDrag]: value })
  }

  function endCursorDrag(e: PointerEvent): void {
    cursorDrag = null
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

  // Persistenza: ogni frame nuovo entra nello storico; senza persistenza lo storico resta vuoto.
  $effect(() => {
    if (!frame || !display.persistence) return
    history.push(frame)
    if (history.length > PERSIST_DEPTH) history.splice(0, history.length - PERSIST_DEPTH)
  })

  // Lo storico si svuota quando cambiano time/div o V/div e quando la persistenza si spegne.
  $effect(() => {
    void scaleKey
    void display.persistence
    history = []
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

  function drawTraces(ctx: CanvasRenderingContext2D, f: DisplayFrame, w: number, h: number, alpha: number): void {
    const colW = w / f.columns
    // M usa la scala e l'offset di CH1 e non dipende dall'abilitazione dei canali.
    const jobs: { trace: ChannelTrace | null; ch: 0 | 1; color: string; width: number; visible: boolean }[] = [
      { trace: f.traces[0], ch: 0, color: CH_COLORS[0], width: 1.25, visible: view.channels[0].enabled },
      { trace: f.traces[1], ch: 1, color: CH_COLORS[1], width: 1.25, visible: view.channels[1].enabled },
      { trace: f.math, ch: 0, color: MATH_COLOR, width: 2, visible: true },
    ]
    ctx.globalAlpha = alpha
    for (const job of jobs) {
      const trace = job.trace
      const channel = view.channels[job.ch]
      if (!trace || !job.visible) continue
      const yOf = (volts: number): number =>
        h / 2 - (volts / channel.voltsPerDiv + channel.offsetDiv) * (h / DIVISIONS_Y)
      const len = Math.min(f.columns, trace.min.length, trace.max.length)
      ctx.strokeStyle = job.color
      ctx.lineWidth = job.width
      ctx.lineJoin = 'round'
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
    ctx.globalAlpha = 1
  }

  // Alfa della schermata passata k-esima (k = 1 la più recente): da 0.4 a 0.05.
  function ghostAlpha(k: number): number {
    const steps = PERSIST_DEPTH - 2
    return GHOST_ALPHA_NEAR + ((GHOST_ALPHA_FAR - GHOST_ALPHA_NEAR) * (k - 1)) / steps
  }

  function drawCursors(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    if (!showTime && !showVolt) return
    ctx.setLineDash([4, 5])
    ctx.strokeStyle = CURSOR_COLOR
    ctx.lineWidth = 1
    ctx.globalAlpha = 0.85
    ctx.beginPath()
    if (showTime) {
      for (const x of [timeToX(cursors.t1), timeToX(cursors.t2)]) {
        const px = Math.round(x) + 0.5
        ctx.moveTo(px, 0)
        ctx.lineTo(px, h)
      }
    }
    if (showVolt) {
      for (const v of [cursors.v1, cursors.v2]) {
        const py = Math.round(voltToY(v, display.cursorChannel)) + 0.5
        ctx.moveTo(0, py)
        ctx.lineTo(w, py)
      }
    }
    ctx.stroke()
    ctx.setLineDash([])
    ctx.globalAlpha = 1
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
    if (frame) {
      if (display.persistence) {
        // Le schermate passate sotto la corrente, dalla più vecchia alla più recente.
        const ghosts = history.filter((g) => g !== frame)
        ghosts.forEach((g, i) => drawTraces(ctx, g, w, h, ghostAlpha(ghosts.length - i)))
      }
      drawTraces(ctx, frame, w, h, 1)
      drawCursors(ctx, w, h)
    }
  }

  // Ridisegno con requestAnimationFrame solo quando cambiano frame, view, display, cursori o dimensioni.
  // Un ridisegno già programmato non viene annullato: legge lo stato più recente quando parte.
  // Annullarlo a ogni aggiornamento lo rinvierebbe all'infinito con frame più frequenti del refresh.
  $effect(() => {
    void frame
    void viewKey
    void displayKey
    void cursorsKey
    void width
    void height
    if (raf === 0) raf = requestAnimationFrame(draw)
  })

  $effect(() => () => {
    if (raf !== 0) cancelAnimationFrame(raf)
    raf = 0
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

      {#if showTime}
        <div class="mono" role="slider" aria-label="Cursore t1" aria-valuenow={cursors.t1} tabindex="-1" onpointerdown={(e) => startCursorDrag(e, 't1')} onpointermove={moveCursorDrag} onpointerup={endCursorDrag} onpointercancel={endCursorDrag} style="position:absolute;top:0;bottom:0;left:{timeToX(cursors.t1) - 5}px;width:10px;cursor:ew-resize;touch-action:none"></div>
        <div class="mono" role="slider" aria-label="Cursore t2" aria-valuenow={cursors.t2} tabindex="-1" onpointerdown={(e) => startCursorDrag(e, 't2')} onpointermove={moveCursorDrag} onpointerup={endCursorDrag} onpointercancel={endCursorDrag} style="position:absolute;top:0;bottom:0;left:{timeToX(cursors.t2) - 5}px;width:10px;cursor:ew-resize;touch-action:none"></div>
      {/if}
      {#if showVolt}
        <div class="mono" role="slider" aria-label="Cursore V1" aria-valuenow={cursors.v1} tabindex="-1" onpointerdown={(e) => startCursorDrag(e, 'v1')} onpointermove={moveCursorDrag} onpointerup={endCursorDrag} onpointercancel={endCursorDrag} style="position:absolute;left:0;right:0;top:{voltToY(cursors.v1, display.cursorChannel) - 5}px;height:10px;cursor:ns-resize;touch-action:none"></div>
        <div class="mono" role="slider" aria-label="Cursore V2" aria-valuenow={cursors.v2} tabindex="-1" onpointerdown={(e) => startCursorDrag(e, 'v2')} onpointermove={moveCursorDrag} onpointerup={endCursorDrag} onpointercancel={endCursorDrag} style="position:absolute;left:0;right:0;top:{voltToY(cursors.v2, display.cursorChannel) - 5}px;height:10px;cursor:ns-resize;touch-action:none"></div>
      {/if}
      {#if readout.length > 0}
        <div style="position:absolute;left:12px;top:14px;min-width:150px">
          <Tip id="cursors">
            <div class="mono" style="padding:8px 10px;border-radius:8px;background:#0b0d10ee;border:1px solid #333b47;font-size:12px;line-height:1.55;color:#dde2e8">
              {#each readout as row (row.k)}
                <div style="display:flex;justify-content:space-between;gap:16px"><span style="color:#9aa3af">{row.k}</span><span>{row.v}</span></div>
              {/each}
            </div>
          </Tip>
        </div>
      {/if}

      <div style="position:absolute;left:36px;bottom:10px;font-size:12px;color:#7f8996">{display.cursors === 'off' ? 'Trascina 1, 2 e T per regolarli' : 'Trascina 1, 2, T e i cursori per regolarli'}</div>
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
