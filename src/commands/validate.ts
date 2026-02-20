import { readFileSync, statSync, readdirSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { validateSpec } from '../lib/validator.js'

function findSpecFiles(dir: string): string[] {
  const results: string[] = []
  const entries = readdirSync(dir, { withFileTypes: true })
  for (const entry of entries) {
    const full = join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...findSpecFiles(full))
    } else if (entry.name.endsWith('.spec.yaml')) {
      results.push(full)
    }
  }
  return results
}

export function runValidate(args: string[]): void {
  const target = args[0] ?? '.notarai'
  const resolved = resolve(target)

  let files: string[]
  try {
    const stat = statSync(resolved)
    if (stat.isDirectory()) {
      files = findSpecFiles(resolved)
    } else {
      files = [resolved]
    }
  } catch {
    console.error(`Error: path not found: ${resolved}`)
    process.exit(1)
    return
  }

  if (files.length === 0) {
    console.error(`No .spec.yaml files found in ${resolved}`)
    process.exit(1)
  }

  let hasFailure = false

  for (const file of files) {
    const content = readFileSync(file, 'utf-8')
    const result = validateSpec(file, content)

    if (result.valid) {
      console.log(`PASS ${file}`)
    } else {
      hasFailure = true
      console.log(`FAIL ${file}`)
      for (const err of result.errors) {
        console.log(`  ${err}`)
      }
    }
  }

  process.exit(hasFailure ? 1 : 0)
}
