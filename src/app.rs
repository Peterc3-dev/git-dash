use std::path::PathBuf;

use crate::git::{self, RepoDetail, RepoInfo};

#[derive(Clone, Debug)]
pub enum View {
    RepoList,
    RepoDetail(usize), // index into filtered repos
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortMode {
    Name,
    LastCommit,
    DirtyCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailTab {
    Commits,
    Changes,
    Branches,
    Stashes,
}

pub struct App {
    pub scan_dir: PathBuf,
    pub repo_paths: Vec<PathBuf>,
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub view: View,
    pub sort_mode: SortMode,
    pub filter_text: String,
    pub filter_input: String,
    pub filter_active: bool,
    pub fetching: bool,
    pub detail: Option<RepoDetail>,
    pub detail_tab: DetailTab,
    pub detail_scroll: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(scan_dir: PathBuf) -> Self {
        let repo_paths = git::discover_repos(&scan_dir);
        let repos = git::scan_all_repos(&repo_paths);

        App {
            scan_dir,
            repo_paths,
            repos,
            selected: 0,
            view: View::RepoList,
            sort_mode: SortMode::Name,
            filter_text: String::new(),
            filter_input: String::new(),
            filter_active: false,
            fetching: false,
            detail: None,
            detail_tab: DetailTab::Commits,
            detail_scroll: 0,
            should_quit: false,
        }
    }

    pub fn filtered_repos(&self) -> Vec<RepoInfo> {
        let mut repos: Vec<RepoInfo> = if self.filter_text.is_empty() {
            self.repos.clone()
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            self.repos
                .iter()
                .filter(|r| r.name.to_lowercase().contains(&filter_lower))
                .cloned()
                .collect()
        };

        match self.sort_mode {
            SortMode::Name => repos.sort_by_key(|r| r.name.to_lowercase()),
            SortMode::LastCommit => {
                repos.sort_by_key(|r| std::cmp::Reverse(r.last_commit_timestamp))
            }
            SortMode::DirtyCount => repos.sort_by_key(|r| std::cmp::Reverse(r.dirty_count)),
        }

        repos
    }

    pub fn refresh(&mut self) {
        self.repo_paths = git::discover_repos(&self.scan_dir);
        self.repos = git::scan_all_repos(&self.repo_paths);
        self.clamp_selection();
    }

    pub fn rescan_status(&mut self) {
        self.repos = git::scan_all_repos(&self.repo_paths);
        self.clamp_selection();
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_repos().len();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn enter_detail(&mut self) {
        let repos = self.filtered_repos();
        if self.selected < repos.len() {
            let repo = &repos[self.selected];
            self.detail = Some(git::get_repo_detail(&repo.path));
            self.detail_tab = DetailTab::Commits;
            self.detail_scroll = 0;
            self.view = View::RepoDetail(self.selected);
        }
    }

    pub fn exit_detail(&mut self) {
        self.view = View::RepoList;
        self.detail = None;
        self.detail_scroll = 0;
    }

    pub fn next_tab(&mut self) {
        self.detail_tab = match self.detail_tab {
            DetailTab::Commits => DetailTab::Changes,
            DetailTab::Changes => DetailTab::Branches,
            DetailTab::Branches => DetailTab::Stashes,
            DetailTab::Stashes => DetailTab::Commits,
        };
        self.detail_scroll = 0;
    }

    pub fn set_tab(&mut self, tab: DetailTab) {
        self.detail_tab = tab;
        self.detail_scroll = 0;
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Name => SortMode::LastCommit,
            SortMode::LastCommit => SortMode::DirtyCount,
            SortMode::DirtyCount => SortMode::Name,
        };
        self.clamp_selection();
    }

    pub fn start_filter(&mut self) {
        self.filter_active = true;
        self.filter_input = self.filter_text.clone();
    }

    pub fn apply_filter(&mut self) {
        self.filter_text = self.filter_input.clone();
        self.filter_active = false;
        self.selected = 0;
    }

    pub fn cancel_filter(&mut self) {
        self.filter_active = false;
        self.filter_input.clear();
    }

    pub fn detail_scroll_down(&mut self) {
        let max = self.detail_item_count();
        if max > 0 {
            self.detail_scroll = (self.detail_scroll + 1).min(max - 1);
        }
    }

    pub fn detail_scroll_up(&mut self) {
        if self.detail_scroll > 0 {
            self.detail_scroll -= 1;
        }
    }

    fn detail_item_count(&self) -> usize {
        if let Some(detail) = &self.detail {
            match self.detail_tab {
                DetailTab::Commits => detail.commits.len(),
                DetailTab::Changes => detail.changed_files.len(),
                DetailTab::Branches => detail.branches.len(),
                DetailTab::Stashes => detail.stashes.len(),
            }
        } else {
            0
        }
    }

    fn clamp_selection(&mut self) {
        let count = self.filtered_repos().len();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::RepoInfo;

    fn repo(name: &str, dirty: u32, ts: i64) -> RepoInfo {
        RepoInfo {
            path: PathBuf::from(name),
            name: name.to_string(),
            branch: "main".to_string(),
            ahead: 0,
            behind: 0,
            dirty_count: dirty,
            last_commit_age: "1d ago".to_string(),
            last_commit_message: "msg".to_string(),
            last_commit_timestamp: ts,
            error: None,
        }
    }

    fn app_with(repos: Vec<RepoInfo>) -> App {
        App {
            scan_dir: PathBuf::from("/dev/null"),
            repo_paths: vec![],
            repos,
            selected: 0,
            view: View::RepoList,
            sort_mode: SortMode::Name,
            filter_text: String::new(),
            filter_input: String::new(),
            filter_active: false,
            fetching: false,
            detail: None,
            detail_tab: DetailTab::Commits,
            detail_scroll: 0,
            should_quit: false,
        }
    }

    #[test]
    fn next_tab_cycles_through_all_tabs() {
        let mut app = app_with(vec![]);
        assert_eq!(app.detail_tab, DetailTab::Commits);
        app.next_tab();
        assert_eq!(app.detail_tab, DetailTab::Changes);
        app.next_tab();
        assert_eq!(app.detail_tab, DetailTab::Branches);
        app.next_tab();
        assert_eq!(app.detail_tab, DetailTab::Stashes);
        app.next_tab();
        assert_eq!(app.detail_tab, DetailTab::Commits);
    }

    #[test]
    fn next_tab_resets_scroll() {
        let mut app = app_with(vec![]);
        app.detail_scroll = 7;
        app.next_tab();
        assert_eq!(app.detail_scroll, 0);
    }

    #[test]
    fn cycle_sort_rotates_modes() {
        let mut app = app_with(vec![]);
        assert_eq!(app.sort_mode, SortMode::Name);
        app.cycle_sort();
        assert_eq!(app.sort_mode, SortMode::LastCommit);
        app.cycle_sort();
        assert_eq!(app.sort_mode, SortMode::DirtyCount);
        app.cycle_sort();
        assert_eq!(app.sort_mode, SortMode::Name);
    }

    #[test]
    fn filtered_repos_sorts_by_name_case_insensitively() {
        let app = app_with(vec![repo("Zebra", 0, 1), repo("alpha", 0, 2)]);
        let names: Vec<_> = app.filtered_repos().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["alpha", "Zebra"]);
    }

    #[test]
    fn filtered_repos_sorts_by_last_commit_desc() {
        let mut app = app_with(vec![repo("old", 0, 100), repo("new", 0, 200)]);
        app.sort_mode = SortMode::LastCommit;
        let names: Vec<_> = app.filtered_repos().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["new", "old"]);
    }

    #[test]
    fn filtered_repos_sorts_by_dirty_count_desc() {
        let mut app = app_with(vec![repo("clean", 0, 1), repo("messy", 9, 1)]);
        app.sort_mode = SortMode::DirtyCount;
        let names: Vec<_> = app.filtered_repos().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["messy", "clean"]);
    }

