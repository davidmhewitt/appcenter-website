'use client'

import { Popover, Transition } from '@headlessui/react'
import TextBox from '../TextBox'
import { ChevronDownIcon } from '@heroicons/react/24/outline'
import { FormEvent, Fragment } from 'react'
import { RequestInit } from 'next/dist/server/web/spec-extension/request'
import { KeyedMutator } from 'swr'

const handleSubmit = async (event: FormEvent, mutator: KeyedMutator<any>) => {
  event.preventDefault()

  const target = event.target as typeof event.target & {
    rdnn: { value: string }
    url: { value: string }
  }

  const data = {
    app_id: target.rdnn.value,
    repository: target.url.value,
  }

  const JSONdata = JSON.stringify(data)
  const endpoint = `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/dashboard/apps`

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

  mutator()
}

interface Props {
  mutator: KeyedMutator<any>
}

export default function AddAppPopoverButton({ mutator }: Props) {
  return (
    <Popover className="relative">
      {({ open }) => (
        <>
          <Popover.Button
            className={`
                ${open ? '' : 'text-opacity-90'}
                group inline-flex items-center rounded-md bg-indigo-700 px-3 py-2 text-base font-medium text-white hover:text-opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75`}
          >
            <span>Add App</span>
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
            <Popover.Panel className="absolute left-1/2 z-10 mt-3 w-screen max-w-sm -translate-x-1/2 transform px-4 sm:px-0 lg:max-w-3xl">
              <div className="overflow-hidden rounded-lg shadow-lg ring-1 ring-black ring-opacity-5">
                <div className="bg-gray-50 p-4">
                  <form onSubmit={(e) => handleSubmit(e, mutator)}>
                    <TextBox name="rdnn" title="App ID (RDNN)" />
                    <TextBox name="url" title="Git Repository URL" />
                    <button
                      type="submit"
                      className="inline-flex rounded-md bg-indigo-600 px-3 py-2 text-base font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                    >
                      Add
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
