import { fixupPluginRules } from '@eslint/compat'
import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import astro from 'eslint-plugin-astro'
import * as mdx from 'eslint-plugin-mdx'
import prettier from 'eslint-config-prettier'

export default [
  // Base JS rules
  js.configs.recommended,

  // TypeScript rules
  ...tseslint.configs.recommended,

  // Astro rules
  ...astro.configs.recommended,

  // MDX rules
  {
    ...mdx.flat,
    // https://github.com/mdx-js/eslint-mdx/issues/604#issuecomment-3928607833
    plugins: {
      mdx: fixupPluginRules(mdx.flat.plugins?.mdx ?? {}),
    },
  },
  prettier,

  // Project-specific overrides
  {
    rules: {
      // Good defaults
      'no-unused-vars': 'off', // handled by TS
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],

      // These are overly noisy in real projects
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
    },
  },
  {
    files: ['**/*.mdx'],
    rules: {
      // this rule gives false positives in mdx if markdown is between an import and some jsx
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },
  // Ignore generated/build files
  {
    ignores: [
      '**/dist/',
      '**/gh-pages',
      '**/build/',
      '**/.astro/',
      '**/node_modules/',
    ],
  },
]
