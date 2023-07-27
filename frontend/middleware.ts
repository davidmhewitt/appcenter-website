import { type NextRequest, NextResponse } from 'next/server'
import acceptLanguage from 'accept-language'
import { fallbackLng, locales } from './app/i18n/settings'

acceptLanguage.languages(locales)

export const config = {
  // matcher: '/:lng*'
  matcher: ['/((?!api|_next/static|_next/image|assets|favicon.ico).*)'],
}

const cookieName = 'i18next'

export default function middleware(req: NextRequest) {
  let lng
  if (req.cookies.has(cookieName))
    lng = acceptLanguage.get(req.cookies.get(cookieName)?.value)
  if (!lng) lng = acceptLanguage.get(req.headers.get('Accept-Language'))
  if (!lng) lng = fallbackLng

  // Redirect if lng in path is not supported
  if (
    !locales.some((loc) => req.nextUrl.pathname.startsWith(`/${loc}`)) &&
    !req.nextUrl.pathname.startsWith('/_next')
  ) {
    return NextResponse.redirect(
      new URL(`/${lng}${req.nextUrl.pathname}`, req.url)
    )
  }

  const response = NextResponse.next()

  const refererString = req.headers.get('referer')
  if (refererString) {
    const refererUrl = new URL(refererString)
    const lngInReferer = locales.find((l) =>
      refererUrl.pathname.startsWith(`/${l}`)
    )
    if (lngInReferer) response.cookies.set(cookieName, lngInReferer)
    return response
  }

  return response
}
