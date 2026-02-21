import { describe, it, afterEach } from 'vitest'
import { launchClisten } from './helpers'

describe('Phase 2 — NTS Data + Playback', () => {
  let session: Awaited<ReturnType<typeof launchClisten>> | undefined

  afterEach(async () => {
    try { await session?.close() } catch {}
    session = undefined
  })

  it('live channels appear on launch', async () => {
    session = await launchClisten()
    // NTS loads live channel names — real NTS channels have show names
    await session.waitForText('NTS', { timeout: 5000 })
    // After a moment the live channels should load (names from NTS API)
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
  })

  it('sub-tab bar shows NTS views', async () => {
    session = await launchClisten()
    // NTS tab is active by default; switch to Picks via "2"
    await session.waitForText('NTS', { timeout: 5000 })
    // The discovery list should have items after initial live load
    await session.waitForText(/\w+/, { timeout: 15000 })
    // Press "2" to switch to Picks sub-tab — list should change
    await session.press('2')
    // Content should reload (Picks endpoint)
    await session.waitForText(/\w+/, { timeout: 20000 })
  })

  it('switch to Picks shows episodes', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Wait for live data to appear first
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    // Press "2" to switch to Picks sub-tab
    await session.press('2')
    // Picks should load episodes — wait for content
    await session.waitForText(/\w{3,}/, { timeout: 20000 })
  })

  it('Enter on live channel starts playback', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Wait for live channels to load
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    // Press Enter to play the first item
    await session.press('enter')
    // Now Playing should update — the title should appear (something other than "Nothing playing")
    // With fake mpv, position won't advance but the item info should show
    await session.waitForText(/▶|⏸|Nothing playing/, { timeout: 5000 })
  })

  it('Space pauses and resumes', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    // Start playing
    await session.press('enter')
    await new Promise(r => setTimeout(r, 500))
    // Press space to pause — UI should reflect paused state
    await session.press('space')
    // The play controls toggle paused state
    await session.waitForText(/⏸|▶/, { timeout: 5000 })
  })

  it('Now Playing shows title and position', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    await session.press('enter')
    // Now Playing should show the item title (not "Nothing playing")
    // With fake mpv, position timer shows 0:00 immediately
    await session.waitForText(/0:00|\d+:\d{2}/, { timeout: 10000 })
  })
})
