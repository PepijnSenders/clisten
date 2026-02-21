import { describe, it, afterEach } from 'vitest'
import { launchClisten } from './helpers'

describe('Phase 5 — Queue + Polish', () => {
  let session: Awaited<ReturnType<typeof launchClisten>> | undefined

  afterEach(async () => {
    try { await session?.close() } catch {}
    session = undefined
  })

  it('a adds to queue, bottom bar shows queue count', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    await session.press('a')
    await session.waitForText('Track 1/1', { timeout: 5000 })
  })

  it('c clears queue, counter disappears', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    await session.press('a')
    await session.waitForText('Track 1/1', { timeout: 5000 })
    await session.press('c')
    await new Promise(r => setTimeout(r, 300))
    const text = await session.text()
    if (text.includes('Track 1/1')) {
      throw new Error('Expected queue count to disappear after clear, but still visible')
    }
  })

  it('? shows help overlay', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.press('?')
    await session.waitForText('Keybindings', { timeout: 5000 })
    await session.waitForText('Play/Pause', { timeout: 3000 })
    await session.waitForText('Quit', { timeout: 3000 })
  })

  it('any key dismisses help overlay', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.press('?')
    await session.waitForText('Keybindings', { timeout: 5000 })
    await session.press('a')
    await new Promise(r => setTimeout(r, 300))
    const text = await session.text()
    if (text.includes('Keybindings') && text.includes('Press any key to close')) {
      throw new Error('Expected help overlay to be dismissed')
    }
    await session.waitForText('NTS', { timeout: 3000 })
  })

  it('missing mpv shows error on launch', async () => {
    session = await launchClisten({ noFakeMpv: true, env: { PATH: '/nonexistent' } })
    await session.waitForText(/mpv is required|mpv not found|Install.*mpv/, { timeout: 10000 })
  })
})
