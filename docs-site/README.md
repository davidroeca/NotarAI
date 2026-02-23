# NotarAI Documentation Site

Built with [Astro Starlight](https://starlight.astro.build). Deployed to GitHub Pages at `https://davidroeca.github.io/NotarAI/`.

## Development

From the repository root (npm workspaces):

```sh
# Install all dependencies
npm ci

# Start dev server at localhost:4321
npm run dev --workspace=docs-site

# Build for production
npm run build --workspace=docs-site

# Preview production build
npm run preview --workspace=docs-site
```

## Content

Pages live in `src/content/docs/` as Markdown or MDX files. The sidebar is configured in `astro.config.mjs`.

```
src/content/docs/
  index.mdx                           # Landing page
  getting-started/
    installation.md
    quick-start.md
  guides/
    spec-format.md                    # Spec format reference
    reconciliation.md                 # How reconciliation works
  background/
    motivation.md                     # Why NotarAI exists
    comparison-to-sdd.md             # Comparison to SDD tools
    design-diagrams.md               # Architecture diagrams
```

Images go in `src/assets/`. Static files (like a CNAME for custom domains) go in `public/`.
