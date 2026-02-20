import Ajv, { type ErrorObject } from 'ajv'
import addFormats from 'ajv-formats'
import { loadSchema } from './schema.js'
import { parseYaml } from './yaml.js'

export interface ValidationResult {
  valid: boolean
  errors: string[]
}

const ajv = new (Ajv as unknown as typeof Ajv.default)({
  allErrors: true,
  strict: false,
})
;(addFormats as unknown as typeof addFormats.default)(ajv)

const schema = loadSchema()
const validate = ajv.compile(schema)

export function validateSpec(
  _filePath: string,
  content: string,
): ValidationResult {
  const parsed = parseYaml(content)

  if (!parsed.ok) {
    return {
      valid: false,
      errors: [`YAML parse error: ${parsed.error}`],
    }
  }

  const valid = validate(parsed.data)

  if (valid) {
    return { valid: true, errors: [] }
  }

  const errors = (validate.errors ?? []).map((err: ErrorObject) => {
    const path = err.instancePath || '/'
    return `${path}: ${err.message}`
  })

  return { valid: false, errors }
}
