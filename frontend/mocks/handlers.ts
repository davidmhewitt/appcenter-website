import { rest } from 'msw'
import recentlyAdded from './data/recently_added.json' assert {type: 'json'}
import recentlyUpdated from './data/recently_updated.json' assert {type: 'json'}
import app from './data/com.github.phase1geo.minder.json' assert {type: 'json'}
import image from './data/io.elementary.iconbrowser.png'

export const handlers = [
  rest.get('/api/apps/recently_added', (_req, res, ctx) => {
    return res(
      ctx.json(recentlyAdded)
    )
  }),
  rest.get('/api/apps/recently_updated', (_req, res, ctx) => {
    return res(
      ctx.json(recentlyUpdated)
    )
  }),
  rest.get('/static/apps/icons/64x64/:id', async (req, res, ctx) => {
    const imageBuffer = await fetch(image.src).then((res) =>
      res.arrayBuffer(),
    )
    return res(
      ctx.set('Content-Length', imageBuffer.byteLength.toString()),
      ctx.set('Content-Type', 'image/png'),
      // Respond with the "ArrayBuffer".
      ctx.body(imageBuffer),
    )
  }),
  rest.get('/static/apps/:id', async (req, res, ctx) => {
    return res(
      ctx.json(app)
    )
  }),
  rest.get('/api/user/apps', async (req, res, ctx) => {
    return res(
      ctx.json(
        [
          {
            "app_id": "com.github.davidmhewitt.torrential",
            "verified": true,
            "version": "3.0.0"
          },
          {
            "app_id": "com.github.davidmhewitt.clipped",
            "verified": true,
            "version": "1.0.3"
          },
          {
            "app_id": "io.elementary.calculator",
            "verified": false,
            "version": "7.1.3"
          },
        ]
      )
    )
  })
]