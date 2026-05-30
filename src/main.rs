mod app;
mod git;
mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::{App, DetailTab, View};

#[derive(Parser)]
#[command(name = "git-dash", about = "Multi-repo git dashboard TUI")]
struct Cli {
    /// Directory to scan for git repos
    #[arg(short, long, default_value_t = default_scan_dir())]
    dir: String,
}

fn default_scan_dir() -> String {
    std::env::var("HOME")
        .map(|h| format!("{}/projects", h))
        .unwrap_or_else(|_| ".".into())
}

/// Messages from background threads
enum BgMessage {
    FetchComplete,
    AutoRefresh,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let scan_dir = PathBuf::from(&cli.dir);

    if !scan_dir.is_dir() {
        eprintln!("Error: {} is not a directory", scan_dir.display());
        std::process::exit(1);
    }

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(scan_dir);

    // Background message channel
    let (tx, rx) = mpsc::channel::<BgMessage>();

    // Auto-refresh timer thread
    let tx_timer = tx.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(30));
        if tx_timer.send(BgMessage::AutoRefresh).is_err() {
            break;
        }
    });

    // Main loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Check background messages (non-blocking)
        if let Ok(msg) = rx.try_recv() {
            match msg {
                BgMessage::FetchComplete => {
                    app.fetching = false;
                    app.rescan_status();
                }
                BgMessage::AutoRefresh => {
                    app.rescan_status();
                }
            }
        }

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Filter input mode
                if app.filter_active {
                    match key.code {
                        KeyCode::Enter => app.apply_filter(),
                        KeyCode::Esc => app.cancel_filter(),
                        KeyCode::Backspace => {
                            app.filter_input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.filter_input.push(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                match &app.view {
                    View::RepoList => match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                        KeyCode::Enter => app.enter_detail(),
                        KeyCode::Char('f') if !app.fetching => {
                            app.fetching = true;
                            let paths = app.repo_paths.clone();
                            let tx_fetch = tx.clone();
                            thread::spawn(move || {
                                git::fetch_all_repos(&paths);
                                let _ = tx_fetch.send(BgMessage::FetchComplete);
                            });
                        }
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Char('/') => app.start_filter(),
                        KeyCode::Char('s') => app.cycle_sort(),
                        KeyCode::Home | KeyCode::Char('g') => app.selected = 0,
                        KeyCode::End | KeyCode::Char('G') => {
                            let count = app.filtered_repos().len();
                            if count > 0 {
                                app.selected = count - 1;
                            }
                        }
                        _ => {}
                    },
                    View::RepoDetail(_) => match key.code {
                        KeyCode::Esc | KeyCode::Backspace => app.exit_detail(),
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::Char('1') => app.set_tab(DetailTab::Commits),
                        KeyCode::Char('2') => app.set_tab(DetailTab::Changes),
                        KeyCode::Char('3') => app.set_tab(DetailTab::Branches),
                        KeyCode::Char('4') => app.set_tab(DetailTab::Stashes),
                        KeyCode::Down | KeyCode::Char('j') => app.detail_scroll_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.detail_scroll_up(),
                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
