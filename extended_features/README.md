# Extended Features

This directory documents features added to `ubermind` that extend the functionality of the underlying `overmind` tool.

## Process

1.  **Propose**: A new feature is identified that solves a specific user need not met by standard `overmind`.
2.  **Implement**: The feature is implemented in `ubermind` as a proof-of-concept.
3.  **Document**: The feature is documented in this directory, explaining the "why" and "how".
4.  **Evaluate**: We periodically review these features to decide if they should be proposed upstream to `overmind` via a Pull Request.

## Features

- [Action Confirmation](action-confirmation.md): Real-time feedback with dots showing command progress and verifying actions succeeded.
- [Echo Filtering](echo-filtering.md): Filter logs by service name.
- [UB Add Shorthand](ub-add-shorthand.md): Add current directory as a service with `ub add` (no args).
