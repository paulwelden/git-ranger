# Security Enhancement: Environment Variable Support

## Summary

Implemented secure token management using environment variables instead of plain-text tokens in configuration files. This addresses the security vulnerability of storing sensitive credentials in `ranger.yaml`.

## Changes Made

### 1. New Config Module ([src/config.rs](src/config.rs))

Created `EnvString` type that supports environment variable resolution:
- **Syntax**: `${ENV_VAR_NAME}` - Resolves from environment
- **Backward Compatible**: Plain strings work as-is
- **Error Handling**: Clear errors when variables are missing
- **Serde Integration**: Seamlessly deserializes from YAML

**Example usage:**
```rust
let token = EnvString::new("${GITLAB_TOKEN}".to_string());
let resolved = token.resolve()?; // Returns value of GITLAB_TOKEN env var
```

### 2. Updated Configuration Template ([src/commands/init.rs](src/commands/init.rs))

Modified the default `ranger.yaml` template to:
- Use `${GITLAB_TOKEN}` and `${GITHUB_TOKEN}` instead of placeholder text
- Include comprehensive security warnings
- Provide setup instructions for Windows, Linux, and macOS
- Add inline examples for setting environment variables

### 3. Enhanced Documentation

**[README.md](README.md)**
- Added "Security: Protecting Your Tokens" section
- Platform-specific instructions for setting environment variables
- Examples showing proper syntax

**[ENV_CONFIG.md](ENV_CONFIG.md)** (New comprehensive guide)
- Detailed environment variable setup for all platforms
- Token generation instructions for GitLab and GitHub
- Multiple environment patterns (work/personal)
- Troubleshooting guide
- Security checklist
- Best practices

### 4. Test Coverage

**New tests ([tests/config_tests.rs](tests/config_tests.rs)):**
- Environment variable resolution
- Literal string passthrough
- Error handling for missing variables
- YAML parsing with env vars
- Mixed literal and env var configs

**Total test results:** 28 tests pass
- 7 library tests (config module)
- 3 main tests
- 5 config integration tests
- 16 init command tests (existing)

## Security Benefits

✅ **No plain-text tokens** in config files  
✅ **Safe to commit** `ranger.yaml` to version control  
✅ **Already protected** - `.gitignore` includes `ranger.yaml`  
✅ **Backward compatible** - literal tokens still work for testing  
✅ **Clear errors** when variables are missing  
✅ **Cross-platform** support (Windows, Linux, macOS)  

## Usage Example

**Step 1: Set environment variables**
```powershell
# Windows PowerShell
$env:GITLAB_TOKEN = "glpat-xxxxxxxxxxxxxxxxxxxx"
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxx"
```

**Step 2: Initialize config**
```bash
git-ranger init
```

**Step 3: Config uses environment variables automatically**
```yaml
providers:
  gitlab:
    token: "${GITLAB_TOKEN}"  # ✅ Reads from environment
  github:
    token: "${GITHUB_TOKEN}"  # ✅ Reads from environment
```

## Migration Path

For existing users with plain-text tokens:
1. Set environment variables with current token values
2. Update `ranger.yaml` to use `${ENV_VAR_NAME}` syntax
3. Verify with a test command (once sync is implemented)
4. Delete old plain-text tokens from backup files

## Files Modified

- ✅ [src/config.rs](src/config.rs) - New EnvString type with resolution logic
- ✅ [src/lib.rs](src/lib.rs) - Expose config module
- ✅ [src/commands/init.rs](src/commands/init.rs) - Updated template with env vars
- ✅ [README.md](README.md) - Added security section
- ✅ [ENV_CONFIG.md](ENV_CONFIG.md) - New comprehensive guide
- ✅ [tests/config_tests.rs](tests/config_tests.rs) - New integration tests

## Next Steps

When implementing the sync command, use EnvString for token fields:
```rust
use git_ranger::config::EnvString;

#[derive(Deserialize)]
struct ProviderConfig {
    token: EnvString,  // Will auto-resolve from env vars
}

// Later in code:
let token_value = config.provider.token.resolve()?;
```
