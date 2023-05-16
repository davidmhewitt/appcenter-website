'use client'

import { Popover, Transition } from '@headlessui/react'
import TextBox from '../TextBox'
import { ChevronDownIcon } from '@heroicons/react/24/outline'
import { FormEvent, Fragment } from 'react'
import { RequestInit } from 'next/dist/server/web/spec-extension/request'

const handleSubmit = async (event: FormEvent, app_id: string) => {
  event.preventDefault()

  const target = event.target as typeof event.target & {
    version: { value: string }
  }

  const data = {
    version_tag: target.version.value,
    app_id,
  }

  const JSONdata = JSON.stringify(data)
  const endpoint = `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/dashboard/submit_app_update`

  const options: RequestInit = {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSONdata,
    credentials: 'include',
  }

  const response = await fetch(endpoint, options)
  // TODO: Handle the response
  const result = await response.blob()
}

interface Props {
  appId: string
}

export default function SubmitAppUpdateButton({ appId }: Props) {
  return (
    <Popover className="relative">
      {({ open }) => (
        <>
          <Popover.Button
            className={`
                ${open ? '' : 'text-opacity-90'}
                group inline-flex items-center rounded-md bg-indigo-700 px-3 py-2 text-base font-medium text-white hover:text-opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75`}
          >
            <span>Submit</span>
            <ChevronDownIcon
              className={`${open ? 'rotate-180' : 'text-opacity-70'}
                  ml-2 h-5 w-5 text-indigo-300 transition duration-150 ease-in-out group-hover:text-opacity-80`}
              aria-hidden="true"
            />
          </Popover.Button>
          <Transition
            as={Fragment}
            enter="transition ease-out duration-200"
            enterFrom="opacity-0 translate-y-1"
            enterTo="opacity-100 translate-y-0"
            leave="transition ease-in duration-150"
            leaveFrom="opacity-100 translate-y-0"
            leaveTo="opacity-0 translate-y-1"
          >
            <Popover.Panel className="absolute right-full z-10 mt-3 translate-x-1/2 transform px-4 sm:px-0 lg:max-w-3xl">
              <div className="overflow-hidden rounded-lg shadow-lg ring-1 ring-black ring-opacity-5">
                <div className="bg-gray-50 p-4">
                  <form onSubmit={(e) => handleSubmit(e, appId)}>
                    <TextBox name="version" title="Version" />
                    <button
                      type="submit"
                      className="inline-flex rounded-md bg-indigo-600 px-3 py-2 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                    >
                      Submit for Review
                    </button>
                  </form>
                </div>
              </div>
            </Popover.Panel>
          </Transition>
        </>
      )}
    </Popover>
  )
}
