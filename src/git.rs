use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use rayon::prelude::*;

/// Information about a single git repository
#[derive(Clone, Debug)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty_count: u32,
    pub last_commit_age: String,
    pub last_commit_message: String,
    pub last_commit_timestamp: i64,
    #[allow(dead_code)]
    pub error: Option<String>,
}

/// Detailed info for a single repo (detail view)
#[derive(Clone, Debug)]
pub struct RepoDetail {
    pub commits: Vec<CommitInfo>,
    pub changed_files: Vec<FileStatus>,
    pub branches: Vec<BranchInfo>,
    pub stashes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct FileStatus {
    pub status: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}

fn git_cmd(repo_path: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).to_string())
            } else {
                None
            }
        })
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Scan a directory for git repos (non-recursive, one level deep)
pub fn discover_repos(scan_dir: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    if let Ok(entries) = std::fs::read_dir(scan_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && is_git_repo(&p) {
                repos.push(p);
            }
        }
    }
    repos.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
    });
    repos
}

/// Scan a single repo for summary info
pub fn scan_repo(path: &Path) -> RepoInfo {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let branch = git_cmd(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "???".into());

    // ahead/behind
    let (ahead, behind) = get_ahead_behind(path);

    // dirty file count
    let dirty_count = git_cmd(path, &["status", "--porcelain"])
        .map(|s| s.lines().filter(|l| !l.is_empty()).count() as u32)
        .unwrap_or(0);

    // last commit
    let (last_commit_age, last_commit_message, last_commit_timestamp) = get_last_commit(path);

    RepoInfo {
        path: path.to_path_buf(),
        name,
        branch,
        ahead,
        behind,
        dirty_count,
        last_commit_age,
        last_commit_message,
        last_commit_timestamp,
        error: None,
    }
}

/// Scan all repos in parallel
pub fn scan_all_repos(paths: &[PathBuf]) -> Vec<RepoInfo> {
    paths.par_iter().map(|p| scan_repo(p)).collect()
}

/// Fetch all repos in parallel
pub fn fetch_all_repos(paths: &[PathBuf]) {
    paths.par_iter().for_each(|p| {
        let _ = Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["fetch", "--all", "--quiet"])
            .output();
    });
}

fn get_ahead_behind(path: &Path) -> (u32, u32) {
    let output = git_cmd(path, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]);
    match output {
        Some(s) => {
            let parts: Vec<&str> = s.trim().split('\t').collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().unwrap_or(0);
                let behind = parts[1].parse().unwrap_or(0);
                (ahead, behind)
            } else {
                (0, 0)
            }
        }
        None => (0, 0),
    }
}

fn get_last_commit(path: &Path) -> (String, String, i64) {
    let output = git_cmd(path, &["log", "-1", "--format=%ct\t%s"]);
    match output {
        Some(s) => {
            let trimmed = s.trim();
            if let Some((ts_str, msg)) = trimmed.split_once('\t') {
                let ts: i64 = ts_str.parse().unwrap_or(0);
                let age = format_age(ts);
                let msg_truncated = if msg.len() > 60 {
                    let truncate_at = msg.char_indices()
                        .take_while(|&(i, _)| i <= 57)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    format!("{}...", &msg[..truncate_at])
                } else {
                    msg.to_string()
                };
                (age, msg_truncated, ts)
            } else {
                ("never".into(), "no commits".into(), 0)
            }
        }
        None => ("never".into(), "no commits".into(), 0),
    }
}

fn format_age(unix_ts: i64) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = now - unix_ts;
    if diff < 0 {
        return "future".into();
    }
    let minutes = diff / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let weeks = days / 7;
    let months = days / 30;

    if minutes < 1 {
        "just now".into()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days < 7 {
        format!("{}d ago", days)
    } else if weeks < 5 {
        format!("{}w ago", weeks)
    } else {
        format!("{}mo ago", months)
    }
}

/// Get detailed info for a single repo
pub fn get_repo_detail(path: &Path) -> RepoDetail {
    let commits = get_recent_commits(path);
    let changed_files = get_changed_files(path);
    let branches = get_branches(path);
    let stashes = get_stashes(path);

    RepoDetail {
        commits,
        changed_files,
        branches,
        stashes,
    }
}

fn get_recent_commits(path: &Path) -> Vec<CommitInfo> {
    let output = git_cmd(
        path,
        &["log", "-20", "--format=%h\t%an\t%cr\t%s"],
    );
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '\t').collect();
                if parts.len() == 4 {
                    Some(CommitInfo {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect(),
        None => vec![],
    }
}

fn get_changed_files(path: &Path) -> Vec<FileStatus> {
    let output = git_cmd(path, &["status", "--porcelain"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let status = line.get(0..2).unwrap_or("??").trim().to_string();
                let file_path = line.get(3..).unwrap_or("").to_string();
                FileStatus {
                    status,
                    path: file_path,
                }
            })
            .collect(),
        None => vec![],
    }
}

fn get_branches(path: &Path) -> Vec<BranchInfo> {
    let output = git_cmd(path, &["branch", "-vv"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let is_current = line.starts_with('*');
                let trimmed = line.trim_start_matches('*').trim();
                let name = trimmed
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let tracking = if let Some(start) = trimmed.find('[') {
                    if let Some(end) = trimmed.find(']') {
                        Some(trimmed[start + 1..end].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                BranchInfo {
                    name,
                    is_current,
                    tracking,
                }
            })
            .collect(),
        None => vec![],
    }
}

fn get_stashes(path: &Path) -> Vec<String> {
    let output = git_cmd(path, &["stash", "list"]);
    match output {
        Some(s) => s.lines().filter(|l| !l.is_empty()).map(String::from).collect(),
        None => vec![],
    }
}
