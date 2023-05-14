'use client'

import "react-responsive-carousel/lib/styles/carousel.min.css";
import { Carousel } from 'react-responsive-carousel';
import { PropsWithChildren } from 'react'

export default function ImageCarousel({ children }: PropsWithChildren) {
    if (!Array.isArray(children)) return null

    return (
        <Carousel
            showThumbs={false}
            infiniteLoop={true}
            autoPlay={false}
            showArrows={true}
            showIndicators={true}
            swipeable={true}
            emulateTouch={true}
            useKeyboardArrows={true}
            showStatus={false}
            dynamicHeight={false}
            className='lg:mx-auto lg:max-w-7xl px-2 sm:px-6 lg:px-8'
        >
            {children}
        </Carousel>
    )
}