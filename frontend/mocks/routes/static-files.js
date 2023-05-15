module.exports = [
  {
    id: 'get-static-icon',
    url: '/static/apps/icons/64x64/*',
    variants: [
      {
        id: 'success',
        type: 'file',
        options: {
          status: 200, // Status to be sent
          path: 'mocks/files/io.elementary.iconbrowser.png', // path of the file to be transferred
          options: {
            // options for the express.sendFile method
            maxAge: 500,
          },
        },
      },
    ],
  },
  {
    id: 'get-static-app',
    url: '/static/apps/*',
    variants: [
      {
        id: 'success',
        type: 'file',
        options: {
          status: 200, // Status to be sent
          path: 'mocks/files/com.github.phase1geo.minder.json', // path of the file to be transferred
          options: {
            // options for the express.sendFile method
            maxAge: 500,
          },
        },
      },
    ],
  },
]
