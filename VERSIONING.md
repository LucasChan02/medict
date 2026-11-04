# Versioning Policy

Medict uses three-segment versions in the form `MAJOR.MINOR.PATCH`.
The policy follows the semantic-versioning rule that incompatible changes
increment `MAJOR`, backward-compatible features increment `MINOR`, and
backward-compatible fixes increment `PATCH`.

## Major

Increment `MAJOR` only when a supported external surface becomes
incompatible, and only after the maintainer explicitly approves the major
release. Do not create or publish a major tag without that approval. Examples:

- A documented command, IPC endpoint, integration binary, or configuration
  format is removed or changed incompatibly.
- Existing dictionary, notebook, or settings data can no longer be read.
- A release replaces a supported distribution architecture in a way that
  breaks external integrations.

An internal rewrite alone does not require a major release when all supported
public behavior and data remain compatible.

## Minor

Increment `MINOR` for backward-compatible features or behavior improvements.
Existing dictionaries, notebooks, settings, and documented integration points
must continue to work.

## Patch

Increment `PATCH` for backward-compatible fixes, packaging corrections,
performance work, and documentation changes that do not add a new feature.

## Current Series

The maintainer directed the Rust rewrite to continue on the `4.0.x` line. The
withdrawn `5.0.0` tag is not used for new builds. The next unused patch after
`4.0.2` is `4.0.3`.

Before a release, compare the new version with the upstream project. Do not
move a published tag. Correct a release mistake with a new version, and obtain
explicit approval before any future major release.
