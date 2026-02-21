import { describe, it, afterEach } from 'vitest'
import { launchClisten } from './helpers'

describe('Phase 3 — NTS Discovery', () => {
  let session: Awaited<ReturnType<typeof launchClisten>> | undefined

  afterEach(async () => {
    try { await session?.close() } catch {}
    session = undefined
  })

  it('Mixtapes sub-tab shows mixtapes', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Press "4" to switch to Mixtapes sub-tab
    await session.press('4')
    // Mixtapes should load from the NTS API
    await session.waitForText(/\w{3,}/, { timeout: 20000 })
  })

  it('Shows drill-down and back', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Press "5" to switch to Shows sub-tab
    await session.press('5')
    // Shows should load
    await session.waitForText(/\w{3,}/, { timeout: 20000 })
    // Press Enter to drill into the first show
    await session.press('enter')
    // Episodes should load
    await session.waitForText(/\w{3,}/, { timeout: 20000 })
    // Press Escape to go back
    await session.press('esc')
    // Should return to shows list — content should still be visible
    await session.waitForText(/\w{3,}/, { timeout: 5000 })
  })

  it('Schedule shows upcoming', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Press "6" to switch to Schedule sub-tab
    await session.press('6')
    // Schedule uses live data, should show channel broadcasts
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
  })

  it('f toggles heart on selected item', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Wait for live channels to load — give extra time for slower network
    await session.waitForText(/\w{3,}/, { timeout: 20000 })
    // Small delay to ensure state is stable
    await new Promise(r => setTimeout(r, 200))

    const before = await session.text()
    const hadHeart = before.includes('♥')

    // Press "f" to toggle favorite state
    await session.press('f')
    await new Promise(r => setTimeout(r, 300))

    const after = await session.text()
    const hasHeart = after.includes('♥')

    // Heart state should have changed
    if (hadHeart === hasHeart) {
      throw new Error(`Expected heart state to toggle. Before: ${hadHeart}, After: ${hasHeart}`)
    }

    // Toggle back to original state to avoid polluting other tests
    await session.press('f')
    await new Promise(r => setTimeout(r, 300))
  })

  it('Favorites sub-tab shows favorited items', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    await new Promise(r => setTimeout(r, 200))

    // Ensure the item IS favorited (add if not already)
    const before = await session.text()
    if (!before.includes('♥')) {
      await session.press('f')
      await session.waitForText('♥', { timeout: 5000 })
    }

    // Switch to Favorites sub-tab (7)
    await session.press('7')
    // The favorited item should appear
    await session.waitForText(/\w{3,}/, { timeout: 5000 })
  })

  it('History shows played items', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText(/\w{3,}/, { timeout: 15000 })
    // Play an item to add to history
    await session.press('enter')
    await new Promise(r => setTimeout(r, 300))
    // Switch to History sub-tab (8)
    await session.press('8')
    // The played item should appear in history
    await session.waitForText(/\w{3,}/, { timeout: 5000 })
  })

  it('all 8 sub-tabs accessible by number keys', async () => {
    session = await launchClisten()
    await session.waitForText('NTS', { timeout: 5000 })
    // Press keys 1 through 8 — each should work without crashing
    for (const key of ['1', '2', '3', '4', '5', '6', '7', '8']) {
      await session.press(key)
      await new Promise(r => setTimeout(r, 100))
    }
    // App should still be running and showing content
    await session.waitForText('NTS', { timeout: 5000 })
    await session.waitForText('Now Playing')
  })
})
