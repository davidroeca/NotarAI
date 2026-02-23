import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mkdtempSync, writeFileSync, rmSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { processHookInput } from '../../src/commands/hook-validate.js'

const VALID_SPEC_YAML = `\
schema_version: "0.4"
intent: "Test spec"
behaviors:
  - name: b1
    given: "x"
    then: "y"
artifacts:
  code:
    - path: "src/foo.ts"
      role: "test"
`

describe('processHookInput', () => {
  let tmp: string

  beforeEach(() => {
    tmp = mkdtempSync(join(tmpdir(), 'notarai-hook-'))
  })

  afterEach(() => {
    rmSync(tmp, { recursive: true, force: true })
  })

  it('returns exitCode 0 for non-spec file path', () => {
    const input = JSON.stringify({
      tool_input: { file_path: join(tmp, 'src/foo.ts') },
    })
    const result = processHookInput(input, tmp)
    expect(result.exitCode).toBe(0)
  })

  it('returns exitCode 0 for malformed JSON', () => {
    const result = processHookInput('not json!', tmp)
    expect(result.exitCode).toBe(0)
  })

  it('returns exitCode 0 for missing file_path', () => {
    const input = JSON.stringify({ tool_input: {} })
    const result = processHookInput(input, tmp)
    expect(result.exitCode).toBe(0)
  })

  it('returns exitCode 0 for valid spec file', () => {
    const specDir = join(tmp, '.notarai')
    mkdirSync(specDir, { recursive: true })
    const specPath = join(specDir, 'test.spec.yaml')
    writeFileSync(specPath, VALID_SPEC_YAML)
    const input = JSON.stringify({ tool_input: { file_path: specPath } })
    const result = processHookInput(input, tmp)
    expect(result.exitCode).toBe(0)
  })

  it('returns exitCode 1 with errors for invalid spec file', () => {
    const specDir = join(tmp, '.notarai')
    mkdirSync(specDir, { recursive: true })
    const specPath = join(specDir, 'test.spec.yaml')
    writeFileSync(specPath, 'schema_version: "0.4"\n')
    const input = JSON.stringify({ tool_input: { file_path: specPath } })
    const result = processHookInput(input, tmp)
    expect(result.exitCode).toBe(1)
    expect(result.errors).toBeDefined()
    expect(result.errors!.length).toBeGreaterThan(0)
  })

  it('returns exitCode 0 for missing file on disk', () => {
    const specPath = join(tmp, '.notarai', 'missing.spec.yaml')
    const input = JSON.stringify({ tool_input: { file_path: specPath } })
    const result = processHookInput(input, tmp)
    expect(result.exitCode).toBe(0)
  })
})
