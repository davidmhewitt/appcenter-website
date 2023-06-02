'use client'

import AddAppPopoverButton from '@/components/AddAppPopoverButton'
import SubmitAppUpdateButton from '@/components/SubmitAppUpdateButton'
import { CheckBadgeIcon } from '@heroicons/react/24/outline'

import { components } from '@/app/schema'

import { useRouter } from 'next/navigation'
import useSWR from 'swr'

type App = components['schemas']['App']
type StripeAccount = components['schemas']['StripeAccount']

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

export default function Dashboard({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const router = useRouter()

  const {
    data: apps,
    mutate: appsMutator,
    isLoading: appsLoading,
  } = useSWR<App[]>('/api/dashboard/apps', fetcher)

  const {
    data: stripeAccount,
    mutate: stripeMutator,
    isLoading: stripeLoading,
  } = useSWR<StripeAccount>('/api/dashboard/stripe_account', fetcher)

  async function enableMonetisation(app_id: string) {
    if (stripeAccount?.account_id == null) {
      const endpoint = `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/dashboard/create_stripe_account`

      const options: RequestInit = {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
      }

      const response = await fetch(endpoint, options)
      await stripeMutator()
      // TODO: Show the error to the user
      if (response.status != 200) {
        return
      }
    }

    if (!stripeAccount?.charges_enabled) {
      router.push(
        `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/dashboard/link_stripe_account`
      )
    }

    const endpoint = `${
      process.env.NEXT_PUBLIC_API_BASE_URL
    }/api/dashboard/enable_app_payments/${encodeURI(app_id)}`

    const options: RequestInit = {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include',
    }

    // TODO: Handle and show errors
    await fetch(endpoint, options)
    await appsMutator()
  }

  return (
    <>
      <div className="my-3 lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
        <div className="flex justify-end">
          <AddAppPopoverButton mutator={appsMutator} />
        </div>
        <div className="flex flex-col">
          <div className="sm:-mx-6 lg:-mx-8 grow">
            <div className="inline-block min-w-full py-2 sm:px-6 lg:px-8">
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
                  {!appsLoading &&
                    apps?.map(
                      (
                        {
                          id,
                          is_verified,
                          stripe_connect_id,
                          last_submitted_version,
                        },
                        index
                      ) => (
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
                            {last_submitted_version}
                          </td>
                          <td className="whitespace-nowrap px-4 py-3">
                            {is_verified && (
                              <SubmitAppUpdateButton appId={id} />
                            )}
                            {!stripeLoading &&
                              (stripeAccount?.account_id == null ||
                                stripe_connect_id == null) && (
                                <button
                                  onClick={() => enableMonetisation(id)}
                                  className="group inline-flex items-center rounded-md bg-indigo-700 my-2 px-3 py-2 text-base font-medium text-white hover:text-opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"
                                >
                                  Enable Monetization
                                </button>
                              )}
                          </td>
                        </tr>
                      )
                    )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
        <h5 className="mb-2 mt-0 text-xl font-medium leading-tight text-primary">
          Monetisation
        </h5>
        <p>
          <b>Stripe Account Key: </b>
          {stripeAccount?.account_id}
        </p>
        <p>
          <b>Stripe Account Enabled: </b>
          {stripeAccount?.charges_enabled ? 'true' : 'false'}
        </p>
      </div>
    </>
  )
}
