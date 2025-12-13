# AI Agent Guidance for Git Ranger

This document provides guidance for AI agents (GitHub Copilot, Cursor, Cline, Claude, etc.) working on the Git Ranger project. Follow these principles to maintain code quality, consistency, and security.

## Project Overview

**Git Ranger** is a Rust-based CLI tool that manages and synchronizes local Git repositories across multiple providers (GitLab, GitHub). It uses YAML configuration to discover, clone, and update repositories automatically.

**Core Philosophy:**
- Keep it simple and focused
- Security first (never expose tokens)
- Provider-agnostic design
- Test-driven development (TDD)
- User-friendly CLI experience

## Coding Standards

### 1. Rust Best Practices

**Idiomatic Rust:**
- Use strong typing and leverage the type system for safety
- Prefer `Result<T, E>` over panicking (use `?` operator)
- Use `thiserror` for custom error types
- Use `anyhow` for application-level error handling
- Avoid `.unwrap()` and `.expect()` except in tests
- Use pattern matching over if/else when handling enums

**Error Handling:**
```rust
// ✅ Good: Custom error types with thiserror
#[derive(Error, Debug)]
pub enum InitError {
    #[error("Configuration file already exists at {0}")]
    ConfigAlreadyExists(String),
    #[error("Failed to write configuration file: {0}")]
    IoError(#[from] std::io::Error),
}

// ✅ Good: Propagate errors with ?
pub fn init_command(target_dir: &Path) -> Result<(), InitError> {
    let config_path = target_dir.join("ranger.yaml");
    std::fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE)?;
    Ok(())
}

// ❌ Bad: Panic in library code
pub fn init_command(target_dir: &Path) {
    std::fs::write(&config_path, template).unwrap(); // Don't do this!
}
```

**Dependencies:**
- Keep dependencies minimal and well-maintained
- Prefer crates with good documentation and active maintenance
- Check Cargo.toml before adding new dependencies

**Code Style:**
- Run `cargo fmt` before commits (use default rustfmt settings)
- Run `cargo clippy` and address warnings
- Use meaningful variable names (prefer clarity over brevity)
- Add doc comments (`///`) for public APIs
- Keep functions focused and small (prefer composition)

### 2. Test-Driven Development (TDD)

**This project uses TDD religiously. Always write tests first or alongside implementation.**

**Test Structure:**
- Unit tests: In `tests/` directory for integration testing
- Module tests: Use `#[cfg(test)]` for internal unit tests when appropriate
- Both unit and integration tests should exist for all commands

**Test Quality:**
```rust
// ✅ Good: Descriptive test names
#[test]
fn test_init_creates_config_file_in_target_directory() { ... }

#[test]
fn test_init_fails_if_config_already_exists() { ... }

// ❌ Bad: Vague test names
#[test]
fn test_init() { ... }

#[test]
fn test_error() { ... }
```

**Test Coverage Requirements:**
- All public functions must have tests
- Both success and error paths must be tested
- Edge cases should be covered
- Use `tempfile` crate for filesystem testing
- Use `assert_fs` for file system assertions
- Use `mockito` or `wiremock` for API mocking

**Test Organization:**
```
tests/
  ├── init_tests.rs      # Init command tests
  ├── sync_tests.rs      # Sync command tests
  ├── config_tests.rs    # Configuration tests
  └── gitlab_tests.rs    # GitLab provider tests
```

### 3. Security Guidelines

**Critical: Token and credential security is paramount.**

**Never:**
- ❌ Store tokens in configuration files directly
- ❌ Log tokens or sensitive data
- ❌ Include tokens in error messages
- ❌ Commit files with real credentials (even in examples)
- ❌ Use hardcoded credentials anywhere

**Always:**
- ✅ Use environment variables for tokens: `${GITLAB_TOKEN}`
- ✅ Document security best practices in templates
- ✅ Add `.gitignore` entries for sensitive files
- ✅ Validate and sanitize user input
- ✅ Use HTTPS for API calls (default in reqwest)
- ✅ Clear instructions in docs about token security

**EnvString Pattern:**
```rust
// ✅ Good: Use EnvString for token resolution
use crate::config::EnvString;

let token = EnvString::new("${GITLAB_TOKEN}".to_string());
let resolved = token.resolve()?;
```

### 4. Command Structure

**Commands are organized in `src/commands/` with a consistent pattern:**

```
src/commands/
  ├── mod.rs        # Module declarations
  ├── init.rs       # git-ranger init
  ├── sync.rs       # git-ranger sync
  └── ...
```

**Command Implementation Pattern:**
```rust
// Each command should:
// 1. Have its own error type
#[derive(Error, Debug)]
pub enum CommandError { ... }

// 2. Have a main function that returns Result
pub fn command_name(args: Args) -> Result<Output, CommandError> { ... }

// 3. Be called from main.rs via clap
// 4. Have comprehensive tests in tests/command_tests.rs
```

### 5. Configuration Management

**YAML Configuration:**
- Use `serde` and `serde_yaml` for serialization
- Configuration file is always `ranger.yaml`
- Support both relative and absolute paths
- Validate configuration on load

