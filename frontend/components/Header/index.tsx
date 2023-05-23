import React from 'react'
import Image from 'next/image'
import Link from 'next/link'
import ProfileButton from '../ProfileButton'
import AppcenterIcon from '../../public/appcenter.svg'

export default function Header() {
  return (
    <nav className="bg-gray-800">
      <div className="mx-auto max-w-7xl px-2 sm:px-6 lg:px-8">
        <div className="relative flex h-16 items-center justify-between">
          <div className="flex flex-1 items-stretch justify-start">
            <div className="flex flex-shrink-0 items-center">
              <Link href="/">
                <Image
                  src={AppcenterIcon}
                  className="block h-8 w-auto"
                  alt="AppCenter"
                />
              </Link>
            </div>
          </div>

          <ProfileButton />
        </div>
      </div>
    </nav>
  )
}
