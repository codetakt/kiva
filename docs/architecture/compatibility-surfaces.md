# Compatibility Surfaces

Kiva tracks compatibility surfaces that existed during refactoring so ongoing
call-site and evidence-reference migration remains auditable. These surfaces
are not design targets for new code, and the current ledger records removed
surfaces as retirement history.
The architecture policy treats compatibility as a transitional source layer:
retained or removed surfaces are listed explicitly, but lifecycle ownership is
a governance overlay named `compatibility-retirement-lifecycle`.

The machine-readable ledger is
`spec/architecture/compatibility-surfaces.yaml`. Each entry records the retained
surface, owner, reason, migration target, expansion policy, retirement
milestone, retirement condition, allowed owner exports, and forbidden
call-site patterns.
`generated/architecture/compatibility-retirement-plane.yaml` is the generated
review plane that joins this ledger with DTO semantic-compatibility retirement
and executable deletion-readiness evidence. `ready-to-delete` is reserved for
future retained rows whose automated blockers are clear; it is not permission
to delete a retained public surface without explicit owner action.

Current compatibility-retirement history includes:

- `crates/kiva-confidential/src/lib.rs` for primitive wrapper re-exports and
  the legacy `SecureAggregateError` root export.
- the former `kiva_confidential::mechanism::integrated` namespace, now removed.
  Coupled implementation bodies are owned by `mechanism::core` and exposed
  through mechanism-specific facades.
- `crates/kiva-policy/src/lib.rs` for existing root policy imports.
- the former `crates/kiva-server/src/tests/helpers.rs` owner, now replaced by
  direct helper module bindings from `crates/kiva-server/src/tests.rs`.

New compatibility exports require a ledger entry and a migration target. The
preferred direction is explicit module imports such as
`kiva_confidential::mechanism::{...}`, `kiva_confidential::primitive`, and
resource-family test helper modules. New confidential mechanism review
metadata should be projected through `spec/confidential/mechanism-catalog.yaml`
instead of reintroducing public `integrated` entrypoints.

Compatibility surfaces are temporary by default. A retained surface may stay
while callers and evidence references migrate, but every entry must name the
milestone and concrete retirement condition that closes the retained surface or
converts it into an explicit public contract. The architecture boundary check
validates those fields, the owner file, retained `pub use` anchors, the exact
set of allowed owner re-exports, and negative text scans that prevent new
convenience imports from growing around retained facades. Rust import bans use
`tools/kiva-rust-use-scan`, so `rust-use-absent` checks are evaluated from Rust
AST `use` items rather than arbitrary source text.
