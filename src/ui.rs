use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, ComboTestResult, Screen, VaultState};
use crate::combo::MatchState;

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            "ComboAuth",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("Arcade-style combo input for a future password workflow"),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    match app.current_screen {
        Screen::Home => render_home(frame, app, chunks[1]),
        Screen::Services => render_services(frame, app, chunks[1]),
        Screen::Combos => render_combos(frame, app, chunks[1]),
        Screen::TestLab => render_test_lab(frame, app, chunks[1]),
        Screen::Settings => render_settings(frame, app, chunks[1]),
        Screen::Quit => {}
    }

    let demo_combo = app
        .demo_combo
        .as_ref()
        .map(|combo| format!("Demo combo loaded: {} steps.", combo.steps().len()))
        .unwrap_or_else(|| "No demo combo loaded.".to_string());

    let help = Paragraph::new(format!(
        "{demo_combo} Up/Down: select. Enter: open. Left/Right: screens. Esc: home. q: quit."
    ))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

fn render_home(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .home_items
        .iter()
        .enumerate()
        .map(|(index, item)| selectable_item(index, app.selected_home_item, item.label()))
        .collect();

    let menu = List::new(items).block(Block::default().title("Home").borders(Borders::ALL));
    frame.render_widget(menu, area);
}

fn render_services(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .services
        .iter()
        .enumerate()
        .map(|(index, service)| {
            selectable_item(
                index,
                app.selected_detail_item,
                format!(
                    "{} | user: {} | combo: {}",
                    service.name, service.username, service.combo_hint
                ),
            )
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Services - mocked entries")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_combos(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .combo_profiles
        .iter()
        .enumerate()
        .map(|(index, combo)| {
            selectable_item(
                index,
                app.selected_detail_item,
                format!("{} | {} | {}", combo.name, combo.sequence, combo.status),
            )
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Combos - parser prototypes")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_test_lab(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let predefined: Vec<ListItem> = app
        .combo_profiles
        .iter()
        .enumerate()
        .map(|(index, combo)| {
            selectable_item(
                index,
                app.selected_detail_item,
                format!("{} | {}", combo.name, combo.sequence),
            )
        })
        .collect();

    let predefined_list = List::new(predefined).block(
        Block::default()
            .title("Predefined combos  [Tab to cycle]")
            .borders(Borders::ALL),
    );
    frame.render_widget(predefined_list, chunks[0]);

    let recorded = app.recorded_combo_input();
    let recorded = if recorded.is_empty() {
        "(empty)".to_string()
    } else {
        recorded
    };

    let selected = app
        .selected_timed_combo()
        .map(|tc| {
            format!(
                "{} | {} steps | window: {} ms",
                app.selected_combo_profile().map(|p| p.name).unwrap_or(""),
                tc.combo.len(),
                tc.timing.window_ms,
            )
        })
        .unwrap_or_else(|| "No predefined combo selected.".to_string());

    let progress = combo_progress_line(
        app.selected_combo_profile().map(|p| p.sequence),
        app.prefix_match_state(),
    );

    let result = match &app.test_result {
        ComboTestResult::Waiting => "Waiting for test.".to_string(),
        ComboTestResult::Match(name) => format!("Match: {name}."),
        ComboTestResult::NoMatch => "No match.".to_string(),
        ComboTestResult::InvalidInput => "Invalid or empty combo.".to_string(),
    };

    let (vault_label, vault_style) = match &app.vault_state {
        VaultState::Locked => (
            "Vault: [locked]".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        VaultState::Unlocked { service, placeholder } => (
            format!("Vault: {service} -> {placeholder}"),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
    };

    let detail = Paragraph::new(vec![
        Line::from(Span::styled(
            "Recording",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(recorded),
        progress,
        Line::from(""),
        Line::from(selected),
        Line::from(result),
        Line::from(""),
        Line::from(Span::styled(vault_label, vault_style)),
        Line::from(""),
        Line::from("Arrows/u/d/l/r/a/b/x/y  diagonals: 7=UL 9=UR 1=DL 3=DR"),
        Line::from("Enter: test | Tab: cycle predefined | p: load | c: clear | Bksp: undo | Esc: exit"),
    ])
    .block(
        Block::default()
            .title("Test Lab - mock only")
            .borders(Borders::ALL),
    );
    frame.render_widget(detail, chunks[1]);
}

fn render_settings(frame: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .settings
        .iter()
        .enumerate()
        .map(|(index, setting)| {
            selectable_item(
                index,
                app.selected_detail_item,
                format!("{}: {}", setting.name, setting.value),
            )
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Settings - no persistence")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn selectable_item(
    index: usize,
    selected_index: usize,
    label: impl Into<String>,
) -> ListItem<'static> {
    let prefix = if index == selected_index { "> " } else { "  " };
    let style = if index == selected_index {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(Span::styled(
        format!("{prefix}{}", label.into()),
        style,
    )))
}

fn combo_progress_line(profile_seq: Option<&str>, state: Option<MatchState>) -> Line<'static> {
    let seq = match profile_seq {
        Some(s) if !s.is_empty() => s,
        _ => {
            return Line::from(Span::styled(
                "  No combo selected.",
                Style::default().fg(Color::DarkGray),
            ))
        }
    };

    let steps: Vec<String> = seq.split_whitespace().map(|s| s.to_owned()).collect();
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(steps.len() * 2);

    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let style = match &state {
            None => Style::default().fg(Color::DarkGray),
            Some(MatchState::Full) => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            Some(MatchState::TooLong) => Style::default().fg(Color::Red),
            Some(MatchState::Partial { matched, .. }) => {
                if i < *matched {
                    Style::default().fg(Color::Green)
                } else if i == *matched {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            }
            Some(MatchState::Mismatch { at }) => {
                if i < *at {
                    Style::default().fg(Color::Green)
                } else if i == *at {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            }
        };
        spans.push(Span::styled(format!("[{step}]"), style));
    }

    let (suffix, suffix_style): (&'static str, Style) = match &state {
        None => ("  <- enter combo above", Style::default().fg(Color::DarkGray)),
        Some(MatchState::Full) => ("  -> press Enter", Style::default().fg(Color::Green)),
        Some(MatchState::TooLong) => ("  -> too long", Style::default().fg(Color::Red)),
        Some(MatchState::Mismatch { .. }) => ("  -> wrong step", Style::default().fg(Color::Red)),
        Some(MatchState::Partial { .. }) => ("", Style::default()),
    };
    if !suffix.is_empty() {
        spans.push(Span::styled(suffix, suffix_style));
    }

    Line::from(spans)
}
