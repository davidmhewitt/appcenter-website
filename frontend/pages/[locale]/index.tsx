import { Inter } from 'next/font/google'
import AppSummaryButton from '@/components/app_summary_button'

const inter = Inter({ subsets: ['latin'] })

import { useTranslation } from 'next-i18next'
import { useRouter } from 'next/router';
import { getStaticPaths, makeStaticProps } from '../../lib/getStatic'
import useSWR from 'swr';
export const fetcher = (url: string) => fetch(url).then((res) => res.json());


interface Icon {
  path: string,
  width: Number,
  height: Number,
}

interface TranslatableString {
  readonly [key: string]: string;
}

interface ComponentSummary {
  id: string,
  name: TranslatableString,
  summary: TranslatableString,
  icons: Icon[]
}

export default function Home() {
  const { t } = useTranslation('common')
  const router = useRouter();
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
        {recently_updated?.map(({ id, name, summary, icons }, index) => (
          <AppSummaryButton
            key={index}
            id={id}
            name={name[router.query.locale as string] ?? name["C"]}
            description={summary[router.query.locale as string] ?? summary["C"]}
            imageUrl={`/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`}
          />
        ))}
      </div>

      <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
        Recently Added
      </h5>

      <div className='grid grid-cols-1 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 gap-3'>
        {recently_added?.map(({ id, name, summary, icons }, index) => (
          <AppSummaryButton
            key={index}
            id={id}
            name={name[router.query.locale as string] ?? name["C"]}
            description={summary[router.query.locale as string] ?? summary["C"]}
            imageUrl={`/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`}
          />
        ))}
      </div>

    </main>
  )
}

const getStaticProps = makeStaticProps(['common'])
export { getStaticPaths, getStaticProps }