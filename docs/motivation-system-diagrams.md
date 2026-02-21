# NotarAI — Design Diagrams

All diagrams from the design process, updated to reflect the NotarAI name and `.notarai/` directory convention.

---

## 1. The Problem: Pre-LLM vs Current LLM Era

### 1a. Pre-LLM: Code Is the Spec

```mermaid
flowchart LR
    Dev["Developer<br/>(intent in head)"]
    Code["Source Code<br/>**authoritative spec**"]
    Docs["Docs<br/>second-class, often stale"]

    Dev -->|writes| Code
    Code -.->|describes| Docs

    style Code fill:#1a2a1a,stroke:#4a9,color:#4a9
    style Dev fill:#1a1d27,stroke:#444,color:#ccc
    style Docs fill:#1a1d27,stroke:#444,color:#ccc
```

### 1b. Current LLM Era: The Three-Body Problem

```mermaid
flowchart TD
    Intent["User Intent<br/>natural language prompt"]
    LLM["LLM"]
    Code["Source Code"]
    Docs["Documentation"]

    Intent --> LLM
    Intent -.->|"edits directly"| Code
    Intent -.->|"edits directly"| Docs
    LLM -->|generates| Code
    LLM -->|generates| Docs
    Code <-..->|"⚠ drift / desync"| Docs

    style Intent fill:#1a1d27,stroke:#444,color:#ccc
    style LLM fill:#2a2040,stroke:#8b6fc0,color:#b89eed
    style Code fill:#1a2a1a,stroke:#4a9,color:#4a9
    style Docs fill:#1e1e2e,stroke:#5588bb,color:#5588bb

    linkStyle 1 stroke:#e05555,stroke-dasharray:5 5
    linkStyle 2 stroke:#e05555,stroke-dasharray:5 5
    linkStyle 5 stroke:#e05555,stroke-dasharray:5 5
```

---

## 2. NotarAI: Spec State File as Single Source of Truth

```mermaid
flowchart TD
    Intent["User Intent<br/>natural language"]
    Spec["**NotarAI Spec**<br/>structured intent representation<br/>canonical source of truth"]
    LLM["LLM (sync engine)"]
    Code["Source Code"]
    Docs["Documentation"]

    Intent -->|updates| Spec
    Spec -->|reads| LLM
    LLM -->|derives| Code
    LLM -->|derives| Docs
    Code -.->|reconcile back| Spec
    Docs -.->|reconcile back| Spec
    Code <-.->|"✓ always in sync via spec"| Docs

    style Intent fill:#1a1d27,stroke:#444,color:#ccc
    style Spec fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style LLM fill:#2a2040,stroke:#8b6fc0,color:#b89eed
    style Code fill:#1a2a1a,stroke:#4a9,color:#4a9
    style Docs fill:#1e1e2e,stroke:#5588bb,color:#5588bb

    linkStyle 4 stroke:#4a9,stroke-dasharray:5 5
    linkStyle 5 stroke:#4a9,stroke-dasharray:5 5
    linkStyle 6 stroke:#4a9,stroke-dasharray:4 4
```

---

## 3. Spec File Anatomy

### 3a. Required Core

```yaml
# .notarai/auth.spec.yaml
schema_version: "0.3"

intent: |
  Users can sign up, log in, and
  reset passwords. Sessions expire
  after 30 min of inactivity.

behaviors:
  - name: "signup"
    given: "valid email + password"
    then: "account created, welcome email sent"
  - name: "session_timeout"
    given: "30 min inactivity"
    then: "session invalidated"

artifacts:
  code:
    - path: "src/auth/**"
  docs:
    - path: "docs/auth.md"
```

### 3b. Optional Extensions

```yaml
# Power users add precision as needed

constraints:
  - "passwords >= 12 chars"
  - "rate limit: 5 login attempts / min"

invariants:
  - "no plaintext passwords in DB"
  - "all endpoints require HTTPS"

decisions:
  - date: "2025-03-12"
    choice: "JWT over session cookies"
    rationale: "stateless scaling"

open_questions:
  - "Should we support OAuth2 providers?"
  - "MFA timeline?"

sync_policy:
  on_code_change: "propose_spec_update"
  on_spec_change: "update_code_and_docs"
```

> **Design note:** The `behaviors` field uses Given/Then language (BDD-adjacent) but stays in natural language — not formal Gherkin. Structured enough to diff and validate, informal enough that non-engineers can author it.

---

## 4. Reconciliation Lifecycle

### 4a. Scenario A: Human Edits Code

```mermaid
flowchart LR
    A1["Human edits code<br/>adds OAuth endpoint"]
    A2["LLM detects drift<br/>code ≠ spec behaviors"]
    A3["LLM proposes spec update<br/>+ add behavior: oauth_login<br/>+ update docs/auth.md"]
    A4["Human approves<br/>or adjusts & approves"]

    A1 -->|trigger| A2
    A2 -->|reconcile| A3
    A3 -->|resolve| A4

    style A1 fill:#1a2a1a,stroke:#4a9,color:#4a9
    style A2 fill:#161820,stroke:#333,color:#ccc
    style A3 fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style A4 fill:#2a2040,stroke:#8b6fc0,color:#b89eed
```

