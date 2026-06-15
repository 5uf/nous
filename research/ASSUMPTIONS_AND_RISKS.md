# Assumptions and Risks

## Assumptions

- Users will not migrate to a new OS immediately.
- Modular adoption is mandatory.
- Local AI will remain important.
- Performance per watt will matter more over time.
- Existing protocols will survive for decades.
- Hardware fragmentation will increase.

## Risks

```text
Scope explosion
Building kernel too early
Overdesigning NousLang before proving modules
Ignoring legacy compatibility
Underestimating driver complexity
Overusing AI-generated code in trusted paths
Making blockchain mandatory
Using too many dependencies
Performance loss from excessive microkernel IPC
Security model too hard for users to understand
```

## Mitigations

```text
MVP first
resource budgets
compatibility bridges
use existing OSes initially
strict module boundaries
hot-path profiling
docs/specs/tests with every module
no trusted generated code without review/tests
```
