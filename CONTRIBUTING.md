# Contributing to NotarAI

Your interest in contributing to this project is appreciated. Below is a series of instructions that will hopefully remain up to date because this tool should help manage that. However, if you notice that the steps seem out of date or misaligned with current practices in the repo, an update to this document could be a and high-value first or second contribution to the project.

Note that the project's own spec drift is self-managed, so please get acquainted with the tool and make sure your contributions stay in sync.

## Development Setup

Install the LTS release of `nodejs`. Preferably the even-numbered LTS release. See [here](https://nodejs.org/en/about/previous-releases) for more details.

```sh
git clone https://github.com/davidroeca/NotarAI.git
cd NotarAI
npm ci
npm run build
```

## Making Changes

1. Create a branch from `main`
2. Make your changes
3. Run `npm run build` to verify the TypeScript compiles
4. Run `npx prettier --check .` to verify formatting
5. Use the `/notarai-reconcile` Claude Code command or use your favorite coding agent to follow these reconciliation instructions
6. Open a pull request

## Code Style

- TypeScript with strict mode
- 2-space indentation
- Prettier for formatting (`npx prettier --write .`)
- ES Modules (`import`/`export`) — no CommonJS
- Functional style preferred over classes
- All local imports use `.js` extensions (required by `module: "nodenext"`)

## Project Structure

See `CLAUDE.md` for a detailed layout and architectural constraints.

## Good First Contributions

These changes will drive broader adoption but are not yet a priority:

- Support other coding agents (e.g. Codex, Aider, Cline, OpenHands, Goose, opencode)
- Optimize/limit token usage with minimal quality loss

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