### 4b. Scenario B: Human Edits Spec

```mermaid
flowchart LR
    B1["Human edits spec<br/>changes session → 60 min"]
    B2["LLM updates code to match"]
    B3["LLM updates docs to match"]
    B4["Human reviews<br/>code + docs diff<br/>as a single PR"]

    B1 -->|direct| B2
    B1 -->|direct| B3
    B2 --> B4
    B3 --> B4

    style B1 fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style B2 fill:#1a2a1a,stroke:#4a9,color:#4a9
    style B3 fill:#1e1e2e,stroke:#5588bb,color:#5588bb
    style B4 fill:#2a2040,stroke:#8b6fc0,color:#b89eed
```

### 4c. Scenario C: Conflict Detected

```mermaid
flowchart LR
    C1["Conflict detected<br/>code says X, spec says Y<br/>docs say Z"]
    C2["LLM presents options<br/>spec says X, but code<br/>does Y — which is right?"]
    C3["Human decides intent<br/>LLM propagates decision<br/>across spec + code + docs"]
    C4["All three aligned<br/>conflict resolved"]

    C1 -->|detect| C2
    C2 -->|reconcile| C3
    C3 -->|resolve| C4

    style C1 fill:#2a1515,stroke:#e05555,color:#e05555
    style C2 fill:#161820,stroke:#333,color:#ccc
    style C3 fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style C4 fill:#1a2a1a,stroke:#4a9,color:#4a9
```

---

## 5. Sync Timing Strategies

**A. Spec-First ⚠ RISKY**

```mermaid
flowchart LR
    A1["User says 'add OAuth'"] --> A2["LLM updates spec first"]
    A2 --> A3["User approves spec"]
    A3 --> A4["LLM writes code"]

    style A1 fill:#1e1a1a,stroke:#665555,color:#ccc
    style A2 fill:#1e1a1a,stroke:#665555,color:#ccc
    style A3 fill:#1e1a1a,stroke:#665555,color:#ccc
    style A4 fill:#1e1a1a,stroke:#665555,color:#ccc
```

**B. Post-Push Reconciliation ✓ RECOMMENDED**

```mermaid
flowchart LR
    B1["Dev + LLM write code freely"] --> B2["git push / open PR"]
    B2 --> B3["CI hook: LLM reviews<br/>diff ∩ affected specs"]
    B3 --> B4["Adds spec + doc updates to PR"]
    B4 --> B5["Single review:<br/>code + spec + docs"]

    style B1 fill:#131e18,stroke:#2a4a3a,color:#ccc
    style B2 fill:#131e18,stroke:#2a4a3a,color:#ccc
    style B3 fill:#131e18,stroke:#2a4a3a,color:#ccc
    style B4 fill:#131e18,stroke:#2a4a3a,color:#ccc
    style B5 fill:#131e18,stroke:#2a4a3a,color:#ccc
```

**C. Ambient Awareness — BALANCED**

```mermaid
flowchart LR
    C1["LLM reads spec as context<br/>(like CLAUDE.md)"] --> C2["LLM writes code<br/>spec-informed, no friction"]
    C2 --> C3["Full reconciliation at push"]

    style C1 fill:#1a1a28,stroke:#3a3a5a,color:#ccc
    style C2 fill:#1a1a28,stroke:#3a3a5a,color:#ccc
    style C3 fill:#1a1a28,stroke:#3a3a5a,color:#ccc
```

> **Recommendation:** Start with **B** (post-push) as the default — lowest friction, easiest to adopt. Design the spec format to support **C** (ambient awareness) as the long-term target. The `sync_policy` field lets teams opt into different strategies per spec.

---

## 6. Post-Push Reconciliation in Practice (Git Integration)

```mermaid
flowchart LR
    S1["Dev + LLM<br/>write code freely<br/>no spec friction"]
    S2["git push<br/>or open PR"]
    S3["CI hook: LLM reviews<br/>diff ∩ affected specs<br/>→ proposes spec updates<br/>→ proposes doc updates"]
    S4["Adds to PR<br/>spec diff + docs diff<br/>alongside code diff"]
    S5["Single review<br/>code + spec + docs<br/>all land together or not"]

    S1 --> S2 --> S3 --> S4 --> S5

    style S1 fill:#1a2a1a,stroke:#4a9,color:#4a9
    style S2 fill:#161820,stroke:#555,color:#ccc
    style S3 fill:#2a2040,stroke:#8b6fc0,color:#b89eed
    style S4 fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style S5 fill:#131e18,stroke:#4a9,color:#4a9
```

> The `artifacts` field in the spec tells the CI hook which specs are affected by which file paths — so it only reconciles what changed.

---

## 7. Spec Composition — The Import Model

### 7a. Directory Structure

