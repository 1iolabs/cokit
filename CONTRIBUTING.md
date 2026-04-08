# Contributing to COKIT

Thank you for your interest in contributing to COKIT.

## Contributor License Agreement (CLA)

Before we can accept your contribution, you must sign a Contributor
License Agreement:

- **Individuals:** Sign the [Individual CLA](CLA.md)
- **Organizations:** Sign the [Corporate CLA](CCLA.md)

### Why is a CLA required?

COKIT is dual-licensed. The community receives the code under
**AGPL-3.0-only**. 1io also offers commercial licenses for organizations
that need proprietary deployment options.

The CLA grants 1io the right to distribute your contributions under
both the AGPL and commercial licenses. Without this grant, 1io could
not offer commercial licensing, because each contributor would retain
exclusive control over the licensing of their contribution.

### How to sign

**Individuals:** Submit a pull request adding your name to the CLA
signatories file, or sign electronically via the CLA bot when you
submit your first pull request.

**Organizations:** Download the [Corporate CLA](CCLA.md), fill in the
details including Schedule A (authorized employees), sign it, and send
it to license@1io.com.

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/).
Prefix your commit messages with a type:

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation only
- `chore:` maintenance (deps, CI, etc.)
- `refactor:` code change that neither fixes a bug nor adds a feature
- `test:` adding or updating tests
- `perf:` performance improvement

Breaking changes should include `!` after the type (e.g., `feat!: remove old API`).

## How to Contribute

1. Fork the repository
2. Create a feature branch
3. Make your changes (using conventional commit messages)
4. Ensure tests pass: `cargo test -p co-sdk`
5. Submit a pull request

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
By participating, you are expected to uphold this code. Please report
unacceptable behavior to conduct@1io.com.

## License

By contributing, you agree that your contributions will be licensed under
**AGPL-3.0-only** and may also be distributed under commercial licenses
as described in the CLA.

## Scope

This repository (`cokit`) is the open-source platform core of COKIT.
GUARD (`guard`) is a separate repository under separate licensing terms
and is not covered by this contribution process.

## Questions

Contact: license@1io.com
Information: https://www.cokit.org
