# Environment Variable Configuration Examples

This document shows how to configure Git Ranger to use environment variables for secure token management.

## Why Environment Variables?

**Security Benefits:**
- ✅ Tokens never stored in plain text files
- ✅ Config files can be safely committed to version control
- ✅ No risk of accidentally exposing tokens in backups or logs
- ✅ Easy to rotate tokens without editing config files
- ✅ Different tokens per environment (dev, prod, personal)

## Configuration Syntax

In `ranger.yaml`, use `${VAR_NAME}` to reference environment variables:

```yaml
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "${GITLAB_TOKEN}"  # Reads from GITLAB_TOKEN env var
  
  github:
    token: "${GITHUB_TOKEN}"  # Reads from GITHUB_TOKEN env var
```

## Setting Environment Variables

### Windows (PowerShell)

**Current session only:**
```powershell
$env:GITLAB_TOKEN = "glpat-xxxxxxxxxxxxxxxxxxxx"
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxx"
```

**Persistent (all sessions):**
```powershell
# Edit your PowerShell profile
notepad $PROFILE

# Add these lines to the profile file:
$env:GITLAB_TOKEN = "glpat-xxxxxxxxxxxxxxxxxxxx"
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxx"

# Reload profile
. $PROFILE
```

**System-wide (requires admin):**
```powershell
[System.Environment]::SetEnvironmentVariable("GITLAB_TOKEN", "glpat-xxx", "User")
[System.Environment]::SetEnvironmentVariable("GITHUB_TOKEN", "ghp-xxx", "User")
```

### Linux/macOS (bash)

**Current session only:**
```bash
export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
```

**Persistent (all sessions):**
```bash
# Add to ~/.bashrc or ~/.bash_profile
echo 'export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"' >> ~/.bashrc
echo 'export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"' >> ~/.bashrc

# Reload
source ~/.bashrc
```

### Linux/macOS (zsh)

```bash
# Add to ~/.zshrc
echo 'export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"' >> ~/.zshrc
echo 'export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"' >> ~/.zshrc

# Reload
source ~/.zshrc
```

## Verifying Environment Variables

### Windows (PowerShell)
```powershell
# Check if variable is set
echo $env:GITLAB_TOKEN
echo $env:GITHUB_TOKEN

# List all environment variables
Get-ChildItem env: | Where-Object {$_.Name -like "*TOKEN*"}
```

### Linux/macOS
```bash
# Check if variable is set
echo $GITLAB_TOKEN
echo $GITHUB_TOKEN

# List all TOKEN-related variables
env | grep TOKEN
```

## Obtaining Tokens

### GitLab Personal Access Token

1. Go to GitLab → Settings → Access Tokens
2. Create a new token with scopes:
   - `read_api` - For reading group and project information
   - `read_repository` - For cloning repositories (if needed)
3. Copy the token (starts with `glpat-`)
4. Set as `GITLAB_TOKEN` environment variable

### GitHub Personal Access Token

1. Go to GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token with scopes:
   - `repo` - Full control of private repositories
   - `read:org` - Read org and team membership
3. Copy the token (starts with `ghp_`)
4. Set as `GITHUB_TOKEN` environment variable

## Multiple Environments

You can use different tokens for different purposes:

```yaml
providers:
  gitlab_work:
    host: "https://gitlab.company.com"
    token: "${GITLAB_WORK_TOKEN}"
  
  gitlab_personal:
    host: "https://gitlab.com"
    token: "${GITLAB_PERSONAL_TOKEN}"
  
  github_work:
    token: "${GITHUB_WORK_TOKEN}"
  
  github_personal:
    token: "${GITHUB_PERSONAL_TOKEN}"
```

## Troubleshooting

### "Environment variable 'XXX' is not set"

This error means Git Ranger cannot find the environment variable.

**Solution:**
1. Verify the variable is set: `echo $env:VARIABLE_NAME` (PowerShell) or `echo $VARIABLE_NAME` (bash/zsh)
2. Check spelling in both the config and the environment
3. Restart your terminal after setting persistent variables
4. Ensure you've reloaded your shell profile after editing it

### Token not working

1. Verify the token has correct permissions (scopes)
2. Check if the token has expired
3. Test the token manually with curl:

```bash
# Test GitLab token
curl -H "PRIVATE-TOKEN: $GITLAB_TOKEN" https://gitlab.example.com/api/v4/user

# Test GitHub token
curl -H "Authorization: token $GITHUB_TOKEN" https://api.github.com/user
```

## Best Practices

1. **Never commit tokens** to version control
2. **Use descriptive variable names** (GITLAB_WORK_TOKEN, not TOKEN1)
3. **Rotate tokens regularly** (every 90 days recommended)
4. **Use minimal scopes** required for Git Ranger
5. **Keep config files generic** - they should work for any team member
6. **Document which variables are needed** in your project README

## Security Checklist

- [ ] Tokens stored only in environment variables
- [ ] `ranger.yaml` in `.gitignore` (automatic with git-ranger init)
- [ ] Tokens have minimal required scopes
- [ ] Tokens are rotated periodically
- [ ] Team members use their own tokens (never share)
- [ ] Shell profiles with tokens have restricted permissions (chmod 600)
