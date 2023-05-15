import { InitOptions } from 'i18next'

export const fallbackLng = 'en'
export const locales = [fallbackLng, 'de', 'fr']
export const defaultNS = 'common'

export function getOptions(
  lng = fallbackLng,
  ns: string | string[] = defaultNS
): InitOptions {
  return {
    supportedLngs: locales,
    fallbackLng,
    lng,
    fallbackNS: defaultNS,
    defaultNS,
    ns,
    returnEmptyString: false,
    returnNull: false,
  }
}
