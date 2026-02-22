#!/usr/bin/env node

import { runValidate } from './commands/validate.js'
import { runInit } from './commands/init.js'
import { runHookValidate } from './commands/hook-validate.js'

const USAGE = `Usage: notarai <command>

Commands:
  validate [file|dir]   Validate spec files (default: .notarai/)
  init                  Set up NotarAI in a project (hook, slash commands, schema, CLAUDE.md context)
  hook validate         Validate spec from Claude Code hook stdin

Options:
  --help                Show this help message`

const args = process.argv.slice(2)
const command = args[0]

switch (command) {
  case 'validate':
    runValidate(args.slice(1))
    break
  case 'init':
    runInit()
    break
  case 'hook':
    if (args[1] === 'validate') {
      runHookValidate()
    } else {
      console.error(USAGE)
      process.exit(1)
    }
    break
  case '--help':
  case '-h':
    console.log(USAGE)
    break
  default:
    console.error(USAGE)
    process.exit(1)
}
