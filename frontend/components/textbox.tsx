import React from 'react';
import { UseFormRegisterReturn, FieldValues } from 'react-hook-form';

type TextBoxProps = {
    name: string,
    title: string,
    type?: string,
    labelClassName?: string,
    inputClassName?: string,
    autoComplete?: string,
    errorMessage?: string,
    register?: UseFormRegisterReturn,
}
export default function TextBox(props: TextBoxProps) {
    return (
        <div className='mb-4'>
            <label htmlFor={props.name} className={`block text-sm font-medium leading-6 text-gray-900 ${props.labelClassName}`}>{props.title}</label>
            <div className="mt-2">
                <input id={props.name} type={props.type} autoComplete={props.autoComplete} {...props.register} className={`block w-full rounded-md border-0 px-2 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6 ${props.inputClassName}`} />
                <p className={`mt-1 text-sm text-red-600 dark:text-red-500`}>{props.errorMessage}</p>
            </div>
        </div>
    )
}