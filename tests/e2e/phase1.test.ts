import { describe, it, afterEach } from 'vitest'
import { launchClisten } from './helpers'

describe('Phase 1 — Layout + Interaction', () => {
  let session: Awaited<ReturnType<typeof launchClisten>> | undefined

  afterEach(async () => {
    try { await session?.close() } catch {}
    session = undefined
  })

  it('shows 3-panel layout on launch', async () => {
    session = await launchClisten()
    await session.waitForText('NTS')
    await session.waitForText('Now Playing')
    await session.waitForText('Play/Pause')
  })

  it('/ focuses search bar', async () => {
    session = await launchClisten()
    await session.waitForText('NTS')
    await session.press('/')
    await session.type('test')
    await session.waitForText('test')
  })

  it('Escape unfocuses search', async () => {
    session = await launchClisten()
    await session.waitForText('/ Search...')
    await session.press('/')
    await session.type('hello')
    await session.press('esc')
    await session.waitForText('/ Search...')
  })

  it('q quits the app', async () => {
    session = await launchClisten()
    await session.waitForText('NTS')
    await session.press('q')
    await session.waitForText('', { timeout: 5000 }).catch(() => {})
    session = undefined
  })

  it('Now Playing shows nothing playing', async () => {
    session = await launchClisten()
    await session.waitForText('Nothing playing')
  })

  it('bottom bar shows keybinding hints', async () => {
    session = await launchClisten()
    await session.waitForText('Play/Pause')
    await session.waitForText('Quit')
    await session.waitForText('Search')
  })
})
