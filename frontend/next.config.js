/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  images: {
    remotePatterns: [
      {
        protocol: 'http',
        hostname: '127.0.0.1',
        port: '3100',
        pathname: '/static/**',
      },
      {
        protocol: 'http',
        hostname: 'localhost',
        port: '3100',
        pathname: '/static/**',
      },
      {
        protocol: 'http',
        hostname: 'backend',
        port: '3100',
        pathname: '/static/**',
      },
      {
        protocol: 'https',
        hostname: 'appcenter-beta.elementary.io',
        pathname: '/static/**',
      },
    ],
  },
}

module.exports = nextConfig
