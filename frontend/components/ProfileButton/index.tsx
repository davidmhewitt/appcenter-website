'use client'

import { useAuth } from '@/context/user'
import Link from 'next/link'
import { useState } from 'react'
import ClickAwayListener from 'react-click-away-listener'
import UserContext from '@/context/user'

function InnerProfileButton() {
  const [profilePopup, setProfilePopup] = useState(false)
  const { user, logout } = useAuth()

  return (
    <>
      {user && (
        <div className="absolute inset-y-0 right-0 flex items-center pr-2 sm:static sm:inset-auto sm:ml-6 sm:pr-0">
          <div className="relative ml-3">
            <div>
              <button
                onClick={() => setProfilePopup(true)}
                type="button"
                className="flex rounded-full bg-gray-800 text-sm focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-gray-800"
                id="user-menu-button"
                aria-expanded={profilePopup}
                aria-haspopup="true"
              >
                <span className="sr-only">Open user menu</span>
                <img
                  className="h-8 w-8 rounded-full"
                  src="https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?ixlib=rb-1.2.1&ixid=eyJhcHBfaWQiOjEyMDd9&auto=format&fit=facearea&facepad=2&w=256&h=256&q=80"
                  alt=""
                />
              </button>
            </div>
            {profilePopup && (
              <ClickAwayListener onClickAway={() => setProfilePopup(false)}>
                <div
                  className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-md bg-white py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
                  role="menu"
                  aria-orientation="vertical"
                  aria-labelledby="user-menu-button"
                  tabIndex={-1}
                >
                  <a
                    href="#"
                    className="block px-4 py-2 text-sm text-gray-700"
                    role="menuitem"
                    tabIndex={-1}
                    id="user-menu-item-0"
                  >
                    Your Profile
                  </a>
                  <a
                    href="#"
                    className="block px-4 py-2 text-sm text-gray-700"
                    role="menuitem"
                    tabIndex={-1}
                    id="user-menu-item-1"
                  >
                    Settings
                  </a>
                  <Link
                    href="/"
                    onClick={logout}
                    className="block px-4 py-2 text-sm text-gray-700"
                    role="menuitem"
                    tabIndex={-1}
                    id="user-menu-item-2"
                  >
                    Sign out
                  </Link>
                </div>
              </ClickAwayListener>
            )}
          </div>
        </div>
      )}
    </>
  )
}
export default function ProfileButton() {
  return (
    <UserContext>
      <InnerProfileButton />
    </UserContext>
  )
}
