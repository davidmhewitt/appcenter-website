module.exports = {
  debug: process.env.NODE_ENV === 'development',
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'fr', 'de', 'it', 'ja'],
    returnEmptyString: false,
    returnNull: false,
  },
}