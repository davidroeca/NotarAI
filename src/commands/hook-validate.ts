import { readFileSync } from 'node:fs'
import { relative } from 'node:path'
import { validateSpec } from '../lib/validator.js'

export interface HookResult {
  exitCode: number
  errors?: string[]
  filePath?: string
}

function isSpecFile(filePath: string, cwd: string): boolean {
  const rel = relative(cwd, filePath)
  return rel.startsWith('.notarai/') && rel.endsWith('.spec.yaml')
}

export function processHookInput(input: string, cwd: string): HookResult {
  let payload: { tool_input?: { file_path?: string } }
  try {
    payload = JSON.parse(input)
  } catch {
    return { exitCode: 0 }
  }

  const filePath = payload.tool_input?.file_path
  if (!filePath || !isSpecFile(filePath, cwd)) {
    return { exitCode: 0 }
  }

  let content: string
  try {
    content = readFileSync(filePath, 'utf-8')
  } catch {
    return { exitCode: 0 }
  }

  const result = validateSpec(filePath, content)

  if (result.valid) {
    return { exitCode: 0, filePath }
  }

  return { exitCode: 1, errors: result.errors, filePath }
}

export async function runHookValidate(): Promise<void> {
  let input = ''
  for await (const chunk of process.stdin) {
    input += chunk
  }

  const result = processHookInput(input, process.cwd())

  if (result.exitCode !== 0 && result.errors) {
    console.error(`Spec validation failed: ${result.filePath}`)
    for (const err of result.errors) {
      console.error(`  ${err}`)
    }
  }

  process.exit(result.exitCode)
}
