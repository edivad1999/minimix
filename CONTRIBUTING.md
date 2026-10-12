# Contributing to Minimix

Thanks for helping. Minimix is a cross-platform virtual microphone with a shared Rust DSP engine.

## Before you start

1. Open an issue for larger changes so direction stays clear.
2. Keep PRs focused. One concern per PR works best with the stacked delivery style this project uses.
3. Do not commit secrets, signing keys, personal paths, editor agent folders, or generated `target/` output.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Platform adapters (PipeWire, Core Audio HAL, WaveRT) need the matching OS. Pure DSP work can land on any host.

## Pull requests

- Target `main` unless you are stacking on an open parent branch.
- Keep the PR ready for review (not draft) when CI is expected to run.
- Fill the PR template.
- CI must stay green on Linux, macOS, and Windows.

## Commit style

Write short commits that say why the change exists. Use your own name and email. Do not add tooling co-author trailers.

## Code of conduct

Participation is covered by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
