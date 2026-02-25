import js from '@eslint/js'
import astro from 'eslint-plugin-astro'
import mdx from 'eslint-plugin-mdx'

export default [
  js.configs.recommended,
  ...astro.configs.recommended,
  ...mdx.flat,
]
