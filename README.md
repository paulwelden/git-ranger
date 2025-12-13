# Git Ranger

Git Ranger is a command line tool that manages and synchronizes local Git repositories across multiple providers. Using a simple YAML configuration file, Git Ranger discovers, clones, and updates repositories from sources like GitLab and GitHub, keeping your local workspace clean and organized.

It removes the repetitive work of tracking down repos, running manual fetches, and maintaining directory structures, so you can focus on development instead of housekeeping.

## Why Git Ranger?

Modern development teams work across many repos and sometimes multiple hosting providers. Git Ranger acts as a field ranger for your source code terrain. It scouts providers for changes, keeps your local environment in sync, and ensures new repos are automatically brought into your workspace.

## Key Features

- Provider agnostic design supporting GitLab and GitHub initially  
- YAML configuration as the single source of truth  
- Automatic repo discovery from configured groups, orgs, or direct URLs  
- Smart cloning and fetching to keep everything current  
- Local directory management with predictable paths  
- Dry run mode for safe previews  
- Plugin friendly architecture for future providers and workflows  

## Example Configuration

```yaml
# ranger.yaml

providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "${GITLAB_TOKEN}"  # Reads from environment variable
  github:
    token: "${GITHUB_TOKEN}"  # Reads from environment variable
   
groups:
  gitlab:
    - name: "DSSI/dssi.product"
      local_dir: "dssi-projects"
      recursive: true
    - name: "tools"

repos:
  - url: "git@github.com:my-org/standalone-tool.git"
    local_dir: "standalone"
```

### Security: Protecting Your Tokens

**Git Ranger uses environment variables to keep tokens secure.** Never commit tokens directly to `ranger.yaml`!

**Setting up environment variables:**

**Windows (PowerShell):**
```powershell
# Set for current session
$env:GITLAB_TOKEN = "your-gitlab-token"
$env:GITHUB_TOKEN = "your-github-token"

# Persist across sessions - add to PowerShell profile
notepad $PROFILE
# Add the lines above to the profile file
```

**Linux/macOS (bash/zsh):**
```bash
# Set for current session
export GITLAB_TOKEN="your-gitlab-token"
export GITHUB_TOKEN="your-github-token"

# Persist across sessions - add to shell profile
echo 'export GITLAB_TOKEN="your-gitlab-token"' >> ~/.bashrc  # or ~/.zshrc
echo 'export GITHUB_TOKEN="your-github-token"' >> ~/.bashrc
source ~/.bashrc
```

**Verify tokens are set:**
```bash
# Windows PowerShell
echo $env:GITLAB_TOKEN

# Linux/macOS
echo $GITLAB_TOKEN
```

The `ranger.yaml` file is automatically ignored by git to prevent accidental commits.

### Configuration Notes

- **`local_dir`**: Optional path where repositories will be cloned. Can be specified per group or per repo.
  - If not specified, repositories will be cloned to the current working directory.
  - Supports both relative paths (e.g., `"dssi-projects"`) and absolute paths (e.g., `"C:/repos/projects"`).

- **`recursive`**: Optional boolean flag for groups to include nested subgroups.
  - When set to `true`, Git Ranger will discover and clone repositories from all nested subgroups within the specified group.
  - When set to `false` or omitted, only repositories directly under the group will be included.
  - Particularly useful for GitLab groups with deep subgroup hierarchies (e.g., `parent/child/grandchild`).

## Installation

### From Binary (Recommended)

Download the latest release for your platform from the [releases page](https://github.com/paulwelden/git-ranger/releases):

**Windows:**
```powershell
# Download and extract
Invoke-WebRequest -Uri "https://github.com/paulwelden/git-ranger/releases/latest/download/git-ranger-windows-x86_64.zip" -OutFile "git-ranger.zip"
Expand-Archive -Path git-ranger.zip -DestinationPath .
# Move to a directory in your PATH, e.g.:
Move-Item git-ranger.exe "C:\Program Files\git-ranger\"
```

**macOS (Intel):**
```bash
curl -L https://github.com/paulwelden/git-ranger/releases/latest/download/git-ranger-macos-x86_64.tar.gz | tar xz
sudo mv git-ranger /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -L https://github.com/paulwelden/git-ranger/releases/latest/download/git-ranger-macos-aarch64.tar.gz | tar xz
sudo mv git-ranger /usr/local/bin/
```

**Linux:**
```bash
curl -L https://github.com/paulwelden/git-ranger/releases/latest/download/git-ranger-linux-x86_64.tar.gz | tar xz
sudo mv git-ranger /usr/local/bin/
```

### From Source (Requires Rust)

```bash
cargo install --path .
```

Or directly from the repository:
```bash
cargo install --git https://github.com/paulwelden/git-ranger
```

**Windows:**

```powershell
# Download git-ranger.exe and add to PATH, or place in a directory already in PATH
# Example: C:\Users\YourName\.local\bin\
```

### From Source

**Prerequisites:**

- Rust toolchain (install from [rustup.rs](https://rustup.rs))

**Build and install:**

```bash
# Clone the repository
git clone https://github.com/your-username/git-ranger.git
cd git-ranger

# Build and install directly to cargo bin directory
cargo install --path .

# Or build and manually copy
cargo build --release
# Binary will be in target/release/git-ranger (or git-ranger.exe on Windows)
# Copy to a directory in your PATH
```

### Verify Installation

```bash
git-ranger --version
```

## How It Works

1. Git Ranger reads your YAML configuration.  
2. It queries each provider for matching groups, orgs, and repos.  
3. It compares those repos with your local filesystem.  
4. Missing repos are cloned.  
5. Existing repos are fetched and updated.  
6. Everything stays neatly organized in one workspace.

## Commands

Git Ranger follows git-style subcommands for a familiar experience:

```bash
# Initialize a new ranger.yaml config in current directory
git-ranger init

# Synchronize workspace: clone missing repos and fetch updates for existing ones
# Idempotent - safe to run daily or after config changes
git-ranger sync

# Sync only specific group or repo
git-ranger sync <group-name>
git-ranger sync <repo-url>

# Show status of all configured repos (like git status, but workspace-wide)
git-ranger status

# List all repos from config with their local paths
git-ranger ls

# Preview what sync would do without making changes
git-ranger sync --dry-run
```

### Common Workflows

```bash
# First time setup
git-ranger init
# Edit ranger.yaml with your providers and groups
git-ranger sync  # Clone and sync everything

# Daily workflow (idempotent - run anytime)
git-ranger sync   # Clones any new repos, fetches updates for existing
git-ranger status # See what changed

# When group membership changes
# Day 1: 5 repos in group
git-ranger sync
# Day 2: someone adds 6th repo to group
git-ranger sync   # Automatically clones the new repo

# Before making changes
git-ranger sync --dry-run  # Preview what would happen
```

## Roadmap

- Full GitLab subgroup support  
- Expanded GitHub org and team filtering  
- Additional provider integrations (Gitea, Bitbucket, etc)  
- Repo archiving or pruning policies  
- Workspace profiles  
- Parallel operations  
- Interactive UI modes  

## Contributing

Contributions are welcome once the project structure is in place. Ideas, issues, and PRs will help Git Ranger evolve into a flexible multi-provider syncing tool.

## License

MIT License unless changed later.
