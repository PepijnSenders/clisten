import { launchTerminal } from 'tuistory'
import { resolve } from 'path'
import { mkdtempSync, writeFileSync, chmodSync, existsSync, mkdirSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'

const BINARY = resolve(__dirname, '../../target/debug/clisten')

// Create a temporary directory with a fake mpv stub so the dependency check passes.
// The stub does nothing (sleeps forever, so the TUI can launch without real audio).
function createFakeMpvDir(): string {
  const dir = mkdtempSync(join(tmpdir(), 'clisten-e2e-'))
  const fakeMpv = join(dir, 'mpv')
  // A minimal shell script that just sleeps so mpv IPC socket logic can proceed
  writeFileSync(fakeMpv, '#!/bin/sh\nsleep 3600\n')
  chmodSync(fakeMpv, 0o755)
  return dir
}

let _fakeMpvDir: string | undefined

function getFakeMpvDir(): string {
  if (!_fakeMpvDir) {
    _fakeMpvDir = createFakeMpvDir()
  }
  return _fakeMpvDir
}

export async function launchClisten(opts?: {
  cols?: number
  rows?: number
  env?: Record<string, string>
  noFakeMpv?: boolean
}) {
  const fakeMpvDir = opts?.noFakeMpv ? undefined : getFakeMpvDir()
  const pathEnv = fakeMpvDir
    ? `${fakeMpvDir}:${process.env.PATH ?? ''}`
    : process.env.PATH ?? ''

  return launchTerminal({
    command: BINARY,
    cols: opts?.cols ?? 120,
    rows: opts?.rows ?? 40,
    env: { ...process.env, PATH: pathEnv, ...opts?.env },
  })
}
