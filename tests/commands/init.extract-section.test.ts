import { describe, it, expect } from 'vitest'
import { extractNotarAISection } from '../../src/commands/init.js'

describe('extractNotarAISection', () => {
  it('extracts section from start to EOF', () => {
    const content = '## NotarAI\n\nSome content here.\n'
    const result = extractNotarAISection(content)
    expect(result).toBe('## NotarAI\n\nSome content here.\n')
  })

  it('extracts section between other ## headings', () => {
    const content = [
      '## Intro',
      '',
      'Intro text.',
      '',
      '## NotarAI',
      '',
      'NotarAI content.',
      '',
      '## Other',
      '',
      'Other content.',
    ].join('\n')
    const result = extractNotarAISection(content)
    expect(result).toBe('## NotarAI\n\nNotarAI content.\n')
  })

  it('returns empty string when not found', () => {
    const content = '## Intro\n\nNo notarai here.\n'
    const result = extractNotarAISection(content)
    expect(result).toBe('')
  })
})
