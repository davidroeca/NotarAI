# Progressive Adoption

You do not need full behavioral specs to get value from NotarAI. This guide describes three maturity levels. Most teams should aim for Level 2 on critical modules and Level 1 everywhere else.

## Level 1: Intent and artifacts only

The simplest useful spec. Maps files to a purpose without describing behaviors.

```yaml
schema_version: '0.7'
intent: >
  HTTP client library with retry logic, connection pooling,
  and timeout configuration.
tier: registered
artifacts:
  code:
    - path: 'src/http/**/*.rs'
      role: 'HTTP client implementation'
  tests:
    - path: 'tests/http_*.rs'
      role: 'HTTP client integration tests'
  docs:
    - path: 'docs/http.md'
      role: 'HTTP client usage guide'
```

**What you get at Level 1:**

- `notarai check` detects coverage gaps and orphaned globs
- Reconciliation surfaces which files changed and which spec governs them
- You have a searchable index of what each module owns

**When Level 1 is enough:** Utilities, configuration, internal tools, and any module where the intent is self-evident from the code.

## Level 2: Add behaviors for critical paths

Add `given`/`then` behavior descriptions for the paths that matter most: error handling, security boundaries, data validation, and user-facing features.

```yaml
schema_version: '0.7'
intent: >
  HTTP client library with retry logic, connection pooling,
  and timeout configuration.
behaviors:
  - name: retry_on_transient_failure
    given: 'a request fails with a 502, 503, or 429 status'
    then: 'retries up to 3 times with exponential backoff (1s, 2s, 4s)'

  - name: timeout_enforcement
    given: 'a request exceeds the configured timeout'
    then: 'aborts the request and returns a timeout error'

  - name: connection_pool_reuse
    given: 'multiple requests to the same host within the keep-alive window'
    then: 'reuses the existing TCP connection'
constraints:
  - 'All HTTP errors must be wrapped in a typed error enum'
  - 'Connection pool size must be configurable at initialization'
artifacts:
  code:
    - path: 'src/http/**/*.rs'
      role: 'HTTP client implementation'
  tests:
    - path: 'tests/http_*.rs'
      role: 'HTTP client integration tests'
```

**What you get at Level 2:**

- Everything from Level 1
- Reconciliation can detect when code behavior drifts from spec (e.g., retry logic changed but spec still says "3 retries")
- `notarai lint` flags incomplete behaviors (missing `given` or `then`)
- New team members can read the spec to understand intended behavior without reading the full implementation

**When to use Level 2:** Business logic, APIs, authentication, data pipelines, and anything where behavioral correctness matters.

## Level 3: Full SDD coverage

Add constraints, invariants, decisions, and cross-cutting concerns. This is the full power of the spec format.

```yaml
schema_version: '0.7'
intent: >
  HTTP client library with retry logic, connection pooling,
  and timeout configuration.
behaviors:
  - name: retry_on_transient_failure
    given: 'a request fails with a 502, 503, or 429 status'
    then: 'retries up to 3 times with exponential backoff (1s, 2s, 4s)'
  - name: timeout_enforcement
    given: 'a request exceeds the configured timeout'
    then: 'aborts the request and returns a timeout error'
constraints:
  - 'All HTTP errors must be wrapped in a typed error enum'
  - 'Connection pool size must be configurable at initialization'
  - 'No unbounded retries: retry count must have a hard cap'
invariants:
  - 'A timed-out request must never silently succeed'
  - 'Connection pool must never leak file descriptors'
decisions:
  - date: '2026-03-15'
    choice: 'Use ureq (blocking) instead of reqwest (async)'
    rationale: >
      Project is synchronous throughout. Adding tokio for HTTP
      alone adds 200KB to the binary and complicates error handling.
artifacts:
  code:
    - path: 'src/http/**/*.rs'
      role: 'HTTP client implementation'
  tests:
    - path: 'tests/http_*.rs'
      role: 'HTTP client integration tests'
```

**What you get at Level 3:**

- Everything from Level 2
- Invariants serve as hard constraints that reconciliation checks against
- Decisions capture the "why" behind architectural choices, preventing them from being unknowingly reversed
- `notarai lint` checks decision freshness and rationale completeness

**When to use Level 3:** Core system components, security-critical modules, and modules where architectural decisions are frequently revisited or questioned.

## Guidance

**Start at Level 1 for everything, then promote.** When you find yourself explaining a module's behavior to a teammate or an LLM, that is a signal to promote it to Level 2.

**Level 3 is not always better.** Over-specified specs create maintenance burden. A utility module with three functions does not need invariants and decisions. Match the spec depth to the module's complexity and risk.

**You do not need Level 3 to get value from NotarAI.** Most teams find that Level 2 on 5-10 critical modules plus Level 1 everywhere else provides the right balance.

See [Brownfield Adoption](./brownfield-adoption.md) for a step-by-step guide to getting started.
