import { dir } from 'i18next'
import { locales } from '../i18n/settings'
import '../globals.css'
import Header from '../../components/Header'

export async function generateStaticParams() {
  return locales.map((lang) => ({ lang }))
}

export default function RootLayout({
  children,
  params: {
    lang
  }
}: { children: React.ReactNode, params: { lang: string } }) {
  return (
    <html lang={lang} dir={dir(lang)}>
      <head />
      <body>
        <Header/>
        {children}
      </body>
    </html>
  )
}