'use client'

import React, { useState } from 'react'
import Image from 'next/image'
import Link from 'next/link'
import TextBox from '@/components/TextBox'

import { useForm } from 'react-hook-form'
import { yupResolver } from '@hookform/resolvers/yup'
import * as Yup from 'yup'

import githubLogo from '../../../public/github-mark-white.svg'

import { useTranslation } from '@/app/i18n/client'
import { useSearchParams } from 'next/navigation'

export default function Register({
  params: { lang },
}: {
  params: { lang: string }
}) {
  const { t } = useTranslation(['register', 'server'], lang)

  // Set up translation keys for validation errors
  // t('validation.email-required', {ns: 'register'})
  // t('validation.email-invalid', {ns: 'register'})
  // t('validation.password-min-6-chars', {ns: 'register'})
  // t('validation.password-required', {ns: 'register'})
  // t('validation.passwords-not-matching', {ns: 'register'})
  // t('validation.confirm-password-required', {ns: 'register'})

  // Set up translation keys for server errors
  // t('registration.generic-problem', {ns: 'server'})
  // t('registration.user-already-exists', {ns: 'server'})
  // t('registration.no-email-permission', {ns: 'server'})

  type FormValues = {
    email: string
    password: string
    confirmPassword: string
  }

  const validationSchema = Yup.object().shape({
    email: Yup.string()
      .required('validation.email-required')
      .email('validation.email-invalid'),
    password: Yup.string()
      .min(6, 'validation.password-min-6-chars')
      .required('validation.password-required'),
    confirmPassword: Yup.string()
      .oneOf(
        [Yup.ref('password'), undefined],
        'validation.passwords-not-matching'
      )
      .required('validation.confirm-password-required'),
  })

  const searchParams = useSearchParams()
  const [registerResult, setRegisterResult] = useState<any>({
    error: searchParams.get('error'),
    translation_key: searchParams.get('error'),
  })

  const formOptions = { resolver: yupResolver(validationSchema) }
  const { register, handleSubmit, formState } = useForm<FormValues>(formOptions)
  const { errors } = formState

  const [isLoading, setLoading] = useState(false)
  const onSubmit = handleSubmit(async (data) => {
    const submitData = {
      email: data.email,
      password: data.password,
    }

    const JSONdata = JSON.stringify(submitData)
    const endpoint = '/api/users/register'

    const options = {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSONdata,
    }

    setLoading(true)
    const response = await fetch(endpoint, options)
    setLoading(false)

    const result = await response.json()
    setRegisterResult(result)

    return false
  })

  return (
    <>
      <div className="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
        <div className="sm:mx-auto sm:w-full sm:max-w-md">
          <h2 className="text-center text-2xl font-bold leading-9 tracking-tight text-gray-900">
            {t('page-title')}
          </h2>
        </div>

        <div className="mt-10 sm:mx-auto sm:w-full sm:max-w-md">
          {registerResult?.error && (
            <div className="font-regular relative block w-full max-w-screen-md rounded-lg bg-red-500 px-4 py-4 text-base text-white">
              <div className="absolute top-4 left-4">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                  aria-hidden="true"
                  className="h-6 w-6"
                >
                  <path
                    fillRule="evenodd"
                    d="M9.401 3.003c1.155-2 4.043-2 5.197 0l7.355 12.748c1.154 2-.29 4.5-2.599 4.5H4.645c-2.309 0-3.752-2.5-2.598-4.5L9.4 3.003zM12 8.25a.75.75 0 01.75.75v3.75a.75.75 0 01-1.5 0V9a.75.75 0 01.75-.75zm0 8.25a.75.75 0 100-1.5.75.75 0 000 1.5z"
                    clipRule="evenodd"
                  ></path>
                </svg>
              </div>
              <div className="ml-8 mr-12">
                <h5 className="block font-sans text-xl font-semibold leading-snug tracking-normal text-white antialiased">
                  Account Creation Failed
                </h5>
                <p className="mt-2 block font-sans text-base font-normal leading-relaxed text-white antialiased">
                  {registerResult.translation_key
                    ? t(registerResult.translation_key, { ns: 'server' })
                    : registerResult.error}
                </p>
              </div>
            </div>
          )}

          {registerResult?.message && (
            <div className="font-regular relative block w-full max-w-screen-md rounded-lg bg-green-500 px-4 py-4 text-base text-white">
              <div className="absolute top-4 left-4">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                  aria-hidden="true"
                  className="mt-px h-6 w-6"
                >
                  <path
                    fillRule="evenodd"
                    d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12zm13.36-1.814a.75.75 0 10-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 00-1.06 1.06l2.25 2.25a.75.75 0 001.14-.094l3.75-5.25z"
                    clipRule="evenodd"
                  ></path>
                </svg>
              </div>
              <div className="ml-8 mr-12">
                <h5 className="block font-sans text-xl font-semibold leading-snug tracking-normal text-white antialiased">
                  Success
                </h5>
                <p className="mt-2 block font-sans text-base font-normal leading-relaxed text-white antialiased">
                  {registerResult.translation_key
                    ? t(registerResult.translation_key)
                    : registerResult.message}
                </p>
              </div>
            </div>
          )}

          {!registerResult?.message && (
            <>
              <form
                onSubmit={onSubmit}
                action="/api/users/register"
                method="POST"
              >
                <div className="flex">
                  <div className="w-full">
                    <TextBox
                      name="email"
                      autoComplete="email"
                      register={register('email')}
                      title={t('form-labels.email-address')}
                      errorMessage={
                        errors.email?.message
                          ? (t(errors.email.message) as string)
                          : undefined
                      }
                      inputClassName={`${
                        errors.email
                          ? 'bg-red-50 ring-red-500 text-red-900 placeholder-red-700 focus:ring-red-500 dark:bg-red-100 dark:border-red-400'
                          : ''
                      }`}
                      labelClassName={`${
                        errors.email ? 'text-red-700 dark:text-red-500' : ''
                      }`}
                    />
                  </div>
                </div>

                <div className="flex">
                  <div className="w-full">
                    <TextBox
                      name="password"
                      type="password"
                      autoComplete="new-password"
                      register={register('password')}
                      title={t('form-labels.password')}
                      errorMessage={
                        errors.password?.message
                          ? (t(errors.password.message) as string)
                          : undefined
                      }
                      inputClassName={`${
                        errors.password
                          ? 'bg-red-50 ring-red-500 text-red-900 placeholder-red-700 focus:ring-red-500 dark:bg-red-100 dark:border-red-400'
                          : ''
                      }`}
                      labelClassName={`${
                        errors.password ? 'text-red-700 dark:text-red-500' : ''
                      }`}
                    />
                  </div>
                </div>

                <div className="flex">
                  <div className="w-full">
                    <TextBox
                      name="confirmPassword"
                      type="password"
                      autoComplete="new-password"
                      register={register('confirmPassword')}
                      title={t('form-labels.confirm-password')}
                      errorMessage={
                        errors.confirmPassword?.message
                          ? (t(errors.confirmPassword.message) as string)
                          : undefined
                      }
                      inputClassName={`${
                        errors.confirmPassword
                          ? 'bg-red-50 ring-red-500 text-red-900 placeholder-red-700 focus:ring-red-500 dark:bg-red-100 dark:border-red-400'
                          : ''
                      }`}
                      labelClassName={`${
                        errors.confirmPassword
                          ? 'text-red-700 dark:text-red-500'
                          : ''
                      }`}
                    />
                  </div>
                </div>

                <div>
                  <button
                    disabled={isLoading}
                    type="submit"
                    className="flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                  >
                    {t('form-labels.register-button')}
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
                <Image
                  className="w-4 mx-2"
                  src={githubLogo}
                  alt="GitHub Logo"
                />
                Continue with GitHub
              </Link>

              <p className="mt-10 text-center text-sm text-gray-500">
                Already have an account?&nbsp;
                <a
                  href="#"
                  className="font-semibold leading-6 text-indigo-600 hover:text-indigo-500"
                >
                  Login
                </a>
              </p>
            </>
          )}
        </div>
      </div>
    </>
  )
}
