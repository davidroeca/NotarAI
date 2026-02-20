import { readFileSync } from 'node:fs'
import { relative } from 'node:path'
import { validateSpec } from '../lib/validator.js'

function isSpecFile(filePath: string): boolean {
  const rel = relative(process.cwd(), filePath)
  return rel.startsWith('.notarai/') && rel.endsWith('.spec.yaml')
}

export async function runHookValidate(): Promise<void> {
  let input = ''
  for await (const chunk of process.stdin) {
    input += chunk
  }

  let payload: { tool_input?: { file_path?: string } }
  try {
    payload = JSON.parse(input)
  } catch {
    process.exit(0)
  }

  const filePath = payload.tool_input?.file_path
  if (!filePath || !isSpecFile(filePath)) {
    process.exit(0)
  }

  let content: string
  try {
    content = readFileSync(filePath, 'utf-8')
  } catch {
    process.exit(0)
  }

  const result = validateSpec(filePath, content)

  if (result.valid) {
    process.exit(0)
  }

  console.error(`Spec validation failed: ${filePath}`)
  for (const err of result.errors) {
    console.error(`  ${err}`)
  }
  process.exit(1)
}
