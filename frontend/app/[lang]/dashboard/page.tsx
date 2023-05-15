import { CheckBadgeIcon, CloudArrowUpIcon } from '@heroicons/react/24/outline'

interface App {
  id: string
  is_verified: boolean
  version: string
}

async function getData(): Promise<App[] | undefined> {
  try {
    const res = await fetch(
      `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/dashboard/apps`,
      { cache: 'no-store' }
    )

    if (!res.ok) {
      throw new Error('Failed to fetch data')
    }

    return res.json()
  } catch (e) {
    console.log(e)
  }
}

export default async function Dashboard({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const apps = await getData()

  return (
    <>
      <div className="my-3 lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
        <div className="flex flex-col">
          <div className="overflow-x-auto sm:-mx-6 lg:-mx-8">
            <div className="inline-block min-w-full py-2 sm:px-6 lg:px-8">
              <div className="overflow-hidden">
                <table className="min-w-full text-left text-sm font-light">
                  <thead className="border-b font-medium dark:border-neutral-500">
                    <tr>
                      <th scope="col" className="px-4 py-3">
                        App Id
                      </th>
                      <th scope="col" className="px-4 py-3">
                        Published Version
                      </th>
                      <th scope="col" className="px-4 py-3">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {apps?.map(({ id, is_verified, version }, index) => (
                      <tr
                        key={index}
                        className="border-b transition duration-300 ease-in-out hover:bg-neutral-100 dark:border-neutral-500 dark:hover:bg-neutral-600"
                      >
                        <td className="whitespace-nowrap px-4 py-3 font-medium">
                          {id}
                          {is_verified && (
                            <CheckBadgeIcon className="w-6 h-6 inline ms-1" />
                          )}
                        </td>
                        <td className="whitespace-nowrap px-4 py-3">
                          {version}
                        </td>
                        <td className="whitespace-nowrap px-4 py-3">
                          {is_verified && (
                            <CloudArrowUpIcon className="w-6 h-6" />
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  )
}
