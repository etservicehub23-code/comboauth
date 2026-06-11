mod app;
mod combo;
mod error;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use app::App;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use error::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let app_result = run_app(&mut terminal, App::default());
    restore_terminal(&mut terminal)?;
    app_result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(output);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Char('c') if app.is_test_lab() => app.clear_recorded_combo(),
                    KeyCode::Char('p') if app.is_test_lab() => app.load_selected_test_combo(),
                    KeyCode::Char(value) if app.is_test_lab() => {
                        app.record_combo_shortcut(value);
                    }
                    KeyCode::Esc => app.go_home(),
                    KeyCode::Backspace if app.is_test_lab() => app.pop_recorded_combo_token(),
                    KeyCode::Backspace => app.go_home(),
                    KeyCode::Up => app.previous_item(),
                    KeyCode::Down => app.next_item(),
                    KeyCode::Left => app.previous_screen(),
                    KeyCode::Right => app.next_screen(),
                    KeyCode::Enter if app.is_test_lab() => app.test_recorded_combo(),
                    KeyCode::Enter => app.activate_selected(),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
