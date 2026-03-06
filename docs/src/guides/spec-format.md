# Spec Format Reference

Specs are YAML files validated against a JSON Schema (`notarai.spec.json`). The format uses progressive disclosure: a small set of required fields for minimum viability, with optional fields for precision as needed.

## Required fields

### `schema_version`

Pins the JSON Schema version. Current version: `"0.7"`. Versions `"0.6"` and `"0.5"` are also accepted for backward compatibility.

```yaml
schema_version: '0.7'
```

### `intent`

Natural language description of what the system or feature should do.

```yaml
intent: |
  Users can sign up, log in, and reset passwords.
  Sessions expire after 30 min of inactivity.
```

### `behaviors`

Structured Given/Then entries describing expected behavior. Each behavior has a `name`, a `given` condition, and a `then` outcome. Required for `full` tier specs; optional for `registered` and `derived` tier specs.

```yaml
behaviors:
  - name: 'signup'
    given: 'valid email + password (>= 12 chars)'
    then: 'account created, welcome email sent'
  - name: 'session_timeout'
    given: '30 min inactivity'
    then: 'session invalidated'
```

Behaviors may also include optional `interaction` and `state_transition` sub-fields:

```yaml
behaviors:
  - name: 'submit_form'
    given: 'user submits a valid form'
    then: 'data saved, confirmation shown'
    interaction:
      trigger: user_action # user_action | timer | system_event | data_change | schedule | external_signal | threshold | manual | lifecycle
      sequence:
        - validate fields
        - post to API
        - show confirmation
    state_transition:
      from: editing
      to: confirmed
```

### `artifacts`

Glob patterns mapping the spec to the files it governs. The schema accepts any string as a category key. Convention categories:

| Category    | When to use             |
| ----------- | ----------------------- |
| `code`      | Source code             |
| `docs`      | Documentation           |
| `tests`     | Test files              |
| `slides`    | Presentation files      |
| `data`      | Data files, CSVs        |
| `configs`   | Configuration, IaC      |
| `notebooks` | Jupyter/R notebooks     |
| `assets`    | Media, images, fonts    |
| `templates` | Reusable templates      |
| `schemas`   | Data schemas, API specs |

```yaml
artifacts:
  code:
    - path: 'src/auth/**'
      role: 'primary implementation'
  docs:
    - path: 'docs/auth.md'
  tests:
    - path: 'tests/auth/**'
```

Each artifact ref may include an optional integer `tier` override (1-4) for files that belong to a different tier than the spec itself:

```yaml
artifacts:
  code:
    - path: 'dist/bundle.js'
      tier: 4 # derived output — tracked for staleness, not authored directly
```

---

## Optional fields

### `constraints`

Rules the system must follow.

```yaml
constraints:
  - 'rate limit: 5 login attempts per minute per IP'
  - 'passwords must be >= 12 characters'
```

### `invariants`

Conditions that must never be violated.

```yaml
invariants:
  - 'no plaintext passwords stored anywhere'
  - 'all API responses include request-id header'
```

### `decisions`

Architectural decision log with date, choice, and rationale.

```yaml
decisions:
  - date: '2025-01-15'
    choice: 'JWT over session cookies'
    rationale: 'Stateless auth simplifies horizontal scaling'
```

### `open_questions`

Unresolved design questions.

```yaml
open_questions:
  - 'Should we support OAuth providers beyond Google?'
  - "What's the session timeout for mobile clients?"
```

### `dependencies`

References to other specs this one interacts with.

```yaml
dependencies:
  - $ref: 'billing.spec.yaml'
    relationship: 'auth gates billing endpoints'
```

### `notes`

Freeform hints for the LLM about implicit relationships.

```yaml
notes: |
  The auth module shares a rate limiter with the API gateway.
  Session storage is Redis in production, in-memory in dev.
```

### `output`

Describes what the spec ultimately produces. Useful for non-software artifacts like presentations or reports.

```yaml
output:
  type: presentation # app | presentation | interactive-doc | game | dashboard | report | library | service | document | course | api | infrastructure | dataset | design-system | campaign | template
  format: pptx
  runtime: static-file # browser | native | static-file | embedded | server
  entry_point: dist/deck.pptx
```

### `content`

Describes the output's logical structure in content terms (slides, scenes, sections) rather than file terms.

```yaml
content:
  structure: graph # ordered | hierarchical | graph | free-form
  sections:
    - id: level_1
      type: scene
      intent: 'Tutorial level introducing movement mechanics'
      duration: { value: 5, unit: minutes }
      connections:
        - to: level_2
          label: completion
        - to: game_over
          label: player_death
      depends_on:
        - id: intro_cutscene
          relationship: 'must complete before this section unlocks'
      evidence:
        - type: data
          source: playtests/run_3.csv
          claim: '85% of players complete within 5 minutes'
```

