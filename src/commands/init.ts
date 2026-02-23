import {
  appendFileSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HOOK_COMMAND = 'npx notarai hook validate'

interface HookEntry {
  type: string
  command: string
}

interface HookMatcher {
  matcher: string
  hooks: HookEntry[]
}

interface Settings {
  hooks?: {
    PostToolUse?: HookMatcher[]
    [key: string]: unknown
  }
  [key: string]: unknown
}

function hasNotaraiHook(matchers: HookMatcher[]): boolean {
  return matchers.some((m) => m.hooks?.some((h) => h.command === HOOK_COMMAND))
}

/**
 * Extract the `## NotarAI` section from a markdown string.
 * Returns the section content (including the heading) from `## NotarAI`
 * up to the next `## ` heading or EOF. Returns empty string if not found.
 */
export function extractNotarAISection(content: string): string {
  const match = content.match(/(^|\n)(## NotarAI\n[\s\S]*?)(?=\n## |\s*$)/)
  if (!match) return ''
  const section = match[2]
  return section.trimEnd() + '\n'
}

export function runInit(projectRoot?: string): void {
  const root = projectRoot ?? process.cwd()
  const claudeDir = join(root, '.claude')
  const settingsPath = join(claudeDir, 'settings.json')

  if (!existsSync(claudeDir)) {
    mkdirSync(claudeDir, { recursive: true })
  }

  let settings: Settings = {}
  if (existsSync(settingsPath)) {
    try {
      settings = JSON.parse(readFileSync(settingsPath, 'utf-8')) as Settings
    } catch {
      console.error('Error: could not parse existing .claude/settings.json')
      process.exit(1)
    }
  }

  if (!settings.hooks) {
    settings.hooks = {}
  }

  if (!settings.hooks.PostToolUse) {
    settings.hooks.PostToolUse = []
  }

  if (hasNotaraiHook(settings.hooks.PostToolUse)) {
    console.log('NotarAI hook already configured in .claude/settings.json')
  } else {
    settings.hooks.PostToolUse.push({
      matcher: 'Write|Edit',
      hooks: [
        {
          type: 'command',
          command: HOOK_COMMAND,
        },
      ],
    })

    writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + '\n')
    console.log('Added NotarAI validation hook to .claude/settings.json')
  }

  setupReconcileCommand(claudeDir)
  setupBootstrapCommand(claudeDir)
  setupSchema(claudeDir)
  setupClaudeContext(root)
}

function setupClaudeContext(projectDir: string): void {
  const __dirname = dirname(fileURLToPath(import.meta.url))
  const templatePath = resolve(__dirname, '../../templates/claude-context.md')

  if (!existsSync(templatePath)) {
    console.error(
      'Warning: bundled claude-context.md not found, skipping CLAUDE.md setup',
    )
    return
  }

  const templateContent = readFileSync(templatePath, 'utf-8')
  const claudeMdPath = join(projectDir, 'CLAUDE.md')

  if (existsSync(claudeMdPath)) {
    const existing = readFileSync(claudeMdPath, 'utf-8')
    if (/(^|\n)## NotarAI/.test(existing)) {
      const existingSection = extractNotarAISection(existing)
      const templateSection = extractNotarAISection(templateContent)
      if (existingSection !== templateSection) {
        console.warn(
          'Warning: the ## NotarAI section in CLAUDE.md has drifted from the bundled template. ' +
            'Review manually, or delete the section and re-run `notarai init` to replace it.',
        )
      } else {
        console.log('NotarAI context already present in CLAUDE.md')
      }
      return
    }
    appendFileSync(claudeMdPath, '\n\n' + templateContent)
  } else {
    writeFileSync(claudeMdPath, templateContent)
  }

  console.log('Added NotarAI context to CLAUDE.md')
}

function setupReconcileCommand(claudeDir: string): void {
  const commandsDir = join(claudeDir, 'commands')
  const destPath = join(commandsDir, 'notarai-reconcile.md')

  if (existsSync(destPath)) {
    console.log(
      'Reconcile command already exists at .claude/commands/notarai-reconcile.md',
    )
    return
  }

  const __dirname = dirname(fileURLToPath(import.meta.url))
  const srcPath = resolve(__dirname, '../../commands/notarai-reconcile.md')

  if (!existsSync(srcPath)) {
    console.error(
      'Warning: bundled notarai-reconcile.md not found, skipping command setup',
    )
    return
  }

  mkdirSync(commandsDir, { recursive: true })
  copyFileSync(srcPath, destPath)
  console.log(
    'Added /notarai-reconcile command to .claude/commands/notarai-reconcile.md',
  )
}

function setupSchema(claudeDir: string): void {
  const __dirname = dirname(fileURLToPath(import.meta.url))
  const srcPath = resolve(__dirname, '../../notarai.spec.json')

  if (!existsSync(srcPath)) {
    console.error(
      'Warning: bundled notarai.spec.json not found, skipping schema setup',
    )
    return
  }

  copyFileSync(srcPath, join(claudeDir, 'notarai.spec.json'))
  console.log('Copied schema to .claude/notarai.spec.json')
}

function setupBootstrapCommand(claudeDir: string): void {
  const commandsDir = join(claudeDir, 'commands')
  const destPath = join(commandsDir, 'notarai-bootstrap.md')

  if (existsSync(destPath)) {
    console.log(
      'Bootstrap command already exists at .claude/commands/notarai-bootstrap.md',
    )
    return
  }

  const __dirname = dirname(fileURLToPath(import.meta.url))
  const srcPath = resolve(__dirname, '../../commands/notarai-bootstrap.md')

  if (!existsSync(srcPath)) {
    console.error(
      'Warning: bundled notarai-bootstrap.md not found, skipping command setup',
    )
    return
  }

  mkdirSync(commandsDir, { recursive: true })
  copyFileSync(srcPath, destPath)
  console.log(
    'Added /notarai-bootstrap command to .claude/commands/notarai-bootstrap.md',
  )
}
