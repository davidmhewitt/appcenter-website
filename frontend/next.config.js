/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: 'export',
  distDir: '_static',
  trailingSlash: true,
  images: {
    unoptimized: true
  },
}

module.exports = nextConfig
