// @ts-check
import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'

// https://astro.build/config
export default defineConfig({
  site: 'https://davidroeca.github.io',
  base: '/NotarAI',
  integrations: [
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
          ],
        },
      ],
    }),
  ],
})
