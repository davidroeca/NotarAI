// @ts-check
import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'
import mermaid from 'astro-mermaid'

// https://astro.build/config
export default defineConfig({
  site: 'https://davidroeca.github.io',
  base: '/NotarAI/docs',
  integrations: [
    mermaid({ autoTheme: true }),
    starlight({
      title: 'NotarAI',
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/davidroeca/NotarAI',
        },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'getting-started/installation' },
            { label: 'Quick Start', slug: 'getting-started/quick-start' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Spec Format Reference', slug: 'guides/spec-format' },
            { label: 'Reconciliation', slug: 'guides/reconciliation' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI Commands', slug: 'reference/cli' },
            { label: 'MCP Server', slug: 'reference/mcp-server' },
          ],
        },
        { label: 'Contributing', slug: 'contributing' },
        {
          label: 'Background',
          items: [
            { label: 'Motivation', slug: 'background/motivation' },
            {
              label: 'Comparison to SDD',
              slug: 'background/comparison-to-sdd',
            },
            {
              label: 'Design Diagrams',
              slug: 'background/design-diagrams',
            },
            { label: 'Inspirations', slug: 'background/inspirations' },
          ],
        },
      ],
    }),
  ],
})
