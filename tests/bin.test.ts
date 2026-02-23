import { describe, it, expect } from 'vitest'
import { execFileSync } from 'node:child_process'
import { resolve } from 'node:path'

const binPath = resolve(import.meta.dirname, '../dist/bin.js')

function run(
  args: string[],
  options?: { input?: string },
): { stdout: string; stderr: string; exitCode: number } {
  try {
    const stdout = execFileSync('node', [binPath, ...args], {
      encoding: 'utf-8',
      input: options?.input,
      timeout: 10_000,
    })
    return { stdout, stderr: '', exitCode: 0 }
  } catch (err) {
    const e = err as {
      stdout?: string
      stderr?: string
      status?: number
    }
    return {
      stdout: e.stdout ?? '',
      stderr: e.stderr ?? '',
      exitCode: e.status ?? 1,
    }
  }
}

describe('bin.ts CLI', () => {
  it('--help exits 0 and prints usage', () => {
    const result = run(['--help'])
    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain('Usage: notarai')
  })

  it('unknown command exits 1 and prints usage', () => {
    const result = run(['nonexistent'])
    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain('Usage: notarai')
  })

  it('validate exits 0 for repo .notarai/', () => {
    const result = run(['validate'])
    expect(result.exitCode).toBe(0)
    expect(result.stdout).toContain('PASS')
  })
})
