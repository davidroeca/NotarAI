# Specs vs Claude Rules

NotarAI specs and Claude rules (CLAUDE.md / `.claude/rules/`) both express
project conventions, but they serve different purposes and trigger at different
times. This guide explains when to use each -- and when to use both.

## Decision framework

| Use a **spec** when...                                | Use a **Claude rule** when...                      |
| ----------------------------------------------------- | -------------------------------------------------- |
| The concern describes _what artifacts must look like_ | The concern describes _how Claude should work_     |
| You want reconciliation to detect drift retroactively | You want to prevent violations proactively         |
| The rule maps to files you can diff against           | The rule is about process, workflow, or tool usage |
| Cross-cutting specs (`applies`) can propagate it      | The convention only matters during generation      |

## Use a spec

Specs are the right home for **artifact-facing rules** -- invariants,
constraints, and behaviors that describe what code, docs, or configs should
look like. The reconciliation engine diffs artifacts against these rules and
proposes fixes when they drift.

Examples from this project:

- "American English throughout" -- `style.spec.yaml` catches existing files
  that use British spellings
- "The engine must never silently auto-modify code" -- an invariant in
  `system.spec.yaml` that reconciliation checks against code changes
- "CLI validates spec files against bundled JSON Schema" -- a behavior in
  `cli.spec.yaml` tied to source files

Cross-cutting specs (referenced via `applies` in the system spec) propagate
invariants and constraints across all subsystems without duplication.

## Use a Claude rule

Claude rules are the right home for **workflow-facing instructions** -- how
Claude should run commands, what tools to prefer, what process to follow. These
have no artifact to reconcile against; they shape how Claude works, not what
the output looks like.

Examples:

- "Tests use `cargo test`" -- tells Claude which command to run
- "When bumping schema version, update these five files" -- a checklist for
  a multi-step process
- "Unit tests are inline `#[cfg(test)]` modules" -- convention for where to
  put new tests

These belong in `.claude/rules/` files (or CLAUDE.md) because there is no
meaningful way to diff project files against them.

## Use both

Some conventions benefit from **both proactive prevention and retroactive
detection**. Style rules are the classic example:

- **Claude rule** prevents new violations: Claude follows the rule as it
  generates code, so new files are correct from the start.
- **Spec** catches existing drift: reconciliation scans all governed files
  and flags violations that predate the rule or were introduced by humans.

This is intentional duplication, not redundancy. The two mechanisms cover
different failure modes.

Examples from this project:

| Convention                | Claude rule              | Spec              |
| ------------------------- | ------------------------ | ----------------- |
| American English          | `.claude/rules/style.md` | `style.spec.yaml` |
| QWERTY-typable characters | `.claude/rules/style.md` | `style.spec.yaml` |

## Anti-patterns

**Don't put process instructions in specs.** A spec behavior like "given a
schema version bump, then update these five files" has no artifact to diff
against. It belongs in a Claude rule or checklist.

**Don't put formal behavioral specs in Claude rules.** A rule like "the CLI
must validate spec files against the bundled schema" is a testable behavior.
If it lives only in CLAUDE.md, reconciliation can't detect when code drifts
away from it.

**Don't duplicate without purpose.** If a convention only needs proactive
prevention (e.g., "run prettier on generated code"), a Claude rule is
sufficient. If it only needs retroactive detection (e.g., "no circular $ref
chains"), a spec invariant is sufficient. Use both only when both failure
modes are real.
