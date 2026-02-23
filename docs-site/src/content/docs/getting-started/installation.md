---
title: Installation
description: How to install the NotarAI CLI.
---

## From npm

```sh
npm install -g notarai
```

Or use directly via npx:

```sh
npx notarai validate
```

## From source

```sh
git clone https://github.com/davidroeca/NotarAI
cd NotarAI
npm ci
npm run build
npm link
```

To uninstall:

```sh
npm uninstall -g notarai
```

## Requirements

- Node.js 18 or later
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) for reconciliation features (optional for validation-only usage)
