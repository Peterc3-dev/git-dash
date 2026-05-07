# git-dash

Multi-repo git dashboard TUI — see the state of every repo in a directory at a glance.

## Features

- Scans a directory for git repositories and displays branch, status, ahead/behind counts
- Detail view per repo with tabs: recent commits, uncommitted changes, branches, stashes
- Background `git fetch --all` across every repo (threaded, non-blocking)
- Auto-refresh every 30 seconds
- Filter repos by name with `/` search
- Sortable repo list (cycle with `s`)
- Parallel status scanning via rayon

## Install

```
cargo build --release
# binary at target/release/git-dash
```

## Usage

```
# scan ~/projects (default)
git-dash

# scan a specific directory
git-dash --dir /path/to/repos
```

## Keybindings

### Repo list

| Key | Action |
|-----|--------|
| `j` / `k` or arrows | Move selection |
| `Enter` | Open repo detail view |
| `f` | Fetch all repos in background |
| `r` | Refresh status |
| `/` | Filter repos by name |
| `s` | Cycle sort order |
| `g` / `G` | Jump to first / last repo |
| `q` / `Ctrl-C` | Quit |

### Detail view

| Key | Action |
|-----|--------|
| `1`-`4` or `Tab` | Switch tab: Commits / Changes / Branches / Stashes |
| `j` / `k` or arrows | Scroll |
| `Esc` / `Backspace` | Back to repo list |
| `q` / `Ctrl-C` | Quit |

---
Built with Rust + ratatui
