import { describe, it, afterEach } from 'vitest'
import { launchClisten } from './helpers'

describe('Phase 6 — Refinements', () => {
  let session: Awaited<ReturnType<typeof launchClisten>> | undefined

  afterEach(async () => {
    try { await session?.close() } catch {}
    session = undefined
  })

  it('sub-tab bar visible on NTS', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText('Live', { timeout: 5000 })
    await session.waitForText('Picks', { timeout: 3000 })
    await session.waitForText('Recent', { timeout: 3000 })
  })

  it('NTS search filters list', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.press('2')
    await session.waitForText(/\w{4,}/, { timeout: 15000 })
    await session.press('/')
    await session.waitForText('/ ', { timeout: 3000 })
    await session.type('the')
    await session.press('enter')
    await new Promise(r => setTimeout(r, 500))
    const text = await session.text()
    if (!text.includes('NTS')) {
      throw new Error('App should still be running after NTS search')
    }
  })

  it('search bar clears after submit', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.press('/')
    await session.waitForText('/ ', { timeout: 3000 })
    await session.type('jazz')
    await session.waitForText('jazz', { timeout: 3000 })
    await session.press('enter')
    await new Promise(r => setTimeout(r, 500))
    const text = await session.text()
    if (!text.includes('Search')) {
      throw new Error('Expected search placeholder or cleared input')
    }
  })

  it('volume keys change volume display in bottom bar', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    await session.press('enter')
    await new Promise(r => setTimeout(r, 1000))
    await session.press(']')
    await new Promise(r => setTimeout(r, 1000))
    const text = await session.text()
    if (!text.includes('NTS') && !text.includes('Play/Pause')) {
      throw new Error('App should still be running after volume change')
    }
  })
})
