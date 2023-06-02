import { Inter } from 'next/font/google'
import Image from 'next/image'
import PayWhatYouWantButton from '@/components/PayWhatYouWantButton'
import ImageCarousel from '@/components/ImageCarousel'
import createDOMPurify from 'dompurify'
import { JSDOM } from 'jsdom'

const inter = Inter({ subsets: ['latin'] })

interface Icon {
  path: string
  width: number
  height: number
}

interface TranslatableString {
  readonly [key: string]: string
}

interface Image {
  url: string
}

interface Screenshot {
  is_default: boolean
  images: Image[]
}

interface Component {
  id: string
  name: TranslatableString
  summary: TranslatableString
  description: TranslatableString
  icons: Icon[]
  screenshots: Screenshot[]
  metadata: { readonly [key: string]: string | undefined } | undefined
}

async function getData(id: string): Promise<Component | undefined> {
  try {
    const res = await fetch(
      `${process.env.SERVER_SIDE_API_URL}/static/apps/${id}.json`,
      { next: { revalidate: 600 } }
    )

    if (!res.ok) {
      throw new Error(`Failed to fetch data for ${id}`)
    }

    return res.json()
  } catch (e) {
    console.log(e)
  }
}

export default async function Page({
  params: { lang, id },
}: {
  params: { lang: string; id: string }
}) {
  const appdata = await getData(id)

  const window = new JSDOM('').window
  const DOMPurify = createDOMPurify(window)

  return (
    <main className={`${inter.className}`}>
      {appdata && (
        <>
          <div className="my-3 lg:flex lg:items-center lg:justify-between lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
            <div className="min-w-0 flex-1 flex">
              <Image
                className="flex-inline"
                width={64}
                height={64}
                alt={''}
                src={`${process.env.SERVER_SIDE_API_URL}/static/apps/icons/${appdata?.icons[0].width}x${appdata?.icons[0].height}/${appdata?.icons[0].path}`}
              />
              <div className="flex-inline mx-3">
                <h2 className="text-2xl font-bold leading-7 text-gray-900 sm:truncate sm:text-3xl sm:tracking-tight">
                  {appdata.name[lang] ?? appdata.name['C']}
                </h2>
                <div className="mt-2 flex items-center text-sm text-gray-500">
                  {appdata.summary[lang] ?? appdata.summary['C']}
                </div>
              </div>
            </div>
            <PayWhatYouWantButton
              appId={id}
              suggestedPrice={
                appdata.metadata &&
                appdata.metadata['x-appcenter-suggested-price']
              }
              appName={appdata.name[lang] ?? appdata.name['C']}
            />
          </div>
          <div id="custom-carousel" className="bg-gray-300">
            {appdata.screenshots.length > 1 && (
              <ImageCarousel>
                {appdata.screenshots.map((sc, index) => (
                  <div key={index}>
                    <img src={sc.images[0].url} alt="screenshot" />
                  </div>
                ))}
              </ImageCarousel>
            )}

            {appdata.screenshots.length == 1 && (
              <img
                className="m-auto"
                src={appdata.screenshots[0].images[0].url}
                alt="screenshot"
              />
            )}
          </div>
          <div
            id="app-description"
            className="lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8"
          >
            <h2 className="text-xl sm:text-2xl font-bold leading-7 text-gray-900 sm:truncate sm:tracking-tight">
              Description
            </h2>

            {appdata.description && (
              <div
                dangerouslySetInnerHTML={{
                  __html: DOMPurify.sanitize(
                    appdata.description[lang] ?? appdata.description['C']
                  ),
                }}
              />
            )}
          </div>
        </>
      )}
    </main>
  )
}