### `states`

Top-level state machine definition for interactive artifacts.

```yaml
states:
  initial: idle
  definitions:
    - id: idle
      transitions:
        - to: running
          on: start
          guard: 'all required fields are populated'
          action: 'initialize timer, log start event'
    - id: running
      transitions:
        - to: idle
          on: stop
```

### `design`

Visual and design specifications for brand-governed artifacts.

```yaml
design:
  theme:
    palette: ['#1a1a2e', '#16213e']
    typography:
      heading: Inter
      body: Roboto
    modes:
      light: { palette: ['#ffffff', '#f0f0f0'] }
      dark: { palette: ['#1a1a2e', '#16213e'] }
  layout:
    type: paginated # slide-deck | scrolling | spatial | grid | free-form | paginated | canvas | timeline | tabbed
    dimensions: letter
  print:
    margins: { top: '1in', right: '1in', bottom: '1in', left: '1in' }
    headers: true
    footers: true
    page_numbers: true
  responsive:
    breakpoints:
      - name: mobile
        max_width: 768
        layout_override: scrolling
      - name: desktop
        min_width: 769
```

### `audience`

Context about who the output is for.

```yaml
audience:
  role: 'Series B investors'
  assumed_knowledge: 'Familiar with SaaS metrics, not technical infrastructure'
  tone: formal-but-engaging
  locale: en-US
  accessibility:
    - high-contrast
    - screen-reader-friendly
```

### `variants`

Multiple versions of the same artifact with selective field overrides.

```yaml
variants:
  - id: investor-deck
    description: 'Condensed version for investor meetings'
    overrides:
      audience.role: 'Series B investors'
  - id: engineering-deep-dive
    description: 'Full technical version for the eng team'
```

Variants are declarative metadata by default. Set `variants_resolved: true` at the spec top level to opt in to programmatic override resolution (scalar replacement, array replacement with `+` prefix for append, deep merge for objects, `null` to clear).

### `pipeline`

Describes the build or generation process for the output artifact.

```yaml
pipeline:
  env:
    NODE_ENV: production
  steps:
    - name: compile
      tool: tsc
      input: 'src/**/*.ts'
      output: dist/
      condition: "output.format == 'web'"
    - name: export_pdf
      command: 'pandoc input.md -o output.pdf'
      condition: "output.format == 'pdf'"
      on_failure: skip
      depends_on: [compile]
      env:
        PANDOC_DATA_DIR: ./templates
  preview:
    command: npx serve dist/
    url: 'http://localhost:3000'
```

### `feedback`

Connects output performance metrics back to the spec for reconciliation triggers.

```yaml
feedback:
  metrics:
    - name: avg_completion_rate
      source: analytics/completion.csv
      threshold: '>= 0.7'
    - name: build_time
      threshold: '< 5s'
  triggers:
    - condition:
        metric: avg_completion_rate
        operator: below_threshold
        duration: { value: 3, unit: days }
      action: reconcile
      priority: high
```

Note: `reconciliation_trigger` (free-form string) is deprecated in favor of `triggers` but still accepted.

### `compliance`

Maps invariants and constraints to regulatory or standards frameworks. The reconciliation engine verifies that framework-required invariants still exist in the spec.

```yaml
compliance:
  frameworks:
    - name: SOC2
      controls:
        - id: CC6.1
          satisfied_by:
            invariants: ['no plaintext passwords stored anywhere']
            constraints: ['rate limit: 5 login attempts per minute per IP']
    - name: WCAG
      level: AA
      satisfied_by:
        invariants: ['all interactive elements have visible focus indicators']
  audit_trail: true
```

---

## Coverage tiers

Every file in the repo falls into one of four tiers:

- **Tier 1 (Full)** — Business logic, APIs, user-facing features. Full behavioral specification required.
- **Tier 2 (Registered)** — Utilities, config, sidecars. Intent and artifact mapping only; `behaviors` not required.
- **Tier 3 (Excluded)** — Explicitly out of scope. Declared via `exclude` globs on the system spec.
- **Tier 4 (Derived)** — Generated outputs tracked for staleness but not authored directly (e.g., build artifacts, compiled bundles). Use `tier: derived` on the spec or `tier: 4` on individual artifact refs.

Files not covered by any tier are flagged as "unspecced" — a lint warning, not a blocker.

Set the spec-level tier with the `tier` field:

```yaml
tier: registered # full (default) | registered | derived
```

---

## Composition

Specs compose via `$ref` (borrowed from JSON Schema/OpenAPI):

- `subsystems` — hierarchical references (system → services)
- `applies` — cross-cutting specs (e.g., security, logging) that apply to all subsystems

A top-level `system.spec.yaml` serves as the manifest, referencing subsystem specs and declaring exclusion patterns for Tier 3 files.
