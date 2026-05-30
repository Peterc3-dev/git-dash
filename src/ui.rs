use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs},
    Frame,
};

use crate::app::{App, DetailTab, SortMode, View};

// Phosphor-green palette
const GREEN_BRIGHT: Color = Color::Rgb(0, 255, 200);
const GREEN_DIM: Color = Color::Rgb(0, 128, 100);
const GREEN_DARK: Color = Color::Rgb(0, 50, 40);
const CYAN: Color = Color::Rgb(0, 220, 255);
const YELLOW: Color = Color::Rgb(255, 220, 0);
const RED: Color = Color::Rgb(255, 80, 80);
const BG: Color = Color::Rgb(10, 10, 10);
const FG: Color = Color::Rgb(0, 200, 156);

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG)), size);

    match &app.view {
        View::RepoList => draw_repo_list(f, app, size),
        View::RepoDetail(_idx) => draw_repo_detail(f, app, size),
    }

    // Draw filter overlay if active
    if app.filter_active {
        draw_filter_overlay(f, app, size);
    }
}

fn draw_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // summary bar
            Constraint::Min(5),    // repo table
            Constraint::Length(3), // help bar
        ])
        .split(area);

    draw_summary_bar(f, app, chunks[0]);
    draw_repo_table(f, app, chunks[1]);
    draw_help_bar(f, app, chunks[2]);
}

