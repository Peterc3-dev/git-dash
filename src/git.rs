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
    git_cmd(
        path,
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    )
    .map(|s| parse_ahead_behind(&s))
    .unwrap_or((0, 0))
}

/// Parse the tab-separated output of `git rev-list --left-right --count`.
fn parse_ahead_behind(output: &str) -> (u32, u32) {
    let parts: Vec<&str> = output.trim().split('\t').collect();
    if parts.len() == 2 {
        let ahead = parts[0].parse().unwrap_or(0);
        let behind = parts[1].parse().unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    }
}

fn get_last_commit(path: &Path) -> (String, String, i64) {
    git_cmd(path, &["log", "-1", "--format=%ct\t%s"])
        .map(|s| parse_last_commit(&s))
        .unwrap_or_else(|| ("never".into(), "no commits".into(), 0))
}

/// Parse the `%ct\t%s`-formatted output of `git log -1`.
fn parse_last_commit(output: &str) -> (String, String, i64) {
    let trimmed = output.trim();
    if let Some((ts_str, msg)) = trimmed.split_once('\t') {
        let ts: i64 = ts_str.parse().unwrap_or(0);
        let age = format_age(ts);
        (age, truncate_message(msg), ts)
    } else {
        ("never".into(), "no commits".into(), 0)
    }
}

/// Truncate a commit subject to ~60 chars on a char boundary, adding an ellipsis.
fn truncate_message(msg: &str) -> String {
    if msg.len() > 60 {
        let truncate_at = msg
            .char_indices()
            .take_while(|&(i, _)| i <= 57)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        format!("{}...", &msg[..truncate_at])
    } else {
        msg.to_string()
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
    let output = git_cmd(path, &["log", "-20", "--format=%h\t%an\t%cr\t%s"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(parse_commit_line)
            .collect(),
        None => vec![],
    }
}

/// Parse a `%h\t%an\t%cr\t%s`-formatted line of `git log` into a [`CommitInfo`].
fn parse_commit_line(line: &str) -> Option<CommitInfo> {
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
}

fn get_changed_files(path: &Path) -> Vec<FileStatus> {
    let output = git_cmd(path, &["status", "--porcelain"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .map(parse_status_line)
            .collect(),
        None => vec![],
    }
}

/// Parse a single porcelain line of `git status --porcelain` into a [`FileStatus`].
fn parse_status_line(line: &str) -> FileStatus {
    let status = line.get(0..2).unwrap_or("??").trim().to_string();
    let file_path = line.get(3..).unwrap_or("").to_string();
    FileStatus {
        status,
        path: file_path,
    }
}

fn get_branches(path: &Path) -> Vec<BranchInfo> {
    let output = git_cmd(path, &["branch", "-vv"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .map(parse_branch_line)
            .collect(),
        None => vec![],
    }
}

/// Parse a single line of `git branch -vv` output into a [`BranchInfo`].
fn parse_branch_line(line: &str) -> BranchInfo {
    let is_current = line.starts_with('*');
    let trimmed = line.trim_start_matches('*').trim();
    let name = trimmed.split_whitespace().next().unwrap_or("").to_string();
    let tracking = trimmed
        .find('[')
        .and_then(|start| trimmed.find(']').map(|end| (start, end)))
        .filter(|&(start, end)| start < end)
        .map(|(start, end)| trimmed[start + 1..end].to_string());
    BranchInfo {
        name,
        is_current,
        tracking,
    }
}

fn get_stashes(path: &Path) -> Vec<String> {
    let output = git_cmd(path, &["stash", "list"]);
    match output {
        Some(s) => s
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahead_behind_parses_tab_separated() {
        assert_eq!(parse_ahead_behind("3\t5\n"), (3, 5));
        assert_eq!(parse_ahead_behind("0\t0"), (0, 0));
    }

    #[test]
    fn ahead_behind_handles_malformed_input() {
        // No upstream / unexpected shape falls back to zeros.
        assert_eq!(parse_ahead_behind(""), (0, 0));
        assert_eq!(parse_ahead_behind("garbage"), (0, 0));
        assert_eq!(parse_ahead_behind("1 2"), (0, 0));
    }

    #[test]
    fn last_commit_parses_timestamp_and_subject() {
        let (_age, msg, ts) = parse_last_commit("1700000000\tInitial commit");
        assert_eq!(msg, "Initial commit");
        assert_eq!(ts, 1700000000);
    }

    #[test]
    fn last_commit_handles_no_commits() {
        let (age, msg, ts) = parse_last_commit("");
        assert_eq!(age, "never");
        assert_eq!(msg, "no commits");
        assert_eq!(ts, 0);
    }

    #[test]
    fn truncate_message_leaves_short_subjects_untouched() {
        assert_eq!(truncate_message("short"), "short");
    }

    #[test]
    fn truncate_message_shortens_long_subjects_with_ellipsis() {
        let long = "a".repeat(100);
        let out = truncate_message(&long);
        assert!(out.ends_with("..."));
        assert!(out.len() < long.len());
    }

    #[test]
    fn truncate_message_respects_char_boundaries() {
        // Multi-byte chars must not be split mid-codepoint (would panic on slice).
        let long = "é".repeat(80);
        let out = truncate_message(&long);
        assert!(out.ends_with("..."));
        // If we sliced mid-codepoint this test would have panicked already.
    }

    #[test]
    fn format_age_reports_just_now_for_current_time() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_age(now), "just now");
    }

    #[test]
    fn format_age_reports_future_for_future_timestamps() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_age(now + 10_000), "future");
    }

    #[test]
    fn format_age_buckets_relative_times() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_age(now - 120), "2m ago");
        assert_eq!(format_age(now - 7_200), "2h ago");
        assert_eq!(format_age(now - 3 * 86_400), "3d ago");
        assert_eq!(format_age(now - 14 * 86_400), "2w ago");
    }

    #[test]
    fn branch_line_marks_current_and_extracts_tracking() {
        let b = parse_branch_line("* main 1a2b3c4 [origin/main] latest work");
        assert!(b.is_current);
        assert_eq!(b.name, "main");
        assert_eq!(b.tracking.as_deref(), Some("origin/main"));
    }

    #[test]
    fn branch_line_without_tracking_or_star() {
        let b = parse_branch_line("  feature/x 9f8e7d6 wip");
        assert!(!b.is_current);
        assert_eq!(b.name, "feature/x");
        assert_eq!(b.tracking, None);
    }

    #[test]
    fn commit_line_parses_four_fields() {
        let c = parse_commit_line("1a2b3c4\tJane Doe\t2 days ago\tFix the bug").unwrap();
        assert_eq!(c.hash, "1a2b3c4");
        assert_eq!(c.author, "Jane Doe");
        assert_eq!(c.date, "2 days ago");
        assert_eq!(c.message, "Fix the bug");
    }

    #[test]
    fn commit_line_keeps_tabs_in_subject() {
        // splitn(4) means tabs inside the subject are preserved.
        let c = parse_commit_line("h\ta\td\tmsg\twith\ttabs").unwrap();
        assert_eq!(c.message, "msg\twith\ttabs");
    }

    #[test]
    fn commit_line_rejects_malformed_lines() {
        assert!(parse_commit_line("not enough fields").is_none());
        assert!(parse_commit_line("").is_none());
    }

    #[test]
    fn status_line_splits_code_and_path() {
        let f = parse_status_line(" M src/main.rs");
        assert_eq!(f.status, "M");
        assert_eq!(f.path, "src/main.rs");

        let f = parse_status_line("?? new_file.txt");
        assert_eq!(f.status, "??");
        assert_eq!(f.path, "new_file.txt");
    }
}
