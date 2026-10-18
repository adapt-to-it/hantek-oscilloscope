import { describe, expect, it } from 'vitest'
import { shortcutFor, toggleCommand } from './keys'

const button = { tagName: 'BUTTON' }
const input = { tagName: 'INPUT' }

describe('toggleCommand', () => {
  it('ferma se il motore gira o è armato, altrimenti avvia', () => {
    expect(toggleCommand('running')).toBe('stop')
    expect(toggleCommand('armed')).toBe('stop')
    expect(toggleCommand('stopped')).toBe('run')
  })
})

describe('shortcutFor', () => {
  it('Spazio alterna run e stop', () => {
    expect(shortcutFor({ key: ' ', target: null }, 'stopped')).toBe('run')
    expect(shortcutFor({ key: ' ', target: button }, 'running')).toBe('stop')
  })

  it('S avvia il singolo, maiuscola o minuscola', () => {
    expect(shortcutFor({ key: 's', target: null }, 'stopped')).toBe('single')
    expect(shortcutFor({ key: 'S', target: button }, 'running')).toBe('single')
  })

  it('ignora i tasti con focus su un campo di input', () => {
    expect(shortcutFor({ key: ' ', target: input }, 'stopped')).toBeNull()
    expect(shortcutFor({ key: 's', target: { tagName: 'textarea' } }, 'stopped')).toBeNull()
    expect(shortcutFor({ key: ' ', target: { tagName: 'DIV', isContentEditable: true } }, 'stopped')).toBeNull()
  })

  it('ignora le combinazioni con modificatori e gli altri tasti', () => {
    expect(shortcutFor({ key: 's', ctrlKey: true, target: null }, 'stopped')).toBeNull()
    expect(shortcutFor({ key: 'a', target: null }, 'stopped')).toBeNull()
  })
})
