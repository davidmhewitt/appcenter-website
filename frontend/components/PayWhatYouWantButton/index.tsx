'use client'

import { ChevronDownIcon } from '@heroicons/react/24/solid'
import { fetcher } from '@/app/swrFetcher'
import useSWR from 'swr'
import { useRouter } from 'next/navigation'

import { components } from '@/app/schema'
import { ur } from 'make-plural'
type App = components['schemas']['App']

type PayWhatYouWantButtonProps = {
  appId: string
  suggestedPrice?: string | undefined
  appName: string
}

export default function PayWhatYouWantButton({
  appId,
  suggestedPrice,
  appName,
}: PayWhatYouWantButtonProps) {
  const { data: app } = useSWR<App>(`/api/apps/${encodeURI(appId)}`, fetcher)
  const router = useRouter()

  function startPayment(): void {
    let url = `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/payments/start`
    url += `?app_id=${encodeURIComponent(appId)}`
    url += `&app_name=${encodeURIComponent(appName)}`
    // TODO: Actual price
    url += `&amount=300`

    router.push(url)
  }

  return (
    <div className="mt-5 flex items-stretch">
      <span className="flex items-stretch">
        <button
          onClick={() => startPayment()}
          type="button"
          className="inline-flex rounded-l-md bg-indigo-600 px-3 py-2 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
        >
          {app?.stripe_connect_id != null &&
            (suggestedPrice != null ? `$${suggestedPrice}.00` : 'Free')}

          {app?.stripe_connect_id == null &&
            (suggestedPrice != null ? `Get it on AppCenter` : 'Free')}
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
