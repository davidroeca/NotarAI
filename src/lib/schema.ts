import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const schemaPath = resolve(__dirname, '../../notarai.spec.json')

export function loadSchema(): Record<string, unknown> {
  const raw = readFileSync(schemaPath, 'utf-8')
  return JSON.parse(raw) as Record<string, unknown>
}
