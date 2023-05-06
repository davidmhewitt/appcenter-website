import { Inter } from 'next/font/google'
import Image from 'next/image'

const inter = Inter({ subsets: ['latin'] })

import { useTranslation } from 'next-i18next'
import { getStaticPaths, makeStaticProps } from '../../lib/getStatic'
import useSWR from 'swr';
export const fetcher = (url: string) => fetch(url).then((res) => res.json());

interface Icon {
  path: string,
  width: Number,
  height: Number,
}

interface ComponentSummary {
  id: String,
  name: Object,
  summary: Object,
  icons: Icon[]
}

export default function Home() {
  const { t } = useTranslation('common')
  const recently_updated = useSWR<ComponentSummary[]>('/api/apps/recently_updated', fetcher).data;
  const recently_added = useSWR<ComponentSummary[]>('/api/apps/recently_added', fetcher).data;

  return (
    <main
      className={`p-24 ${inter.className}`}
    >
      <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
        Recently Updated
      </h5>

      <div className='grid grid-cols-1 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 gap-3'>
        {recently_updated?.map(({ name, summary, icons }, index) => (
          <div key={index}
            className="block rounded-lg bg-white p-3 shadow-[0_2px_15px_-3px_rgba(0,0,0,0.07),0_10px_20px_-2px_rgba(0,0,0,0.04)] dark:bg-neutral-700">
            <Image className='float-left mx-3' src={`/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`} alt={Object.entries(name)[Object.keys(name).indexOf("C")][1]} />
            <h5
              className="mb-1 text-xl font-medium leading-tight text-neutral-800 dark:text-neutral-50">
              {Object.entries(name)[Object.keys(name).indexOf("C")][1]}
            </h5>
            <p className="mb-3 text-base text-neutral-600 dark:text-neutral-200">
              {Object.entries(summary)[Object.keys(summary).indexOf("C")][1]}
            </p>
          </div>
        ))}
      </div>

      <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
        Recently Added
      </h5>

      <div className='grid grid-cols-1 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 gap-3'>
        {recently_added?.map(({ name, summary, icons }, index) => (
          <div key={index}
            className="block rounded-lg bg-white p-3 shadow-[0_2px_15px_-3px_rgba(0,0,0,0.07),0_10px_20px_-2px_rgba(0,0,0,0.04)] dark:bg-neutral-700">
            <Image className='float-left mx-3' src={`/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`} alt={Object.entries(name)[Object.keys(name).indexOf("C")][1]} />
            <h5
              className="mb-1 text-xl font-medium leading-tight text-neutral-800 dark:text-neutral-50">
              {Object.entries(name)[Object.keys(name).indexOf("C")][1]}
            </h5>
            <p className="mb-3 text-base text-neutral-600 dark:text-neutral-200">
              {Object.entries(summary)[Object.keys(summary).indexOf("C")][1]}
            </p>
          </div>
        ))}
      </div>

    </main>
  )
}

const getStaticProps = makeStaticProps(['common'])
export { getStaticPaths, getStaticProps }