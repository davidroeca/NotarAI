# Non-Software Examples

NotarAI works for any artifact with intent, not just code. These examples show how the schema applies to presentations, legal documents, and research reports.

---

## Presentation spec

A conference talk governed for audience alignment and slide drift.

```yaml
schema_version: '0.7'
domain: presentation

intent: >
  A 30-minute conference talk introducing NotarAI to developers unfamiliar with
  spec-driven workflows. Attendees should leave understanding the three-body drift
  problem and how to run notarai init on their own project.

behaviors:
  - name: opening_hook
    given: 'speaker takes the stage'
    then: 'the intro slide presents a relatable drift scenario in under 90 seconds'
  - name: demo_live
    given: 'the demo section'
    then: 'speaker runs notarai init and reconcile live on a sample repo; audience sees a real drift report'

audience:
  role: 'mid-to-senior developers at a software conference'
  assumed_knowledge: 'Familiar with git, CI/CD, and code review workflows; may not know NotarAI'
  tone: formal-but-engaging
  locale: en-US

output:
  type: presentation
  format: pptx
  runtime: static-file
  entry_point: dist/talk.pptx

content:
  structure: ordered
  sections:
    - id: intro
      type: slide
      intent: 'Hook: show a real drift incident and its cost'
      duration: { value: 3, unit: minutes }
    - id: problem
      type: slide
      intent: 'Explain the three-body drift problem (spec, code, docs)'
      duration: { value: 5, unit: minutes }
    - id: demo
      type: interactive
      intent: 'Live notarai init + reconcile demo on a sample repo'
      duration: { value: 10, unit: minutes }
      content_ref: demo/sample-repo/
    - id: takeaways
      type: slide
      intent: 'Three action items the audience can do today'
      duration: { value: 2, unit: minutes }

design:
  theme:
    palette: ['#0f172a', '#6366f1', '#ffffff']
    typography:
      heading: Inter
      body: Inter
  layout:
    type: slide-deck
    dimensions: '16:9'

artifacts:
  slides:
    - path: 'slides/**/*.md'
      role: 'slide source content'
  assets:
    - path: 'assets/**'
      role: 'images and diagrams'
```

**What this demonstrates:** `output.type: presentation`, `content.sections` with `duration`, `audience`, and `design`. The reconciliation engine uses `duration` to detect if the talk now runs over time, and `intent` per section to detect off-message slides.

---

## Legal contract spec

A service agreement governed for compliance and clause integrity.

```yaml
schema_version: '0.7'
domain: legal

intent: >
  A standard SaaS service agreement for enterprise customers. Governs payment
  terms, liability limits, data processing obligations, and termination rights.
  The spec tracks which clauses satisfy which regulatory requirements so that
  removing or weakening a clause triggers a compliance drift alert.

behaviors:
  - name: data_processing
    given: 'customer data is processed by the service'
    then: 'the DPA clause defines processing purposes, data categories, and sub-processor obligations per GDPR Article 28'
  - name: liability_cap
    given: 'a dispute arises'
    then: 'liability is capped at 12 months of fees paid, except for gross negligence or data breach'

constraints:
  - 'All clause changes must be reviewed by legal counsel before execution'
  - 'Governing law must match the entity jurisdiction for each signed copy'

invariants:
  - 'The DPA clause must never be removed from the agreement'
  - 'Liability cap language must reference the specific cap amount'

compliance:
  frameworks:
    - name: GDPR
      controls:
        - id: Art28
          satisfied_by:
            invariants:
              ['The DPA clause must never be removed from the agreement']
    - name: SOC2
      controls:
        - id: CC9.2
          satisfied_by:
            constraints:
              [
                'All clause changes must be reviewed by legal counsel before execution',
              ]
  audit_trail: true

output:
  type: document
  format: pdf

content:
  structure: ordered
  sections:
    - id: definitions
      type: clause
      intent: 'Define all capitalized terms used in the agreement'
    - id: services
      type: clause
      intent: 'Describe the scope and delivery of services'
    - id: payment
      type: clause
      intent: 'Payment terms, invoicing cycle, and late payment penalties'
    - id: dpa
      type: clause
      intent: 'Data Processing Agreement per GDPR Article 28'
      depends_on:
        - id: definitions
          relationship: 'References defined terms for data categories and processing'
    - id: liability
      type: clause
      intent: 'Limit liability to 12 months fees; carve out gross negligence and data breach'
    - id: termination
      type: clause
      intent: 'Termination for convenience (30 days notice) and for cause (material breach)'

design:
  layout:
    type: paginated
    dimensions: letter
  print:
    margins: { top: '1in', right: '1in', bottom: '1in', left: '1in' }
    headers: true
    footers: true
    page_numbers: true

artifacts:
  docs:
    - path: 'contracts/service-agreement.md'
      role: 'master agreement source'
  configs:
    - path: 'contracts/variables.yaml'
      role: 'per-customer variable substitutions (entity name, jurisdiction, fees)'
```

