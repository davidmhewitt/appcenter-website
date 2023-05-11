import { getStaticPaths, makeStaticProps } from '../../../lib/getStatic'

import {
    CheckBadgeIcon,
    CloudArrowUpIcon
} from '@heroicons/react/24/outline'

import useSWR from 'swr';
export const fetcher = (url: string) => fetch(url).then((res) => res.json());

interface App {
    app_id: string,
    verified: boolean,
    version: string
}

export default function Dashboard() {
    const recentlyUpdated = useSWR<App[]>('/api/user/apps', fetcher).data;

    return <>
        <div className="my-3 lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8">
            <div className="flex flex-col">
                <div className="overflow-x-auto sm:-mx-6 lg:-mx-8">
                    <div className="inline-block min-w-full py-2 sm:px-6 lg:px-8">
                        <div className="overflow-hidden">
                            <table className="min-w-full text-left text-sm font-light">
                                <thead className="border-b font-medium dark:border-neutral-500">
                                    <tr>
                                        <th scope="col" className="px-4 py-3">App Id</th>
                                        <th scope="col" className="px-4 py-3">Published Version</th>
                                        <th scope="col" className="px-4 py-3">Actions</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {recentlyUpdated?.map(({ app_id, verified, version }, index) => (
                                        <tr
                                            className="border-b transition duration-300 ease-in-out hover:bg-neutral-100 dark:border-neutral-500 dark:hover:bg-neutral-600">
                                            <td className="whitespace-nowrap px-4 py-3 font-medium">
                                                {app_id}{verified && (<CheckBadgeIcon className='w-6 h-6 inline ms-1' />)}
                                            </td>
                                            <td className="whitespace-nowrap px-4 py-3">{version}</td>
                                            <td className="whitespace-nowrap px-4 py-3"><CloudArrowUpIcon className='w-6 h-6' /></td>
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
}

const getStaticProps = makeStaticProps(['common'])
export { getStaticPaths, getStaticProps }