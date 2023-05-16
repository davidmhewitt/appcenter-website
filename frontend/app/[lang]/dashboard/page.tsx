'use client'

import AddAppPopoverButton from '@/components/AddAppPopoverButton'
import { CheckBadgeIcon, CloudArrowUpIcon } from '@heroicons/react/24/outline'

import useSWR from 'swr'

async function fetcher(url: string) {
  const res = await fetch(`${process.env.NEXT_PUBLIC_API_BASE_URL}${url}`, {
    credentials: 'include',
  })
  if (res.status !== 200) {
    throw new Error("Couldn't fetch")
  }

  const json = await res.json()
  return json
}

interface App {
  id: string
  is_verified: boolean
  version: string
}

export default function Dashboard({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const { data, mutate, isLoading } = useSWR<App[]>(
    '/api/dashboard/apps',
    fetcher
  )
  const apps = data

  return (
    <>
      <div className="my-3 lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
        <div className="flex justify-end">
          <AddAppPopoverButton mutator={mutate} />
        </div>
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
                    {!isLoading &&
                      apps?.map(({ id, is_verified, version }, index) => (
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
