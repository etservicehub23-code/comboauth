mod service;
mod app;
mod profile;
mod vault;
mod combo;
mod error;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use app::{App, Screen, ServicesPhase};
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
        app.tick();
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // RecordCombo NameEntry — capture all chars for the name field (must come first)
                    KeyCode::Char(ch) if app.is_record_combo_name_entry() => {
                        app.record_name_push_char(ch);
                    }
                    KeyCode::Backspace if app.is_record_combo_name_entry() => {
                        app.record_name_backspace();
                    }
                    KeyCode::Enter if app.is_record_combo_name_entry() => {
                        app.confirm_name_entry();
                    }
                    KeyCode::Esc if app.is_record_combo() => app.cancel_record_combo(),
                    // RecordCombo TokenCapture — arrow keys record directions (must come before cancel handler)
                    KeyCode::Up if app.is_record_combo_token_capture() => {
                        app.record_combo_token("up");
                    }
                    KeyCode::Down if app.is_record_combo_token_capture() => {
                        app.record_combo_token("down");
                    }
                    KeyCode::Left if app.is_record_combo_token_capture() => {
                        app.record_combo_token("left");
                    }
                    KeyCode::Right if app.is_record_combo_token_capture() => {
                        app.record_combo_token("right");
                    }
                    KeyCode::Left if app.is_record_combo() => app.cancel_record_combo(),
                    KeyCode::Right if app.is_record_combo() => app.cancel_record_combo(),
                    KeyCode::Char('c') if app.is_record_combo_token_capture() => {
                        app.clear_recorded_combo();
                    }
                    KeyCode::Char(value) if app.is_record_combo_token_capture() => {
                        app.record_combo_shortcut(value);
                    }
                    KeyCode::Backspace if app.is_record_combo_token_capture() => {
                        app.pop_recorded_combo_token();
                    }
                    KeyCode::Enter if app.is_record_combo_token_capture() => {
                        app.save_recorded_combo();
                    }
                    // Services — add service name entry (must come before general char/backspace/esc)
                    KeyCode::Char(ch) if app.is_services_add_name() => {
                        app.service_name_push_char(ch);
                    }
                    KeyCode::Backspace if app.is_services_add_name() => {
                        app.service_name_backspace();
                    }
                    KeyCode::Enter if app.is_services_add_name() => {
                        app.save_new_service();
                    }
                    KeyCode::Esc if app.is_services_add_name() => {
                        app.cancel_services_action();
                    }
                    // Services — assign combo picker
                    KeyCode::Enter if app.is_services_assign_combo() => {
                        app.confirm_assign_combo();
                    }
                    KeyCode::Esc if app.is_services_assign_combo() => {
                        app.cancel_services_action();
                    }
                    // General quit (guarded to not fire during name entry)

                    KeyCode::Char('q') => app.quit(),
                    // TestLab
                    KeyCode::Char('c') if app.is_test_lab() => app.clear_recorded_combo(),
                    KeyCode::Char(value) if app.is_test_lab() => {
                        app.record_combo_shortcut(value);
                    }
                    // Services screen actions (only in List phase)
                    KeyCode::Char('n') if app.current_screen == Screen::Services
                        && app.services_phase == ServicesPhase::List =>
                    {
                        app.start_add_service();
                    }
                    KeyCode::Char('a') if app.current_screen == Screen::Services
                        && app.services_phase == ServicesPhase::List =>
                    {
                        app.start_assign_combo();
                    }
                    // Trigger recording from the Combos screen
                    KeyCode::Char('n') if app.current_screen == Screen::Combos => {
                        app.start_record_combo();
                    }
                    KeyCode::Esc => app.go_home(),
                    KeyCode::Backspace if app.is_test_lab() => app.pop_recorded_combo_token(),
                    KeyCode::Backspace => app.go_home(),
                    KeyCode::Up if app.is_test_lab() => app.record_combo_token("up"),
                    KeyCode::Down if app.is_test_lab() => app.record_combo_token("down"),
                    KeyCode::Left if app.is_test_lab() => app.record_combo_token("left"),
                    KeyCode::Right if app.is_test_lab() => app.record_combo_token("right"),
                    KeyCode::Up => app.previous_item(),
                    KeyCode::Down => app.next_item(),
                    KeyCode::Left => app.previous_screen(),
                    KeyCode::Right => app.next_screen(),
                    KeyCode::Enter if app.is_test_lab() => app.test_recorded_combo(),
                    KeyCode::Enter => app.activate_selected(),
                    _ if app.is_test_lab() => {}
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
