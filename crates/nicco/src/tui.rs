use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use nicco_core::{Snapshot, Source};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io::{self, IsTerminal};

trait TerminalOps {
    fn raw(&mut self) -> io::Result<()>;
    fn enter(&mut self) -> io::Result<()>;
    fn restore_screen(&mut self) -> io::Result<()>;
    fn restore_raw(&mut self) -> io::Result<()>;
}

struct RealTerminal;
impl TerminalOps for RealTerminal {
    fn raw(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }
    fn enter(&mut self) -> io::Result<()> {
        execute!(io::stdout(), EnterAlternateScreen)
    }
    fn restore_screen(&mut self) -> io::Result<()> {
        execute!(io::stdout(), crossterm::cursor::Show, LeaveAlternateScreen)
    }
    fn restore_raw(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }
}

// Construct the guard before setup so partial setup failures also restore state.
struct Session<T: TerminalOps> {
    ops: T,
    raw_attempted: bool,
    screen_attempted: bool,
}
impl<T: TerminalOps> Session<T> {
    fn open(ops: T) -> io::Result<Self> {
        let mut session = Self {
            ops,
            raw_attempted: false,
            screen_attempted: false,
        };
        session.raw_attempted = true;
        session.ops.raw()?;
        session.screen_attempted = true;
        session.ops.enter()?;
        Ok(session)
    }

    fn close(&mut self) -> io::Result<()> {
        let screen = if self.screen_attempted {
            self.ops.restore_screen()
        } else {
            Ok(())
        };
        let raw = if self.raw_attempted {
            self.ops.restore_raw()
        } else {
            Ok(())
        };
        if screen.is_ok() {
            self.screen_attempted = false;
        }
        if raw.is_ok() {
            self.raw_attempted = false;
        }
        screen.and(raw)
    }
}
impl<T: TerminalOps> Drop for Session<T> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

struct App {
    snapshot: Snapshot,
    selected: usize,
    error: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
enum Action {
    Continue,
    Refresh,
    Quit,
}
impl App {
    fn key(&mut self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Continue;
        }
        if key.code == KeyCode::Char('q')
            || key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return Action::Quit;
        }
        match key.code {
            KeyCode::Char('r') => return Action::Refresh,
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected =
                    (self.selected + 1).min(self.snapshot.interfaces.len().saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            _ => {}
        }
        Action::Continue
    }

