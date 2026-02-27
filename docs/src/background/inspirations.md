# Inspirations

NotarAI draws from several established traditions:

- **[Cucumber](https://cucumber.io/) / [Gherkin](https://cucumber.io/docs/gherkin/):** The Given/Then behavior format in NotarAI specs comes from BDD's structured scenario language, but kept in natural language rather than formal Gherkin syntax to lower the authoring barrier.
- **[Terraform](https://www.terraform.io/) and Infrastructure-as-Code:** The reconciliation model (declare desired state, detect drift from actual state, propose a plan to converge) is borrowed from IaC tools like Terraform, [Pulumi](https://www.pulumi.com/), and [CloudFormation](https://aws.amazon.com/cloudformation/). NotarAI's spec is a state file for intent, not infrastructure.
- **[JSON Schema](https://json-schema.org/) / [OpenAPI](https://www.openapis.org/):** The `$ref` composition model and the use of a JSON Schema to govern spec validity come directly from these standards.
- **[Design by Contract](https://en.wikipedia.org/wiki/Design_by_contract) (Eiffel):** The distinction between `constraints` (what the system enforces) and `invariants` (what must never be violated) echoes Eiffel's preconditions, postconditions, and class invariants.
- **[Architecture Decision Records](https://adr.github.io/):** The `decisions` field in the spec is a lightweight ADR log, capturing the _why_ alongside the _what_.
