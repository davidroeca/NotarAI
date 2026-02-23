import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { validateSpec } from '../../src/lib/validator.js'

const fixtureSpec = readFileSync(
  resolve(import.meta.dirname, '../../.notarai/cli.spec.yaml'),
  'utf-8',
)

describe('validateSpec', () => {
  it('validates a real spec file as valid', () => {
    const result = validateSpec('cli.spec.yaml', fixtureSpec)
    expect(result.valid).toBe(true)
    expect(result.errors).toEqual([])
  })

  it('returns invalid for missing required fields', () => {
    const result = validateSpec('bad.spec.yaml', 'schema_version: "0.4"\n')
    expect(result.valid).toBe(false)
    expect(result.errors.length).toBeGreaterThan(0)
  })

  it('returns YAML parse error for malformed YAML', () => {
    const result = validateSpec('bad.spec.yaml', 'foo: [unterminated')
    expect(result.valid).toBe(false)
    expect(result.errors[0]).toMatch(/YAML parse error/)
  })

  it('returns invalid for wrong schema_version', () => {
    const result = validateSpec(
      'bad.spec.yaml',
      'schema_version: "99.99"\nintent: "test"\n',
    )
    expect(result.valid).toBe(false)
    expect(result.errors.some((e) => e.includes('schema_version'))).toBe(true)
  })
})