    fn refreshed(&mut self, result: Result<Snapshot, nicco_core::Error>) {
        match result {
            Ok(snapshot) => {
                // Preserve selection by stable kernel index when possible.
                let index = self.snapshot.interfaces.get(self.selected).map(|i| i.index);
                self.selected = index
                    .and_then(|index| snapshot.interfaces.iter().position(|i| i.index == index))
                    .unwrap_or(0);
                self.snapshot = snapshot;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(2),
        Constraint::Length(3),
    ])
    .split(frame.area());
    let source = match app.snapshot.source {
        Source::Live => "LIVE · read-only",
        Source::Demo => "DEMO · synthetic data",
    };
    frame.render_widget(
        Paragraph::new(format!("nicco — {source}")).block(Block::bordered()),
        areas[0],
    );
    let panes = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(areas[1]);
    let items = app
        .snapshot
        .interfaces
        .iter()
        .map(|i| ListItem::new(format!("{}  {}", i.name, i.oper_state)))
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::bordered().title("Interfaces"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        panes[0],
        &mut state,
    );
    let mut details = match app.snapshot.interfaces.get(app.selected) {
        Some(i) => format!(
            "{} (index {})\nOperational state: {}\nOwner: {}\n\n{}",
            i.name,
            i.index,
            i.oper_state,
            i.owner.as_deref().unwrap_or("unknown"),
            if i.addresses.is_empty() {
                "No addresses observed".into()
            } else {
                i.addresses
                    .iter()
                    .map(|a| format!("{}/{}", a.address, a.prefix_len))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        ),
        None => "No interfaces observed".into(),
    };
    for backend in &app.snapshot.backends {
        details.push_str(&format!(
            "\n\n{} runtime directory: {}\n{}",
            backend.backend, backend.runtime_directory_present, backend.limitation
        ));
    }
    frame.render_widget(
        Paragraph::new(details)
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title("Observation")),
        panes[1],
    );
    let footer = app
        .error
        .as_ref()
        .map(|e| format!("STALE: {e} · r retry · q quit"))
        .unwrap_or_else(|| "↑/↓ or j/k select · r refresh · q/Esc/Ctrl-C quit".into());
    frame.render_widget(
        Paragraph::new(footer)
            .wrap(Wrap { trim: false })
            .block(Block::bordered()),
        areas[2],
    );
}

pub async fn run(demo: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "TUI requires terminal stdin and stdout; use nicco status for pipes",
        )
        .into());
    }
    // Collect before touching terminal state; errors leave the caller's terminal intact.
    let mut app = App {
        snapshot: nicco_core::collect(demo).await?,
        selected: 0,
        error: None,
    };
    let mut session = Session::open(RealTerminal)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result: io::Result<()> = async {
        loop {
            terminal.draw(|frame| draw(frame, &app))?;
            if event::poll(std::time::Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => match app.key(key) {
                        Action::Quit => break,
                        Action::Refresh => {
                            // A refresh is bounded to five seconds. No automatic polling/probes.
                            app.refreshed(nicco_core::collect(demo).await);
                        }
                        Action::Continue => {}
                    },
                    Event::Resize(_, _) => {} // Next draw uses the new frame size.
                    _ => {}
                }
            }
        }
        Ok(())
    }
    .await;
    drop(terminal);
    let cleanup = session.close();
    result?;
    cleanup?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    fn app() -> App {
        App {
            snapshot: Snapshot::demo().unwrap(),
            selected: 0,
            error: None,
        }
    }
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn navigation_quit_and_refresh() {
        let mut app = app();
        app.key(key(KeyCode::Down));
        app.key(key(KeyCode::Down));
        assert_eq!(app.selected, 1);
        assert_eq!(app.key(key(KeyCode::Char('r'))), Action::Refresh);
        assert_eq!(app.key(key(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(
            app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
        app.key(key(KeyCode::Up));
        app.key(key(KeyCode::Up));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn empty_and_failed_refresh_keep_safe_selection() {
        let mut app = app();
        let original = app.snapshot.clone();
        app.refreshed(Err(nicco_core::Error::Timeout));
        assert_eq!(app.snapshot, original);
        assert!(app.error.is_some());
        let mut empty = original;
        empty.interfaces.clear();
        app.refreshed(Ok(empty));
        app.key(key(KeyCode::Down));
        assert_eq!(app.selected, 0);
        assert!(app.error.is_none());
    }

    #[test]
    fn renders_demo_and_resizes_without_a_real_terminal() {
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(90, 20)).unwrap();
        terminal.draw(|f| draw(f, &app())).unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(text.contains("DEMO"));
        assert!(text.contains("192.0.2.10/24"));
        terminal.backend_mut().resize(8, 3);
        terminal.autoresize().unwrap();
        terminal.draw(|f| draw(f, &app())).unwrap();
    }

    struct FakeTerminal {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail: Option<&'static str>,
    }
    impl FakeTerminal {
        fn call(&mut self, name: &'static str) -> io::Result<()> {
            self.calls.borrow_mut().push(name);
            if self.fail == Some(name) {
                Err(io::Error::other("injected failure"))
            } else {
                Ok(())
            }
        }
    }
    impl TerminalOps for FakeTerminal {
        fn raw(&mut self) -> io::Result<()> {
            self.call("raw")
        }
        fn enter(&mut self) -> io::Result<()> {
            self.call("enter")
        }
        fn restore_screen(&mut self) -> io::Result<()> {
            self.call("screen_cleanup")
        }
        fn restore_raw(&mut self) -> io::Result<()> {
            self.call("raw_cleanup")
        }
    }

    #[test]
    fn setup_failures_and_normal_drop_restore_terminal() {
        for fail in [None, Some("raw"), Some("enter"), Some("screen_cleanup")] {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let session = Session::open(FakeTerminal {
                calls: calls.clone(),
                fail,
            });
            drop(session);
            let calls = calls.borrow();
            assert!(calls.contains(&"raw_cleanup"));
            if fail != Some("raw") {
                assert!(calls.contains(&"screen_cleanup"));
            }
        }
    }

    #[test]
    fn panic_unwind_restores_terminal() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let copy = calls.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = Session::open(FakeTerminal {
                calls: copy,
                fail: None,
            })
            .unwrap();
            panic!("injected draw failure");
        }));
        assert_eq!(
            &*calls.borrow(),
            &["raw", "enter", "screen_cleanup", "raw_cleanup"]
        );
    }
}