**What this demonstrates:** `domain: legal`, `compliance.frameworks` with control mappings, `content.sections` with `type: clause` and `depends_on`, `design.print` for paginated layout. The compliance block creates an explicit link between the GDPR requirement and the DPA clause -- if someone removes the DPA clause, the reconciliation engine flags it as a high-priority drift event.

---

## Research report spec

An evidence-backed technical report governed for citation integrity.

```yaml
schema_version: '0.7'
domain: research

intent: >
  A technical report evaluating three approaches to LLM-assisted code review:
  prompt-only, RAG-augmented, and spec-anchored. Reports accuracy, latency, and
  reviewer acceptance metrics from a 90-day study across 12 repositories.

behaviors:
  - name: methodology_reproducible
    given: 'a reader follows the methodology section'
    then: 'they can reproduce the experimental setup using the linked code and dataset'
  - name: results_traceable
    given: 'a claim appears in the results section'
    then: 'it is linked to a specific row or aggregate in the dataset'

constraints:
  - 'All quantitative claims must cite a specific data source in evidence'
  - 'Comparison tables must include confidence intervals'
  - 'Methodology must describe exclusion criteria for repositories'

output:
  type: document
  format: pdf

content:
  structure: ordered
  sections:
    - id: abstract
      type: section
      intent: 'Summarize the study question, methods, and key finding in 150 words'
      duration: { value: 2, unit: minutes }
    - id: methodology
      type: section
      intent: 'Describe the 90-day study design, repository selection criteria, and evaluation metrics'
      content_ref: sections/methodology.md
      evidence:
        - type: reference
          ref: 'Chen et al. 2023 -- LLM code review benchmarks'
          claim: 'Our accuracy metric aligns with the Chen et al. framework'
          relationship: 'supports methodology choice'
    - id: results
      type: section
      intent: 'Present accuracy, latency, and acceptance metrics per approach with confidence intervals'
      content_ref: sections/results.md
      evidence:
        - type: data
          source: data/results_final.csv
          claim: 'Spec-anchored approach achieves 94% accuracy vs 81% for prompt-only'
          relationship: 'primary quantitative result'
        - type: data
          source: data/latency.csv
          claim: 'Median review latency under 4 seconds for all approaches'
    - id: discussion
      type: section
      intent: 'Interpret results, discuss limitations, and suggest future work'
      depends_on:
        - id: results
          relationship: 'Interpretation requires results to be finalized'
    - id: conclusion
      type: section
      intent: 'State the recommendation: spec-anchored review for accuracy-critical workflows'

feedback:
  metrics:
    - name: peer_review_score
      threshold: '>= 3.5 / 5'
    - name: reproduction_success_rate
      threshold: '>= 0.8'
  triggers:
    - condition:
        metric: peer_review_score
        operator: below_threshold
      action: reconcile
      priority: high

artifacts:
  docs:
    - path: 'sections/**/*.md'
      role: 'report section source content'
  data:
    - path: 'data/**/*.csv'
      role: 'experimental results datasets'
  configs:
    - path: 'analysis/**/*.py'
      role: 'analysis scripts that produce data/ outputs'
```

**What this demonstrates:** `domain: research`, `content.sections` with `evidence` entries linking claims to data sources, `depends_on` between sections, `feedback.triggers` for structured review thresholds, and `duration` for time-budgeted writing. When `data/results_final.csv` changes, the reconciliation engine flags the results section's claim for review because it is linked via `evidence`.
