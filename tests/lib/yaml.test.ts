import { describe, it, expect } from 'vitest'
import { parseYaml } from '../../src/lib/yaml.js'

describe('parseYaml', () => {
  it('parses valid YAML and returns data', () => {
    const result = parseYaml('foo: bar\nnum: 42')
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toEqual({ foo: 'bar', num: 42 })
    }
  })

  it('returns ok: false for invalid YAML', () => {
    const result = parseYaml('foo: [unterminated')
    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error).toBeTruthy()
    }
  })

  it('returns ok: true with null for empty string', () => {
    const result = parseYaml('')
    expect(result.ok).toBe(true)
    if (result.ok) {
      expect(result.data).toBeUndefined()
    }
  })
})
