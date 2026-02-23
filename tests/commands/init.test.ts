import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  mkdtempSync,
  rmSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
} from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { runInit } from '../../src/commands/init.js'

describe('runInit', () => {
  let tmp: string

  beforeEach(() => {
    tmp = mkdtempSync(join(tmpdir(), 'notarai-init-'))
  })

  afterEach(() => {
    rmSync(tmp, { recursive: true, force: true })
  })

  it('creates .claude/ dir when missing', () => {
    runInit(tmp)
    expect(existsSync(join(tmp, '.claude'))).toBe(true)
  })

  it('creates settings.json with PostToolUse hook', () => {
    runInit(tmp)
    const settings = JSON.parse(
      readFileSync(join(tmp, '.claude', 'settings.json'), 'utf-8'),
    )
    expect(settings.hooks.PostToolUse).toHaveLength(1)
    expect(settings.hooks.PostToolUse[0].matcher).toBe('Write|Edit')
    expect(settings.hooks.PostToolUse[0].hooks[0].command).toBe(
      'npx notarai hook validate',
    )
  })

  it('preserves existing settings keys when adding hook', () => {
    mkdirSync(join(tmp, '.claude'), { recursive: true })
    writeFileSync(
      join(tmp, '.claude', 'settings.json'),
      JSON.stringify({ customKey: 'preserved' }),
    )
    runInit(tmp)
    const settings = JSON.parse(
      readFileSync(join(tmp, '.claude', 'settings.json'), 'utf-8'),
    )
    expect(settings.customKey).toBe('preserved')
    expect(settings.hooks.PostToolUse).toHaveLength(1)
  })

  it('is idempotent: second run does not duplicate hook', () => {
    runInit(tmp)
    runInit(tmp)
    const settings = JSON.parse(
      readFileSync(join(tmp, '.claude', 'settings.json'), 'utf-8'),
    )
    expect(settings.hooks.PostToolUse).toHaveLength(1)
  })

  it('creates CLAUDE.md when missing', () => {
    runInit(tmp)
    expect(existsSync(join(tmp, 'CLAUDE.md'))).toBe(true)
    const content = readFileSync(join(tmp, 'CLAUDE.md'), 'utf-8')
    expect(content).toContain('## NotarAI')
  })

  it('appends to existing CLAUDE.md without NotarAI header', () => {
    writeFileSync(join(tmp, 'CLAUDE.md'), '# My Project\n\nExisting content.\n')
    runInit(tmp)
    const content = readFileSync(join(tmp, 'CLAUDE.md'), 'utf-8')
    expect(content).toContain('# My Project')
    expect(content).toContain('## NotarAI')
  })

  it('skips CLAUDE.md when NotarAI header present and matches', () => {
    // First run creates it
    runInit(tmp)
    const original = readFileSync(join(tmp, 'CLAUDE.md'), 'utf-8')
    // Second run should skip
    runInit(tmp)
    const after = readFileSync(join(tmp, 'CLAUDE.md'), 'utf-8')
    expect(after).toBe(original)
  })

  it('warns when NotarAI section content has drifted from template', () => {
    runInit(tmp)
    // Modify the NotarAI section
    const claudeMdPath = join(tmp, 'CLAUDE.md')
    const content = readFileSync(claudeMdPath, 'utf-8')
    writeFileSync(claudeMdPath, content.replace('intent', 'MODIFIED'))

    const warns: string[] = []
    const origWarn = console.warn
    console.warn = (msg: string) => warns.push(msg)
    try {
      runInit(tmp)
    } finally {
      console.warn = origWarn
    }
    expect(warns.some((w) => w.includes('drifted'))).toBe(true)
  })

  it('does not warn when section matches template', () => {
    runInit(tmp)

    const warns: string[] = []
    const origWarn = console.warn
    console.warn = (msg: string) => warns.push(msg)
    try {
      runInit(tmp)
    } finally {
      console.warn = origWarn
    }
    expect(warns.some((w) => w.includes('drifted'))).toBe(false)
  })

  it('copies slash commands to .claude/commands/', () => {
    runInit(tmp)
    expect(
      existsSync(join(tmp, '.claude', 'commands', 'notarai-reconcile.md')),
    ).toBe(true)
    expect(
      existsSync(join(tmp, '.claude', 'commands', 'notarai-bootstrap.md')),
    ).toBe(true)
  })

  it('copies schema to .claude/notarai.spec.json', () => {
    runInit(tmp)
    expect(existsSync(join(tmp, '.claude', 'notarai.spec.json'))).toBe(true)
  })

  it('always overwrites schema on re-run', () => {
    runInit(tmp)
    const schemaPath = join(tmp, '.claude', 'notarai.spec.json')
    writeFileSync(schemaPath, '{}')
    runInit(tmp)
    const content = readFileSync(schemaPath, 'utf-8')
    expect(content).not.toBe('{}')
  })
})