    #[test]
    fn filtered_repos_applies_substring_filter() {
        let mut app = app_with(vec![repo("frontend", 0, 1), repo("backend", 0, 1)]);
        app.filter_text = "END".to_string();
        let names: Vec<_> = app.filtered_repos().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["backend", "frontend"]);

        app.filter_text = "front".to_string();
        let names: Vec<_> = app.filtered_repos().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["frontend"]);
    }

    #[test]
    fn select_next_and_prev_stay_in_bounds() {
        let mut app = app_with(vec![repo("a", 0, 1), repo("b", 0, 1)]);
        app.select_prev();
        assert_eq!(app.selected, 0); // can't go below 0
        app.select_next();
        assert_eq!(app.selected, 1);
        app.select_next();
        assert_eq!(app.selected, 1); // clamped at last index
        app.select_prev();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn apply_filter_commits_input_and_resets_selection() {
        let mut app = app_with(vec![repo("a", 0, 1), repo("b", 0, 1)]);
        app.selected = 1;
        app.start_filter();
        app.filter_input = "a".to_string();
        app.apply_filter();
        assert_eq!(app.filter_text, "a");
        assert!(!app.filter_active);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn cancel_filter_discards_input() {
        let mut app = app_with(vec![]);
        app.start_filter();
        app.filter_input = "draft".to_string();
        app.cancel_filter();
        assert!(!app.filter_active);
        assert!(app.filter_input.is_empty());
        assert!(app.filter_text.is_empty());
    }

    #[test]
    fn exit_detail_clears_detail_state() {
        let mut app = app_with(vec![]);
        app.view = View::RepoDetail(0);
        app.detail_scroll = 5;
        app.exit_detail();
        assert!(matches!(app.view, View::RepoList));
        assert!(app.detail.is_none());
        assert_eq!(app.detail_scroll, 0);
    }
}
