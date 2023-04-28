const i18nextConfig = require('./next-i18next.config')

module.exports = {
    locales: i18nextConfig.i18n.locales,
    output: 'public/locales/$LOCALE/$NAMESPACE.json',
}