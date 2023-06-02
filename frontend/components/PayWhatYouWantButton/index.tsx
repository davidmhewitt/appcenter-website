'use client'

import { ChevronDownIcon } from '@heroicons/react/24/solid'
import { fetcher } from '@/app/swrFetcher'
import useSWR from 'swr'

import { components } from '@/app/schema'
type App = components['schemas']['App']

type PayWhatYouWantButtonProps = {
  appId: string
  suggestedPrice?: string | undefined
}

export default function PayWhatYouWantButton({
  appId,
  suggestedPrice,
}: PayWhatYouWantButtonProps) {
  const { data: app } = useSWR<App>(`/api/apps${encodeURI(appId)}`, fetcher)

  return (
    <div className="mt-5 flex items-stretch">
      <span className="flex items-stretch">
        <button
          type="button"
          className="inline-flex rounded-l-md bg-indigo-600 px-3 py-2 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
        >
          {`$${suggestedPrice}.00` ?? 'Free'}
        </button>
        <button
          type="button"
          className="inline-flex h-full place-items-center rounded-r-md bg-indigo-600 px-3 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
        >
          <ChevronDownIcon className="h-4" />
        </button>
      </span>
    </div>
  )
}
