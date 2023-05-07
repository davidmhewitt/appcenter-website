import { Inter } from 'next/font/google'
import Image from 'next/image'
import DOMPurify from 'dompurify';
import {
  ChevronDownIcon,
} from '@heroicons/react/24/solid'
import { Menu, Transition } from '@headlessui/react'
import "react-responsive-carousel/lib/styles/carousel.min.css";
import { Carousel } from 'react-responsive-carousel';

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

interface TranslatableString {
  readonly [key: string]: string;
}

interface Image {
  url: string,
}

interface Screenshot {
  is_default: boolean,
  images: Image[],
}

interface Component {
  id: string,
  name: TranslatableString,
  summary: TranslatableString,
  description: TranslatableString,
  icons: Icon[],
  screenshots: Screenshot[],
}

export default function Home() {
  const { t } = useTranslation('common');
  const router = useRouter();

  const appdata = useSWR<Component>('/static/apps/' + router.query.id + '.json', fetcher).data;

  return (
    <main
      className={`${inter.className}`}
    >
      {appdata &&
        <>
          <div className="my-3 lg:flex lg:items-center lg:justify-between lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
            <div className="min-w-0 flex-1 flex">
              <Image className="flex-inline" width={64} height={64} alt={''} src={`/static/apps/icons/${appdata?.icons[0].width}x${appdata?.icons[0].height}/${appdata?.icons[0].path}`} />
              <div className='flex-inline mx-3'>
                <h2 className="text-2xl font-bold leading-7 text-gray-900 sm:truncate sm:text-3xl sm:tracking-tight">
                  {appdata.name[router.query.locale as string] ?? appdata.name["C"]}
                </h2>
                <div className="mt-2 flex items-center text-sm text-gray-500">
                  {appdata.summary[router.query.locale as string] ?? appdata.summary["C"]}
                </div>
              </div>
            </div>
            <div className="mt-5 flex items-stretch">
              <span className="flex items-stretch">
                <button
                  type="button"
                  className="inline-flex rounded-l-md bg-indigo-600 px-3 py-2 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                >
                  $3.00
                </button>
                <button
                  type="button"
                  className="inline-flex h-full place-items-center rounded-r-md bg-indigo-600 px-3 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                >
                  <ChevronDownIcon className='h-4' />
                </button>
              </span>
            </div>
          </div>
          <div id='custom-carousel' className='bg-gray-300'>
            {appdata.screenshots.length > 1 && (
              <Carousel
                showThumbs={false}
                infiniteLoop={true}
                autoPlay={false}
                showArrows={true}
                showIndicators={appdata.screenshots.length > 1}
                swipeable={true}
                emulateTouch={true}
                useKeyboardArrows={true}
                showStatus={false}
                dynamicHeight={false}
                className='lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8'
              >
                {appdata.screenshots.map((sc, index) => (
                  <div key={index}>
                    <img src={sc.images[0].url} alt='screenshot' />
                  </div>
                ))}
              </Carousel>
            )}

            {appdata.screenshots.length == 1 && (
              <img className='m-auto' src={appdata.screenshots[0].images[0].url} alt='screenshot' />
            )}

          </div>
          <div id='app-description' className='lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8'>
            <h2 className="text-xl sm:text-2xl font-bold leading-7 text-gray-900 sm:truncate sm:tracking-tight">Description</h2>

            {appdata.description && (
              <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(appdata.description[router.query.locale as string] ?? appdata.description["C"]) }} />
            )}
          </div>
        </>
      }
    </main>
  )
}

const getStaticProps = makeStaticProps(['common'])
export { getStaticPaths, getStaticProps }

