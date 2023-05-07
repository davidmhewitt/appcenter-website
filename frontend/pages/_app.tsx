import '@/styles/globals.css'
import type { AppProps } from 'next/app'
import { appWithTranslation } from 'next-i18next'
import Layout from '../components/layout'
import UserContext from '../context/user'

if (process.env.NODE_ENV === 'development') {
  require('../mocks')
}

function App({ Component, pageProps }: AppProps) {
  return (
    <UserContext>
      <Layout>
        <Component {...pageProps} />
      </Layout>
    </UserContext>
  )
}

export default appWithTranslation(App)
