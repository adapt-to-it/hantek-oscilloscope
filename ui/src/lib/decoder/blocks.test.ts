import { describe, expect, it } from 'vitest'
import type { AnnotationView } from '../types'
import { stripBlocks } from './blocks'

/** Annotazione con i campi di default; i test sovrascrivono solo ciò che conta. */
function ann(start_s: number, end_s: number, extra: Partial<AnnotationView> = {}): AnnotationView {
  return { start_s, end_s, kind: 'data', text: '', value: null, error: false, ...extra }
}

describe('stripBlocks', () => {
  it('calcola posizione e larghezza in percentuale della finestra', () => {
    // Finestra 0..10 s: 1..2 s diventa left 10, width 10.
    const [b] = stripBlocks([ann(1, 2)], 0, 10, 1000)
    expect(b.left).toBeCloseTo(10)
    expect(b.width).toBeCloseTo(10)
  })

  it('ritaglia i blocchi che sporgono dalla finestra a 0..100', () => {
    const blocks = stripBlocks([ann(-2, 3), ann(8, 14)], 0, 10, 1000)
    expect(blocks).toHaveLength(2)
    expect(blocks[0].left).toBeCloseTo(0)
    expect(blocks[0].width).toBeCloseTo(30)
    expect(blocks[1].left).toBeCloseTo(80)
    expect(blocks[1].width).toBeCloseTo(20)
  })

  it('scarta i blocchi del tutto fuori dalla finestra', () => {
    const blocks = stripBlocks([ann(11, 12), ann(-5, -1), ann(-2, 0)], 0, 10, 1000)
    expect(blocks).toEqual([])
  })

  it('scarta i blocchi più stretti di 0,25 %', () => {
    // 0..0,02 s su finestra di 10 s: 0,2 %.
    expect(stripBlocks([ann(0, 0.02)], 0, 10, 1000)).toEqual([])
    // 0..0,03 s: 0,3 %, resta.
    expect(stripBlocks([ann(0, 0.03)], 0, 10, 1000)).toHaveLength(1)
  })

  it('dà a uno span nullo una larghezza minima di 0,6 %', () => {
    const [b] = stripBlocks([ann(5, 5)], 0, 10, 1000)
    expect(b.left).toBeCloseTo(50)
    expect(b.width).toBeCloseTo(0.6)
  })

  it('nasconde il testo se non ci sta nella larghezza, ma lo tiene nel title', () => {
    // 10 % di 400 px = 40 px; "ABCDEFGH" richiede 8 * 7,5 + 10 = 70 px.
    const [narrow] = stripBlocks([ann(0, 1, { text: 'ABCDEFGH' })], 0, 10, 400)
    expect(narrow.text).toBe('')
    expect(narrow.title).toBe('ABCDEFGH')

    // Stessa annotazione su 1000 px: 100 px, il testo entra.
    const [wide] = stripBlocks([ann(0, 1, { text: 'ABCDEFGH' })], 0, 10, 1000)
    expect(wide.text).toBe('ABCDEFGH')
  })

  it('usa i colori di error per kind "error" o error=true', () => {
    const [byFlag] = stripBlocks([ann(0, 1, { kind: 'data', error: true, text: 'x' })], 0, 10, 1000)
    expect(byFlag.bg).toBe('#e5392b')
    expect(byFlag.fg).toBe('#ffffff')
    expect(byFlag.title).toBe('Errore: x')

    const [byKind] = stripBlocks([ann(0, 1, { kind: 'error' })], 0, 10, 1000)
    expect(byKind.bg).toBe('#e5392b')
    expect(byKind.title).toBe('Errore')
  })

  it('usa i colori del kind e ripiega sul colore data per kind sconosciuti', () => {
    const [ack] = stripBlocks([ann(0, 1, { kind: 'ack' })], 0, 10, 1000)
    expect(ack.bg).toBe('#5fd68d')
    expect(ack.fg).toBe('#06210f')

    const [unknown] = stripBlocks([ann(0, 1, { kind: 'constructor' })], 0, 10, 1000)
    expect(unknown.bg).toBe('#6fc4f0')
    expect(unknown.fg).toBe('#06141c')
  })

  it('restituisce [] con finestra vuota o invertita', () => {
    expect(stripBlocks([ann(0, 1)], 5, 5, 1000)).toEqual([])
    expect(stripBlocks([ann(0, 1)], 5, 1, 1000)).toEqual([])
  })
})
