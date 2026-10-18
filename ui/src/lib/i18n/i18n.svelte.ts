// Lingua dell'interfaccia: stato reattivo, scelta iniziale, persistenza e traduttori per area.
// L'italiano è la lingua di riferimento: ogni area dichiara le chiavi in italiano e le altre lingue le coprono tutte.

export const LOCALES = ['it', 'en', 'fr', 'es'] as const
export type Locale = (typeof LOCALES)[number]

/** Nome di ogni lingua nella lingua stessa, per il selettore. */
export const LOCALE_NAMES: Record<Locale, string> = {
  it: 'Italiano',
  en: 'English',
  fr: 'Français',
  es: 'Español',
}

const STORAGE_KEY = 'h6022.locale'

function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && (LOCALES as readonly string[]).includes(value)
}

/** Lingua salvata, altrimenti quella del sistema, altrimenti italiano. */
function initialLocale(): Locale {
  try {
    const saved = globalThis.localStorage?.getItem(STORAGE_KEY)
    if (isLocale(saved)) return saved
  } catch {
    // localStorage non disponibile: si passa alla lingua del sistema.
  }
  const system = globalThis.navigator?.language?.slice(0, 2).toLowerCase()
  return isLocale(system) ? system : 'it'
}

const state = $state({ locale: initialLocale() })

/** Lingua corrente. Letta dentro markup o $derived, aggiorna la vista al cambio di lingua. */
export function getLocale(): Locale {
  return state.locale
}

export function setLocale(locale: Locale): void {
  state.locale = locale
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, locale)
  } catch {
    // Persistenza non disponibile: la scelta vale fino alla chiusura.
  }
  if (globalThis.document) document.documentElement.lang = locale
}

/** Testi di un'area: chiavi in italiano, le altre lingue con le stesse chiavi. */
export type Messages<K extends string> = { it: Record<K, string> } & Record<Exclude<Locale, 'it'>, Record<NoInfer<K>, string>>

export type Params = Record<string, string | number>

/** Sostituisce i segnaposto `{nome}` con i parametri. Un segnaposto senza parametro resta com'è. */
export function interpolate(text: string, params?: Params): string {
  if (!params) return text
  return text.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match))
}

/** Traduttore di un'area: `tr('chiave', { n: 3 })` restituisce il testo nella lingua corrente. */
export function translator<K extends string>(messages: Messages<K>): (key: K, params?: Params) => string {
  return (key, params) => interpolate(messages[state.locale][key] ?? messages.it[key], params)
}
