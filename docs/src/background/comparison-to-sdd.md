# Comparison to SDD

Spec-driven development (SDD) has emerged as a major pattern for AI-assisted coding, but the term covers several distinct approaches. [Birgitta Böckeler's taxonomy](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html) identifies three levels:

- **Spec-first:** Write a spec, generate code, discard or ignore the spec afterward.
- **Spec-anchored:** Keep the spec around for ongoing maintenance, but how it stays current is left vague.
- **Spec-as-source:** The spec _replaces_ code as the primary artifact. People never touch code directly.

Most SDD tools ([Kiro](https://kiro.dev/), [Spec Kit](https://github.com/github/spec-kit), [OpenSpec](https://github.com/Fission-AI/OpenSpec)) are spec-first in practice: they help you go from intent to plan to tasks to code, but once the code exists, the spec quietly goes stale. [Superpowers](https://github.com/obra/superpowers) takes the spec-first workflow further with a structured seven-stage methodology and subagent-driven execution, but its plans are task-scoped artifacts. [Tessl](https://docs.tessl.io/) is exploring spec-as-source, where code is generated from specs and marked "DO NOT EDIT", but this sacrifices the flexibility of direct code editing.

**NotarAI occupies the gap that Böckeler's taxonomy identifies but no current tool fills: spec-anchored with automated maintenance.** The spec persists for the lifetime of the feature, and an LLM reconciliation engine actively keeps it aligned with code and docs as all three evolve.

## SDD tools solve the cold-start problem. NotarAI solves the entropy problem.

SDD tools help you _write_ specs. NotarAI helps you _keep them true_.

- A developer adds a feature -- NotarAI detects the spec doesn't account for it and proposes an update
- A team lead updates the spec -- NotarAI propagates the change to code and docs
- Code contradicts a spec constraint -- NotarAI flags the conflict and asks the user to decide

The spec isn't just a blueprint. It's a **witness** -- a living contract the LLM continuously verifies against reality.

## Landscape comparison

| Tool                                                   | SDD Level                             | Direction                            | Spec Lifespan           | Brownfield Support                |
| ------------------------------------------------------ | ------------------------------------- | ------------------------------------ | ----------------------- | --------------------------------- |
| [**Kiro**](https://kiro.dev/)                          | Spec-first                            | Spec -> code                         | Change request          | Limited                           |
| [**Spec Kit**](https://github.com/github/spec-kit)     | Spec-first (aspires to anchored)      | Spec -> code                         | Branch / change request | Limited                           |
| [**Tessl**](https://docs.tessl.io/)                    | Spec-as-source                        | Spec -> code (human edits spec only) | Feature lifetime        | Reverse-engineering CLI           |
| [**OpenSpec**](https://github.com/Fission-AI/OpenSpec) | Spec-first                            | Spec -> code                         | Change request          | Limited                           |
| [**Superpowers**](https://github.com/obra/superpowers) | Spec-first (workflow methodology)     | Spec -> plan -> subagent execution   | Task / branch           | Git worktree isolation            |
| [**Semcheck**](https://semcheck.ai/)                   | Compliance checking                   | Spec -> code (one-way check)         | Ongoing                 | Yes                               |
| **NotarAI**                                            | Spec-anchored + active reconciliation | Spec <-> code <-> docs               | Feature lifetime        | Bootstrap flow with LLM interview |
