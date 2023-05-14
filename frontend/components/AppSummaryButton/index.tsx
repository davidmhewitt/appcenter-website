import React from 'react';
import Image from 'next/image'
import Link from 'next/link';

type AppButtonProps = {
    id: string,
    name: string,
    description: string,
    imageUrl: string,
}
export default function AppSummaryButton({id, name, description, imageUrl}: AppButtonProps) {
    return (
        <Link href={`/app/${id}`}>
            <div
                className="block rounded-lg bg-white p-3 shadow-[0_2px_15px_-3px_rgba(0,0,0,0.07),0_10px_20px_-2px_rgba(0,0,0,0.04)] dark:bg-neutral-700">
                <Image width={64} height={64} className='float-left mx-3' src={imageUrl} alt={name} />
                <h5
                    className="mb-1 text-xl font-medium leading-tight text-neutral-800 dark:text-neutral-50">
                    {name}
                </h5>
                <p className="mb-3 text-base text-neutral-600 dark:text-neutral-200">
                    {description}
                </p>
            </div>
        </Link>
    )
}