```
project/
├── .notarai/
│   ├── system.spec.yaml          # top-level system spec
│   ├── auth.spec.yaml            # auth service (Tier 1)
│   ├── billing.spec.yaml         # billing service (Tier 1)
│   ├── api.spec.yaml             # API layer (Tier 1)
│   ├── utils.spec.yaml           # shared utilities (Tier 2)
│   ├── redis-cache.spec.yaml     # sidecar process (Tier 2)
│   └── _shared/
│       ├── security.spec.yaml    # cross-cutting
│       └── logging.spec.yaml     # cross-cutting
├── src/
│   ├── auth/
│   ├── billing/
│   └── api/
└── docs/
```

### 7b. Composition Relationships

```mermaid
flowchart TD
    System["**system.spec.yaml**<br/>top-level intent + invariants"]

    Auth[".notarai/auth.spec.yaml"]
    Billing[".notarai/billing.spec.yaml"]
    API[".notarai/api.spec.yaml"]

    Security["_shared/security.spec.yaml<br/>applies to: all subsystems"]
    Logging["_shared/logging.spec.yaml<br/>applies to: all subsystems"]

    System -->|"$ref"| Auth
    System -->|"$ref"| Billing
    System -->|"$ref"| API

    Security -.->|applies| Auth
    Security -.->|applies| Billing
    Security -.->|applies| API
    Logging -.->|applies| Auth
    Logging -.->|applies| Billing
    Logging -.->|applies| API

    style System fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style Auth fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style Billing fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style API fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style Security fill:#1e1e2e,stroke:#5588bb,color:#5588bb
    style Logging fill:#1e1e2e,stroke:#5588bb,color:#5588bb

    linkStyle 3 stroke:#5588bb,stroke-dasharray:5 5
    linkStyle 4 stroke:#5588bb,stroke-dasharray:5 5
    linkStyle 5 stroke:#5588bb,stroke-dasharray:5 5
    linkStyle 6 stroke:#5588bb,stroke-dasharray:5 5
    linkStyle 7 stroke:#5588bb,stroke-dasharray:5 5
    linkStyle 8 stroke:#5588bb,stroke-dasharray:5 5
```

> When the LLM checks `auth.spec.yaml`, it also loads `security.spec.yaml` and validates that auth code satisfies **both** specs' invariants. Cross-cutting concerns are defined once and enforced everywhere.

---

## 8. Coverage Model — Three Tiers

```mermaid
flowchart LR
    subgraph T1["Tier 1: Full Spec"]
        T1a["Business logic services"]
        T1b["API endpoints"]
        T1c["Data models / schemas"]
        T1d["Anything user-facing"]
    end

    subgraph T2["Tier 2: Registered"]
        T2a["Utility libraries"]
        T2b["Shared helpers / constants"]
        T2c["Config files"]
        T2d["Sidecar processes"]
    end

    subgraph T3["Tier 3: Excluded"]
        T3a["Generated code / build output"]
        T3b["Vendored dependencies"]
        T3c["IDE / editor configs"]
        T3d["node_modules, .git, etc."]
    end

    style T1 fill:#1e2a1a,stroke:#4a9,color:#4a9
    style T2 fill:#1e1e28,stroke:#5588bb,color:#5588bb
    style T3 fill:#1e1a1a,stroke:#665555,color:#998877
```

**Coverage equation:** `Tier 1 + Tier 2 + Tier 3 = entire repo`

Anything not covered = **unspecced** (a lint warning, not a block).

### Coverage Resolution (Zero Context Cost)

```
# At reconciliation time, the LLM:

1. Collects all artifact globs from every referenced spec (recursive)
   → covered

2. Collects all exclude patterns from system.spec.yaml
   → excluded

3. Lists all files in repo
   → git ls-files

4. Computes the gap
   → unspecced = all_files - covered - excluded

5. If unspecced is non-empty:
   → "These files aren't governed by any spec:
      - src/notifications/email.ts
      - src/notifications/sms.ts
      Assign to a spec or exclude?"

# Cost: ONE shell command + set math
# No file contents loaded — zero context window impact
```

---

## 9. Bootstrap Flow for Existing Codebases

```mermaid
flowchart LR
    S1["1. Ingest<br/>code + docs +<br/>commit history +<br/>README / ADRs"]
    S2["2. LLM interviews<br/>What's the goal?<br/>Any undocumented rules?"]
    S3["3. Draft spec<br/>required fields only<br/>intent + behaviors +<br/>artifact mappings"]
    S4["4. Human review<br/>correct, enrich,<br/>add constraints /<br/>open questions"]
    S5["5. Activate<br/>sync engine<br/>watches for drift<br/>from this point on"]

    S1 --> S2 --> S3 --> S4 --> S5

    style S1 fill:#161820,stroke:#333,color:#ccc
    style S2 fill:#2a2040,stroke:#8b6fc0,color:#b89eed
    style S3 fill:#2d2215,stroke:#d4a24e,color:#d4a24e
    style S4 fill:#161820,stroke:#333,color:#ccc
    style S5 fill:#1a2a1a,stroke:#4a9,color:#4a9
```

> Bootstrap starts minimal and accrues precision over time — the spec is a living document.