fn draw_summary_bar(f: &mut Frame, app: &App, area: Rect) {
    let repos = &app.filtered_repos();
    let total = repos.len();
    let clean = repos
        .iter()
        .filter(|r| r.dirty_count == 0 && r.behind == 0 && r.ahead == 0)
        .count();
    let dirty = repos.iter().filter(|r| r.dirty_count > 0).count();
    let behind = repos.iter().filter(|r| r.behind > 0).count();
    let total_changes: u32 = repos.iter().map(|r| r.dirty_count).sum();

    let sort_label = match app.sort_mode {
        SortMode::Name => "name",
        SortMode::LastCommit => "last commit",
        SortMode::DirtyCount => "dirty count",
    };

    let line = Line::from(vec![
        Span::styled("  REPOS: ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            format!("{}", total),
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  CLEAN: ", Style::default().fg(GREEN_DIM)),
        Span::styled(format!("{}", clean), Style::default().fg(GREEN_BRIGHT)),
        Span::styled("  |  DIRTY: ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            format!("{}", dirty),
            Style::default().fg(if dirty > 0 { YELLOW } else { GREEN_BRIGHT }),
        ),
        Span::styled("  |  BEHIND: ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            format!("{}", behind),
            Style::default().fg(if behind > 0 { RED } else { GREEN_BRIGHT }),
        ),
        Span::styled("  |  UNCOMMITTED: ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            format!("{}", total_changes),
            Style::default().fg(if total_changes > 0 {
                YELLOW
            } else {
                GREEN_BRIGHT
            }),
        ),
        Span::styled(
            format!("  |  sort: {}", sort_label),
            Style::default().fg(GREEN_DARK),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN_DARK))
        .title(Span::styled(
            " git-dash ",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG));

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}

fn draw_repo_table(f: &mut Frame, app: &App, area: Rect) {
    let repos = app.filtered_repos();
    let header = Row::new(vec![
        Cell::from("  Repo").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Branch").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("↑/↓").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Dirty").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Last Commit")
            .style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Message").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().bg(BG));

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = i == app.selected;

            // Determine row color based on state
            let row_color = if repo.behind > 0 {
                RED
            } else if repo.dirty_count > 0 {
                YELLOW
            } else if repo.ahead > 0 {
                CYAN
            } else {
                GREEN_BRIGHT
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(row_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(GREEN_BRIGHT)
                    .add_modifier(Modifier::BOLD)
            };

            let base_style = if is_selected {
                Style::default().fg(Color::Black).bg(row_color)
            } else {
                Style::default().fg(row_color)
            };

            let branch_style = if is_selected {
                Style::default().fg(Color::Black).bg(row_color)
            } else {
                Style::default().fg(CYAN)
            };

            let dim_style = if is_selected {
                Style::default().fg(Color::Black).bg(row_color)
            } else {
                Style::default().fg(GREEN_DIM)
            };

            let ahead_behind = if repo.ahead > 0 || repo.behind > 0 {
                format!("↑{} ↓{}", repo.ahead, repo.behind)
            } else {
                "—".into()
            };

            let dirty = if repo.dirty_count > 0 {
                format!("{}", repo.dirty_count)
            } else {
                "✓".into()
            };

            let marker = if is_selected { "▸ " } else { "  " };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, repo.name)).style(name_style),
                Cell::from(repo.branch.clone()).style(branch_style),
                Cell::from(ahead_behind).style(base_style),
                Cell::from(dirty).style(base_style),
                Cell::from(repo.last_commit_age.clone()).style(dim_style),
                Cell::from(repo.last_commit_message.clone()).style(dim_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Fill(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(GREEN_DARK))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    " Repositories ",
                    Style::default().fg(GREEN_BRIGHT),
                )),
        )
        .row_highlight_style(Style::default());

    f.render_widget(table, area);
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let filter_text = if !app.filter_text.is_empty() {
        format!("  filter: \"{}\"", app.filter_text)
    } else {
        String::new()
    };

    let fetching_text = if app.fetching { "  [FETCHING...]" } else { "" };

    let line = Line::from(vec![
        Span::styled(
            "  q",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" detail  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "f",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" fetch  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "r",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" refresh  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "/",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" filter  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "s",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" sort  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            fetching_text,
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(filter_text, Style::default().fg(YELLOW)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN_DARK))
        .style(Style::default().bg(BG));

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}

fn draw_filter_overlay(f: &mut Frame, app: &App, area: Rect) {
    let width = 40.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 2;
    let popup_area = Rect::new(x, y, width, 3);

    f.render_widget(Clear, popup_area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("/ ", Style::default().fg(GREEN_BRIGHT)),
        Span::styled(&app.filter_input, Style::default().fg(FG)),
        Span::styled("█", Style::default().fg(GREEN_BRIGHT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_BRIGHT))
            .title(Span::styled(
                " Filter repos ",
                Style::default()
                    .fg(GREEN_BRIGHT)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(BG)),
    );

    f.render_widget(input, popup_area);
}

fn draw_repo_detail(f: &mut Frame, app: &App, area: Rect) {
    let idx = match &app.view {
        View::RepoDetail(i) => *i,
        _ => return,
    };

    let repos = app.filtered_repos();
    if idx >= repos.len() {
        return;
    }
    let repo = &repos[idx];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(2), // tabs
            Constraint::Min(5),    // content
            Constraint::Length(3), // help
        ])
        .split(area);

    // Header
    let header_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            &repo.name,
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  on  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            &repo.branch,
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", repo.path.display()),
            Style::default().fg(GREEN_DARK),
        ),
    ]);
    let header = Paragraph::new(header_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(header, chunks[0]);

    // Tabs
    let tab_titles = vec!["Commits", "Changes", "Branches", "Stashes"];
    let tabs = Tabs::new(tab_titles)
        .select(match app.detail_tab {
            DetailTab::Commits => 0,
            DetailTab::Changes => 1,
            DetailTab::Branches => 2,
            DetailTab::Stashes => 3,
        })
        .style(Style::default().fg(GREEN_DIM).bg(BG))
        .highlight_style(
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled(" │ ", Style::default().fg(GREEN_DARK)));
    f.render_widget(tabs, chunks[1]);

    // Content area
    if let Some(detail) = &app.detail {
        match app.detail_tab {
            DetailTab::Commits => draw_commits(f, detail, app, chunks[2]),
            DetailTab::Changes => draw_changes(f, detail, app, chunks[2]),
            DetailTab::Branches => draw_branches(f, detail, app, chunks[2]),
            DetailTab::Stashes => draw_stashes(f, detail, app, chunks[2]),
        }
    } else {
        let loading = Paragraph::new("  Loading...")
            .style(Style::default().fg(GREEN_DIM).bg(BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(GREEN_DARK))
                    .style(Style::default().bg(BG)),
            );
        f.render_widget(loading, chunks[2]);
    }

    // Help
    let help_line = Line::from(vec![
        Span::styled(
            "  Esc/Backspace",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" back  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "Tab/1-4",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" switch tab  ", Style::default().fg(GREEN_DIM)),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" scroll", Style::default().fg(GREEN_DIM)),
    ]);
    let help = Paragraph::new(help_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(help, chunks[3]);
}

fn draw_commits(f: &mut Frame, detail: &crate::git::RepoDetail, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Hash").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Author").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("When").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
        Cell::from("Message").style(Style::default().fg(GREEN_DIM).add_modifier(Modifier::BOLD)),
    ]);

    let visible_height = area.height.saturating_sub(3) as usize; // borders + header
    let total = detail.commits.len();
    let offset = compute_scroll_offset(app.detail_scroll, total, visible_height);

    let rows: Vec<Row> = detail
        .commits
        .iter()
        .skip(offset)
        .take(visible_height)
        .enumerate()
        .map(|(i, c)| {
            let is_sel = i + offset == app.detail_scroll;
            let hash_style = if is_sel {
                Style::default().fg(Color::Black).bg(GREEN_DIM)
            } else {
                Style::default().fg(GREEN_DIM)
            };
            let style = if is_sel {
                Style::default().fg(Color::Black).bg(GREEN_DIM)
            } else {
                Style::default().fg(FG)
            };
            Row::new(vec![
                Cell::from(c.hash.clone()).style(hash_style),
                Cell::from(c.author.clone()).style(style),
                Cell::from(c.date.clone()).style(style),
                Cell::from(c.message.clone()).style(style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(16),
        Constraint::Length(14),
        Constraint::Fill(1),
    ];

    let title = format!(" Commits ({}) ", detail.commits.len());
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG))
            .title(Span::styled(title, Style::default().fg(GREEN_BRIGHT))),
    );

    f.render_widget(table, area);
}

