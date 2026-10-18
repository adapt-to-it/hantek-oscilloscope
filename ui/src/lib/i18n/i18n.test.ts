import { describe, it, expect } from 'vitest'
import { getLocale, interpolate, setLocale, translator } from './i18n.svelte'

const tr = translator({
  it: { ciao: 'Ciao {nome}' },
  en: { ciao: 'Hello {nome}' },
  fr: { ciao: 'Bonjour {nome}' },
  es: { ciao: 'Hola {nome}' },
})

describe('i18n', () => {
  it('parte in italiano nei test', () => {
    expect(getLocale()).toBe('it')
  })

  it('traduce nella lingua corrente', () => {
    expect(tr('ciao', { nome: 'Marco' })).toBe('Ciao Marco')
    setLocale('es')
    expect(tr('ciao', { nome: 'Marco' })).toBe('Hola Marco')
    setLocale('it')
  })

  it('lascia i segnaposto senza parametro', () => {
    expect(interpolate('{a} e {b}', { a: 1 })).toBe('1 e {b}')
  })
})