**Structure:**
```yaml
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "${GITLAB_TOKEN}"

groups:
  gitlab:
    - name: "org/team"
      local_dir: "projects"
      recursive: true

repos:
  - url: "git@github.com:user/repo.git"
    local_dir: "standalone"
```

## Architecture Principles

### 1. Modularity

**Separation of Concerns:**
- `src/commands/` - CLI command implementations
- `src/providers/` - Provider-specific API integrations
- `src/config.rs` - Configuration parsing and validation
- `src/lib.rs` - Public library interface
- `src/main.rs` - CLI entry point only

### 2. Provider Abstraction

**Design for multiple providers:**
- Create traits for common provider operations
- Keep provider-specific code in `src/providers/`
- Don't leak provider details into command logic

```rust
// Future pattern (not yet implemented):
trait Provider {
    fn list_repos(&self, group: &str) -> Result<Vec<Repo>>;
    fn clone_repo(&self, url: &str, target: &Path) -> Result<()>;
}
```

### 3. CLI Design

**Use clap with derive macros:**
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "git-ranger")]
#[command(about = "Manages and synchronizes Git repositories")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(short, long, default_value = ".")]
        dir: String,
    },
    // ... more commands
}
```

**User Experience:**
- Provide helpful error messages
- Show progress for long operations
- Offer `--dry-run` for safety
- Include `--help` text for all commands
- Use consistent flag naming

## Documentation Standards

### 1. README.md

**Keep README updated with:**
- Clear project description
- Installation instructions
- Usage examples
- Security best practices
- Configuration documentation

### 2. Code Documentation

**Doc Comments:**
```rust
/// Initializes a new Git Ranger configuration file.
///
/// # Arguments
/// * `target_dir` - The directory where ranger.yaml will be created
///
/// # Errors
/// Returns `InitError::ConfigAlreadyExists` if ranger.yaml already exists.
/// Returns `InitError::IoError` if file creation fails.
///
/// # Examples
/// ```
/// use std::path::Path;
/// use git_ranger::commands::init::init_command;
///
/// let result = init_command(Path::new("."));
/// ```
pub fn init_command(target_dir: &Path) -> Result<(), InitError> { ... }
```

### 3. Implementation Documentation

**Track major implementations in markdown files:**
- `IMPLEMENTATION.md` - Feature implementation summaries
- `SECURITY_IMPLEMENTATION.md` - Security-related changes
- `ENV_CONFIG.md` - Configuration guides

## Git Workflow

### Commits

**Write clear, descriptive commit messages:**
```
✅ Good:
feat: Add init command with YAML template generation
fix: Prevent init from overwriting existing configs
docs: Add security section to README
test: Add integration tests for init command

❌ Bad:
update stuff
fix bug
wip
changes
```

**Commit Message Format:**
- Use conventional commits: `type: description`
- Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`
- Keep first line under 72 characters
- Add body for complex changes

### Branches

**Not strictly enforced, but recommended:**
- `main` - stable, working code
- Feature branches for new work
- Keep branches focused and short-lived

## Common Patterns

### File System Operations

```rust
use std::path::{Path, PathBuf};
use std::fs;

// ✅ Good: Use Path/PathBuf for cross-platform compatibility
pub fn create_config(target_dir: &Path) -> Result<(), Error> {
    let config_path = target_dir.join("ranger.yaml");
    fs::write(&config_path, content)?;
    Ok(())
}

// ❌ Bad: String concatenation for paths
pub fn create_config(target_dir: &str) -> Result<(), Error> {
    let config_path = format!("{}/ranger.yaml", target_dir); // Don't do this!
}
```

### API Calls

```rust
// Use reqwest with proper error handling
use reqwest::blocking::Client;

let client = Client::new();
let response = client
    .get(&url)
    .header("PRIVATE-TOKEN", &token)
    .send()?;

let repos: Vec<Repo> = response.json()?;
```

### Progress Feedback

```rust
// Provide user feedback for long operations
println!("Discovering repositories from GitLab...");
let repos = discover_repos(&config)?;
println!("Found {} repositories", repos.len());

// Use --verbose flag for detailed output
if args.verbose {
    println!("Cloning {} to {}", repo.name, target_path.display());
}
```

## When Adding New Features

1. **Discuss approach first** - Understand requirements and architecture
2. **Write tests first** - Create failing tests that define behavior
3. **Implement minimally** - Solve the problem, don't over-engineer
4. **Run test suite** - Ensure all tests pass: `cargo test`
5. **Update documentation** - README, doc comments, implementation notes
6. **Run linters** - `cargo fmt && cargo clippy`
7. **Test manually** - Actually run the CLI to verify UX

## What to Avoid

❌ **Don't:**
- Add features without tests
- Break existing tests without fixing them
- Expose secrets or tokens
- Make breaking changes without discussion
- Add unnecessary dependencies
- Write overly complex code when simple works
- Skip error handling
- Ignore clippy warnings without good reason
- Commit commented-out code
- Use `println!` for logging (use proper logging if needed)

## Questions or Concerns?

When uncertain about:
- **Architecture decisions** - Ask before implementing
- **Security implications** - Always err on the side of caution
- **Breaking changes** - Discuss impact first
- **New dependencies** - Justify the addition
- **Test strategy** - Better to over-test than under-test

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

---

**Remember: Quality over speed. Test-driven. Security first. User-focused.**
