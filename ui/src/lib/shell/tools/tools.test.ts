import { describe, expect, it } from 'vitest'
import { CUTOFF_STEPS, classHighlight, mathNote, stepCutoff } from './tools'

describe('CUTOFF_STEPS e stepCutoff', () => {
  it('elenca le frequenze di taglio da 100 Hz a 200 kHz', () => {
    expect(CUTOFF_STEPS).toEqual([100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000])
  })

  it('scorre alla frequenza successiva e precedente', () => {
    expect(stepCutoff(500, 1)).toBe(1000)
    expect(stepCutoff(500, -1)).toBe(200)
  })

  it('parte dalla frequenza più vicina se il valore non è in lista', () => {
    expect(stepCutoff(300, 1)).toBe(500)
    expect(stepCutoff(300, -1)).toBe(100)
  })

  it('si ferma agli estremi della lista', () => {
    expect(stepCutoff(200000, 1)).toBe(200000)
    expect(stepCutoff(100, -1)).toBe(100)
  })
})

describe('classHighlight', () => {
  it('evidenzia il chip corrispondente a ogni classe', () => {
    expect(classHighlight('clock')).toBe('Clock')
    expect(classHighlight('pwm')).toBe('PWM')
    expect(classHighlight('serial')).toBe('Seriale')
    expect(classHighlight('burst')).toBe('Burst')
    expect(classHighlight('analog')).toBe('Sinusoide')
    expect(classHighlight('noise')).toBe('Rumore')
  })

  it('non evidenzia nessun chip per dc e other', () => {
    expect(classHighlight('dc')).toBeNull()
    expect(classHighlight('other')).toBeNull()
  })

  it('non evidenzia nulla per una classe sconosciuta', () => {
    expect(classHighlight('constructor')).toBeNull()
  })
})

describe('mathNote', () => {
  it('con Off spiega come scegliere un’operazione', () => {
    expect(mathNote('off', 'time')).toBe('Scegli un’operazione tra i canali: la traccia M compare nella vista Tempo')
  })

  it('con il prodotto indica unità e scala', () => {
    expect(mathNote('mul', 'time')).toBe('M = CH1 × CH2 in V², sulla scala di CH1')
  })

  it('fuori dalla vista Tempo rimanda alla vista Tempo', () => {
    expect(mathNote('add', 'spectrum')).toBe('La traccia M si vede nella vista Tempo')
    expect(mathNote('integral', 'xy')).toBe('La traccia M si vede nella vista Tempo')
  })
})
