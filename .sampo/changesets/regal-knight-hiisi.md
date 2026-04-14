---
cargo/notarai: major
---

- Add --strict circular ref detection and behavior completeness
- Update .github/workflows to accommodate new restructuring
- orchestrate reconciliation with cli wrapper
- set up github action crate
- update docs
- adopt [sampo](https://github.com/bruits/sampo)
- Drift Scoring
  - Add MCP initialize integration with driftScore/driftStatus/mostDrifted
- Test-Spec Alignment
  - Add T001/T002 checks in core/check.rs with correct tiers (Critical/Housekeeping).
- better alternative coding agent support
