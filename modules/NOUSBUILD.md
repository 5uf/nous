# Module: NousBuild

Reproducible build and image generation system.

## Responsibilities

```text
build modules deterministically
store artifacts in NousFS
generate device-specific images
record provenance
sign artifacts
rollback broken builds
```

## Inspirations

Nix, Guix, Bazel, Buildroot, Yocto.

## MVP

A simple manifest format that records inputs, command, outputs, hashes.
