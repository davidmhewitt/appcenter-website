import { useTranslation } from '../i18n'
import AppSummaryButton from '../../components/AppSummaryButton'

interface Icon {
  path: string
  width: number
  height: number
}

interface TranslatableString {
  readonly [key: string]: string
}

interface ComponentSummary {
  id: string
  name: TranslatableString
  summary: TranslatableString
  icons: Icon[]
}

async function getRecentlyAdded(): Promise<ComponentSummary[] | undefined> {
  try {
    const res = await fetch(
      `${process.env.SERVER_SIDE_API_URL}/api/apps/recently_added`,
      { next: { revalidate: 600 } }
    )

    if (!res.ok) {
      throw new Error('Failed to fetch data')
    }

    return res.json()
  } catch (e) {
    console.log(e)
  }
}

async function getRecentlyUpdated(): Promise<ComponentSummary[] | undefined> {
  try {
    const res = await fetch(
      `${process.env.SERVER_SIDE_API_URL}/api/apps/recently_updated`,
      { next: { revalidate: 600 } }
    )

    if (!res.ok) {
      throw new Error('Failed to fetch data')
    }

    return res.json()
  } catch (e) {
    console.log(e)
  }
}

export default async function Page({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const { t } = await useTranslation(['common'], lang)
  const added = getRecentlyAdded()
  const updated = getRecentlyUpdated()

  const [recentlyAdded, recentlyUpdated] = await Promise.all([added, updated])

  return (
    <div className="my-3 lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
      <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
        Recently Updated
      </h5>

      <div className="grid grid-cols-1 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 gap-3">
        {recentlyUpdated?.map(({ id, name, summary, icons }, index) => (
          <AppSummaryButton
            key={index}
            id={id}
            name={name[lang] ?? name['C']}
            description={summary[lang] ?? summary['C']}
            imageUrl={`${process.env.SERVER_SIDE_API_URL}/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`}
          />
        ))}
      </div>

      <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
        Recently Added
      </h5>

      <div className="grid grid-cols-1 sm:grid-cols-1 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-3 gap-3">
        {recentlyAdded?.map(({ id, name, summary, icons }, index) => (
          <AppSummaryButton
            key={index}
            id={id}
            name={name[lang] ?? name['C']}
            description={summary[lang] ?? summary['C']}
            imageUrl={`${process.env.SERVER_SIDE_API_URL}/static/apps/icons/${icons[0].width}x${icons[0].height}/${icons[0].path}`}
          />
        ))}
      </div>
    </div>
  )
}
