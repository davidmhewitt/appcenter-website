import { setupTranslations } from '@/app/i18n'
import TextBox from '@/components/TextBox'
import Link from 'next/link'
import Image from 'next/image'

import githubLogo from '@/public/github-mark-white.svg'

export default async function Login({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const { t } = await setupTranslations('login', lang)

  return (
    <>
      <div className="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
        <div className="sm:mx-auto sm:w-full sm:max-w-md">
          <h2 className="text-center text-2xl font-bold leading-9 tracking-tight text-gray-900">
            {t('page-title')}
          </h2>
        </div>

        <div className="mt-10 sm:mx-auto sm:w-full sm:max-w-md">
          <form
            action={`${process.env.NEXT_PUBLIC_API_BASE_URL}/api/users/login`}
            method="POST"
          >
            <TextBox
              name="email"
              autoComplete="email"
              title={t('form-labels.email-address')}
            />

            <TextBox
              name="password"
              type="password"
              autoComplete="current-password"
              title={t('form-labels.password')}
            />

            <div>
              <button
                type="submit"
                className="flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
              >
                {t('form-labels.sign-in-button')}
              </button>
            </div>
          </form>

          <div className="my-4 flex items-center before:mt-0.5 before:flex-1 before:border-t before:border-neutral-300 after:mt-0.5 after:flex-1 after:border-t after:border-neutral-300">
            <p className="mx-4 mb-0 text-center font-semibold dark:text-neutral-200">
              OR
            </p>
          </div>

          <Link
            className="mb-3 bg-black hover:bg-slate-800 flex w-full items-center justify-center rounded-md px-3 py-1.5 text-center text-sm font-normal leading-6 text-white"
            href={`${process.env.NEXT_PUBLIC_API_BASE_URL}/api/users/github/login`}
            role="button"
          >
            <Image className="w-4 mx-2" src={githubLogo} alt="GitHub Logo" />
            {t('github-login')}
          </Link>

          <p className="mt-10 text-center text-sm text-gray-500">
            {t('no-account')}&nbsp;
            <Link
              href="/register"
              className="font-semibold leading-6 text-indigo-600 hover:text-indigo-500"
            >
              {t('register-action')}
            </Link>
          </p>
        </div>
      </div>
    </>
  )
}
