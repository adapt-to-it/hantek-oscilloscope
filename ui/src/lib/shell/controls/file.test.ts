import { describe, expect, it } from 'vitest'
import type { RecordingView } from '../../types'
import { LIMITS, defaultFileName, fileName, formatBytes, recordingLabel } from './file'

function view(patch: Partial<RecordingView>): RecordingView {
  return { path: 'h6022.csv', format: 'csv', frames: 0, running: false, error: null, ...patch }
}

describe('defaultFileName', () => {
  it('compone prefisso, data e ora locali con zeri iniziali', () => {
    // Mese 9 è ottobre (indice da zero).
    const d = new Date(2026, 9, 8, 7, 5, 9)
    expect(defaultFileName('h6022', 'csv', d)).toBe('h6022-20261008-070509.csv')
  })
})

describe('fileName', () => {
  it('restituisce la parte dopo l\'ultimo slash', () => {
    expect(fileName('/home/u/h6022-recordings/h6022.csv')).toBe('h6022.csv')
    expect(fileName('h6022.csv')).toBe('h6022.csv')
  })
})

describe('formatBytes', () => {
  it('usa i byte sotto i mille', () => {
    expect(formatBytes(512)).toBe('512 B')
  })

  it('usa i kB in base 1000 con una cifra e la virgola', () => {
    expect(formatBytes(12300)).toBe('12,3 kB')
    expect(formatBytes(1000)).toBe('1,0 kB')
  })

  it('usa i MB oltre il milione', () => {
    expect(formatBytes(4500000)).toBe('4,5 MB')
  })
})

describe('recordingLabel', () => {
  it('mostra Pronto senza registrazione', () => {
    expect(recordingLabel(null)).toBe('Pronto')
  })

  it('mostra l\'errore quando la registrazione è fallita', () => {
    expect(recordingLabel(view({ error: 'disco pieno', running: false }))).toBe('Errore: disco pieno')
  })

  it('mostra REC e i frame mentre la registrazione è in corso', () => {
    expect(recordingLabel(view({ running: true, frames: 1200 }))).toBe('REC · 1200 frame')
  })

  it('mostra Finita con i frame quando la registrazione si è chiusa', () => {
    expect(recordingLabel(view({ running: false, frames: 3000 }))).toBe('Finita · 3000 frame')
  })
})

describe('LIMITS', () => {
  it('elenca manuale, 10 s e 60 s', () => {
    expect(LIMITS.map((l) => l.seconds)).toEqual([null, 10, 60])
  })
})
