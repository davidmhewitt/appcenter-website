import { Inter } from 'next/font/google'
import Image from 'next/image'

import { useRouter } from 'next/router';

const inter = Inter({ subsets: ['latin'] })

import { useTranslation } from 'next-i18next'
import { getStaticPaths, makeStaticProps } from '../../../lib/getStatic'
import useSWR from 'swr';
export const fetcher = (url: string) => fetch(url).then((res) => res.json());

interface Icon {
  path: string,
  width: Number,
  height: Number,
}


interface Component {
  id: string,
  name: Object,
  summary: Object,
  icons: Icon[]
}

export default function Home() {
  const { t } = useTranslation('common');
  const router = useRouter();

  const appdata = useSWR<Component>('/static/apps/' + router.query.id + '.json', fetcher).data;

  return (
    <main
      className={`p-24 ${inter.className}`}
    >
      <h1>{appdata ? Object.entries(appdata?.name)[Object.keys(appdata?.name).indexOf("C")][1] : null}</h1>
    </main>
  )
}

const getStaticProps = makeStaticProps(['common'])
export { getStaticPaths, getStaticProps }
