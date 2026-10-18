<script lang="ts">
  import { formatVolts } from '../shell/format'
  // Vista XY su canvas: CH1 sull'asse orizzontale, CH2 su quello verticale. Nessuna chiamata di rete.
  import { CH_COLORS } from '../types'
  import type { ViewSettings, XyView } from '../types'
  import { XY_DIVISIONS, voltsToPx } from './xy'

  interface Props {
    data: XyView | null
    view: ViewSettings
  }

  let { data, view }: Props = $props()

  const NO_DATA_NOTE = 'La vista XY richiede entrambi i canali attivi.'
  const HINT =
    "CH1 sull'asse orizzontale, CH2 su quello verticale. Una ellisse indica uno sfasamento, una retta segnali in fase."

  let plot: HTMLDivElement | undefined = $state()
  let canvas: HTMLCanvasElement | undefined = $state()
  // Lato del quadrato centrato nell'area, in pixel CSS.
  let side = $state(0)
  let raf = 0

  const hasData = $derived(data !== null && data.x.length > 0)
  // Chiave serializzata: fa ridisegnare il canvas anche a modifiche in profondità di view.
  const viewKey = $derived(JSON.stringify(view))

  // Il quadrato è il lato minore dell'area: così resta quadrato e centrato a ogni resize.
  $effect(() => {
    if (!plot) return
    const ro = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect
      side = Math.floor(Math.min(width, height))
    })
    ro.observe(plot)
    return () => ro.disconnect()
  })

  function drawGrid(ctx: CanvasRenderingContext2D, s: number): void {
    const step = s / XY_DIVISIONS
    ctx.lineWidth = 1
    ctx.strokeStyle = '#1b2128'
    ctx.beginPath()
    for (let i = 0; i <= XY_DIVISIONS; i++) {
      const p = Math.round(i * step) + 0.5
      ctx.moveTo(p, 0)
      ctx.lineTo(p, s)
      ctx.moveTo(0, p)
      ctx.lineTo(s, p)
    }
    ctx.stroke()

    // Assi centrali più marcati.
    ctx.strokeStyle = '#3a4350'
    ctx.beginPath()
    const c = Math.round(s / 2) + 0.5
    ctx.moveTo(c, 0)
    ctx.lineTo(c, s)
    ctx.moveTo(0, c)
    ctx.lineTo(s, c)
    ctx.stroke()
  }

  function drawPoints(ctx: CanvasRenderingContext2D, xy: XyView, s: number): void {
    const [ch1, ch2] = view.channels
    const len = Math.min(xy.x.length, xy.y.length)
    ctx.strokeStyle = CH_COLORS[0]
    ctx.globalAlpha = 0.6
    ctx.lineWidth = 1.25
    ctx.beginPath()
    let pen = false
    for (let i = 0; i < len; i++) {
      const vx = xy.x[i]
      const vy = xy.y[i]
      // Campione non valido: interrompe la linea.
      if (!Number.isFinite(vx) || !Number.isFinite(vy)) {
        pen = false
        continue
      }
      // L'asse y del canvas cresce verso il basso: la tensione di CH2 si specchia rispetto al lato.
      const px = voltsToPx(vx, ch1, s)
      const py = s - voltsToPx(vy, ch2, s)
      if (pen) ctx.lineTo(px, py)
      else ctx.moveTo(px, py)
      pen = true
    }
    ctx.stroke()
    ctx.globalAlpha = 1
  }

  function draw(): void {
    raf = 0
    if (!canvas || side <= 0) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const dpr = window.devicePixelRatio || 1
    const s = side
    canvas.width = Math.round(s * dpr)
    canvas.height = Math.round(s * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    ctx.fillStyle = '#06080a'
    ctx.fillRect(0, 0, s, s)
    drawGrid(ctx, s)
    if (data && hasData) drawPoints(ctx, data, s)
  }

  // Ridisegno con requestAnimationFrame solo quando cambiano dati, view o dimensioni.
  $effect(() => {
    void data
    void viewKey
    void side
    if (raf === 0) raf = requestAnimationFrame(draw)
  })

  // Il ridisegno programmato si annulla solo alla chiusura della vista, come in Scope.
  $effect(() => () => {
    if (raf !== 0) cancelAnimationFrame(raf)
    raf = 0
  })
</script>

<section class="card" style="flex:1;min-height:300px;display:flex;flex-direction:column;overflow:hidden;background:#06080a">
  <div bind:this={plot} style="position:relative;flex:1;min-height:240px;overflow:hidden">
    <canvas bind:this={canvas} style="position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);width:{side}px;height:{side}px;display:block"></canvas>

    <div style="position:absolute;left:12px;bottom:10px;font-size:12px;color:#8a94a1;max-width:420px;line-height:1.4">{HINT}</div>

    {#if !hasData}
      <div style="position:absolute;left:0;top:0;right:0;bottom:0;background:#06080af2;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;text-align:center;padding:24px">
        <div style="font-size:13px;color:#9aa3af;max-width:340px;line-height:1.45;text-wrap:pretty">{NO_DATA_NOTE}</div>
      </div>
    {/if}
  </div>

  <div class="mono" style="display:flex;flex-wrap:wrap;align-items:center;gap:4px 16px;padding:7px 14px;border-top:1px solid #1b2128;background:#0a0c0f;font-size:12px">
    <span style="color:{CH_COLORS[0]}">CH1 {formatVolts(view.channels[0].voltsPerDiv)}/div</span>
    <span style="color:{CH_COLORS[1]}">CH2 {formatVolts(view.channels[1].voltsPerDiv)}/div</span>
    <span>XY</span>
  </div>
</section>