fn draw_changes(f: &mut Frame, detail: &crate::git::RepoDetail, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = detail.changed_files.len();
    let offset = compute_scroll_offset(app.detail_scroll, total, visible_height);

    let lines: Vec<Line> = if detail.changed_files.is_empty() {
        vec![Line::from(Span::styled(
            "  Working tree clean ✓",
            Style::default().fg(GREEN_BRIGHT),
        ))]
    } else {
        detail
            .changed_files
            .iter()
            .skip(offset)
            .take(visible_height)
            .enumerate()
            .map(|(i, fs)| {
                let is_sel = i + offset == app.detail_scroll;
                let status_color = match fs.status.as_str() {
                    "M" => YELLOW,
                    "A" => GREEN_BRIGHT,
                    "D" => RED,
                    "??" => Color::Rgb(150, 150, 150),
                    _ => FG,
                };
                if is_sel {
                    Line::from(vec![
                        Span::styled(
                            format!("  {:>2} ", fs.status),
                            Style::default().fg(Color::Black).bg(status_color),
                        ),
                        Span::styled(&fs.path, Style::default().fg(Color::Black).bg(status_color)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("  {:>2} ", fs.status),
                            Style::default()
                                .fg(status_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&fs.path, Style::default().fg(FG)),
                    ])
                }
            })
            .collect()
    };

    let title = format!(" Changes ({}) ", detail.changed_files.len());
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG))
            .title(Span::styled(title, Style::default().fg(GREEN_BRIGHT))),
    );
    f.render_widget(paragraph, area);
}

fn draw_branches(f: &mut Frame, detail: &crate::git::RepoDetail, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = detail.branches.len();
    let offset = compute_scroll_offset(app.detail_scroll, total, visible_height);

    let lines: Vec<Line> = detail
        .branches
        .iter()
        .skip(offset)
        .take(visible_height)
        .enumerate()
        .map(|(i, b)| {
            let is_sel = i + offset == app.detail_scroll;
            let marker = if b.is_current { "▸ " } else { "  " };
            let tracking_info = b
                .tracking
                .as_ref()
                .map(|t| format!(" → {}", t))
                .unwrap_or_default();

            if is_sel {
                Line::from(vec![
                    Span::styled(
                        format!("  {}{}", marker, b.name),
                        Style::default().fg(Color::Black).bg(CYAN),
                    ),
                    Span::styled(tracking_info, Style::default().fg(Color::Black).bg(CYAN)),
                ])
            } else {
                let name_color = if b.is_current { GREEN_BRIGHT } else { CYAN };
                Line::from(vec![
                    Span::styled(
                        format!("  {}{}", marker, b.name),
                        Style::default().fg(name_color),
                    ),
                    Span::styled(tracking_info, Style::default().fg(GREEN_DIM)),
                ])
            }
        })
        .collect();

    let title = format!(" Branches ({}) ", detail.branches.len());
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG))
            .title(Span::styled(title, Style::default().fg(GREEN_BRIGHT))),
    );
    f.render_widget(paragraph, area);
}

fn draw_stashes(f: &mut Frame, detail: &crate::git::RepoDetail, app: &App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = detail.stashes.len();
    let offset = compute_scroll_offset(app.detail_scroll, total, visible_height);

    let lines: Vec<Line> = if detail.stashes.is_empty() {
        vec![Line::from(Span::styled(
            "  No stashes",
            Style::default().fg(GREEN_DIM),
        ))]
    } else {
        detail
            .stashes
            .iter()
            .skip(offset)
            .take(visible_height)
            .enumerate()
            .map(|(i, s)| {
                let is_sel = i + offset == app.detail_scroll;
                if is_sel {
                    Line::from(Span::styled(
                        format!("  {}", s),
                        Style::default().fg(Color::Black).bg(YELLOW),
                    ))
                } else {
                    Line::from(Span::styled(
                        format!("  {}", s),
                        Style::default().fg(YELLOW),
                    ))
                }
            })
            .collect()
    };

    let title = format!(" Stashes ({}) ", detail.stashes.len());
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GREEN_DARK))
            .style(Style::default().bg(BG))
            .title(Span::styled(title, Style::default().fg(GREEN_BRIGHT))),
    );
    f.render_widget(paragraph, area);
}

fn compute_scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    if selected < visible / 2 {
        0
    } else if selected >= total.saturating_sub(visible / 2) {
        total.saturating_sub(visible)
    } else {
        selected.saturating_sub(visible / 2)
    }
}
