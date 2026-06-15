# Longevity and Reconstructability

## Alien device test

If an intelligent being discovered a Nous archive after 1,000 years, could they recover its purpose, data, and execution model?

If not, redesign it.

## Requirements

```text
Self-describing artifacts
Human-readable specs
Formal specs for core protocols
Reference implementations
Multiple independent implementations for critical modules
Examples and tests bundled with specs
Migration paths between versions
Source and IR archived with binaries
Open formats
No vendor dependency
No mandatory cloud dependency
```

## NousDNA metadata

Every module should declare:

```text
purpose
interfaces
dependencies
resource budgets
capabilities required
version
migration path
test vectors
reference implementation
human explanation
```

## Canonical artifact rule

Canonical:

```text
spec + source + IR + tests + proofs/examples
```

Non-canonical:

```text
optimized target binary
```
