import { load } from 'js-yaml'

export interface YamlResult {
  ok: true
  data: unknown
}

export interface YamlError {
  ok: false
  error: string
}

export function parseYaml(content: string): YamlResult | YamlError {
  try {
    const data = load(content)
    return { ok: true, data }
  } catch (err) {
    const message =
      err instanceof Error ? err.message : 'Unknown YAML parse error'
    return { ok: false, error: message }
  }
}
