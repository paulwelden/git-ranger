# AI Agent Guidance for Git Ranger

## Project Overview

**Git Ranger** is a Rust CLI tool that manages and synchronizes local Git repositories across multiple providers (GitLab, GitHub). It uses YAML configuration to discover, clone, and update repositories automatically.

**Core Philosophy:**
- Keep it simple and focused
- Security first (never expose tokens)

- Test-driven development (TDD)
- User-friendly CLI experience

## Command Structure

Commands live in `src/commands/` with a consistent pattern:

```
src/commands/
  mod.rs        # Module declarations
  init.rs       # git-ranger init
  sync.rs       # git-ranger sync
  template.rs   # git-ranger template
  ...
```

Each command must:
1. Define its own error enum using `thiserror` with descriptive prefixed messages
   (e.g., `"Template error: {0}"`, `"Init error: {0}"`)
2. Expose a main function returning `Result<Output, CommandError>`
3. Be wired into `main.rs` via clap
4. Have comprehensive tests in `tests/<command>_tests.rs`

## Source Layout

```
src/
  commands/     # CLI command implementations
  providers/    # Provider-specific API integrations (GitLab, GitHub)
  config.rs     # Configuration parsing and validation
  lib.rs        # Public library interface
  main.rs       # CLI entry point only
```

## EnvString Pattern

Tokens are never stored as plain strings. Use the `EnvString` type for resolution:

```rust
use crate::config::EnvString;

let token = EnvString::new("${GITLAB_TOKEN}".to_string());
let resolved = token.resolve()?;
```

## Test Structure

**Organization:** One test file per command module in `tests/`.

```
tests/
  init_tests.rs
  sync_tests.rs
  config_tests.rs
  template_tests.rs
  gitlab_tests.rs
  ...
```

**Unit tests** use `#[cfg(test)]` inside the module when testing internals.

**Integration tests** live in `tests/` and use:
- `assert_fs` for file system assertions
- `serial_test` for tests that touch shared state
- `mockito` or `wiremock` for API mocking

**Environment variable tests** must use drop guards (not closures) for panic safety,
and must be annotated with `#[serial]`:

```rust
use serial_test::serial;

struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    fn new(key: &str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key: key.to_string(), original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

#[test]
#[serial]
fn test_token_resolves_from_env() {
    let _guard = EnvGuard::new("MY_TOKEN", "secret");
    // test logic here; guard restores env on drop (even on panic)
}
```

## Security Rules

**Never:**
- Store tokens in configuration files directly
- Log tokens or sensitive data
- Include tokens in error messages
- Commit files with real credentials (even in examples)
- Use hardcoded credentials anywhere

**Always:**
- Use environment variables for tokens: `${GITLAB_TOKEN}`
- Document security best practices in templates
- Add `.gitignore` entries for sensitive files
- Validate and sanitize user input
- Use HTTPS for API calls

## When Adding New Features

1. Discuss approach first -- understand requirements and architecture
2. Write tests first -- create failing tests that define behavior
3. Implement minimally -- solve the problem, don't over-engineer
4. Run test suite -- ensure all tests pass: `cargo test`
5. Update documentation -- README, doc comments, implementation notes
6. Run linters -- `cargo fmt && cargo clippy`
7. Test manually -- actually run the CLI to verify UX

## Summary Checklist

Before submitting changes:
- [ ] Tests written and passing (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (README, doc comments)
- [ ] No security issues (secrets, tokens)
- [ ] Manual testing performed
- [ ] Commit messages are clear
- [ ] Error handling is robust
