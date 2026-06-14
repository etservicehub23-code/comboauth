use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::activation::ActivationResult;
use crate::app::{App, Screen, ServicesPhase};

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

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(chunks[1]);

    render_sidebar(frame, app, body[0]);

    match app.current_screen {
        Screen::Home => render_home(frame, app, body[1]),
        Screen::Services => render_services(frame, app, body[1]),
        Screen::Combos => render_combos(frame, app, body[1]),
        Screen::TestLab => render_test_lab(frame, app, body[1]),
        Screen::Settings => render_settings(frame, app, body[1]),
        Screen::RecordCombo => render_record_combo(frame, app, body[1]),
        Screen::Quit => {}
    }

    let demo_combo = app
        .demo_combo
        .as_ref()
        .map(|combo| format!("Demo combo loaded: {} steps.", combo.steps().len()))
        .unwrap_or_else(|| "No demo combo loaded.".to_string());

    let status_text = if let Some(secs) = app.clipboard_secs_remaining() {
        format!("Clipboard clears in {secs}s  |  {demo_combo}")
    } else {
        format!("{demo_combo} Up/Down: select. Enter: open. Left/Right: screens. Esc: home. Ctrl-K: quick launch. q: quit.")
    };
    let status_style = if app.clipboard_secs_remaining().is_some() {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let help = Paragraph::new(Line::from(Span::styled(status_text, status_style)))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);

    if app.quick_launch_open {
        render_quick_launch_popup(frame, app, area);
    }
}

fn render_sidebar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .service_registry
        .services()
        .iter()
        .map(|service| {
            let label = format!("{} [{}]", service.name, service.status.label());
            ListItem::new(Line::from(Span::raw(label)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Services")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_quick_launch_popup(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let popup_width = 40u16;
    let popup_height = 5u16;
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    let step_count = app.quick_launch_tokens.len();
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Input: {step_count} steps captured"),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Enter: activate  Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("Quick Launch  Ctrl-K")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(content, popup_area);
}

fn render_home(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .home_items
        .iter()
        .enumerate()
        .map(|(index, item)| selectable_item(index, app.selected_home_item, item.label()))
        .collect();

    let menu = List::new(items).block(Block::default().title("Home").borders(Borders::ALL));
    frame.render_widget(menu, area);
}

fn render_services(frame: &mut Frame<'_>, app: &App, area: Rect) {
    match app.services_phase {
        ServicesPhase::List => render_services_list(frame, app, area),
        ServicesPhase::AddName => render_services_add_name(frame, app, area),
        ServicesPhase::AssignCombo => render_services_assign_combo(frame, app, area),
    }
}

fn render_services_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .service_registry
        .services()
        .iter()
        .enumerate()
        .map(|(index, service)| {
            let user = if service.username.is_empty() {
                "-".to_owned()
            } else {
                service.username.clone()
            };
            selectable_item(
                index,
                app.selected_detail_item,
                format!("{} | user: {} | {}", service.name, user, service.status.label()),
            )
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Services  n: add service  a: assign combo")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn render_services_add_name(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let name_display = if app.service_name_input.is_empty() {
        "_".to_owned()
    } else {
        format!("{}_", app.service_name_input)
    };

    let detail = Paragraph::new(vec![
        Line::from("Add a new service entry"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Name: "),
            Span::styled(
                name_display,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Type a name, then press Enter to save",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().title("Services — Add New").borders(Borders::ALL));
    frame.render_widget(detail, area);
}

fn render_services_assign_combo(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(4)])
        .split(area);

    let services = app.service_registry.services();
    let (service_name, current_status) = services
        .get(app.selected_detail_item)
        .map(|s| (s.name.clone(), s.status.label()))
        .unwrap_or_else(|| ("(none)".to_owned(), ""));

    let service_info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Service: "),
            Span::styled(service_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(current_status, Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default().title("Assign Combo — select and press Enter  Esc: cancel").borders(Borders::ALL));
    frame.render_widget(service_info, chunks[0]);

    let items: Vec<ListItem> = app
        .combo_profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| {
            selectable_item(
                index,
                app.services_assign_cursor,
                profile.name.clone(),
            )
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Saved Combos")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, chunks[1]);
}

fn render_combos(frame: &mut Frame<'_>, app: &App, area: Rect) {
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

fn render_test_lab(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let recorded = app.recorded_combo_input();
    let input_span = if recorded.is_empty() {
        Span::styled("(nothing yet)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(recorded, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    };

    let result_line = match &app.last_activation {
        ActivationResult::Waiting => Line::from(Span::styled(
            "Enter your combo, then press Enter.",
            Style::default().fg(Color::DarkGray),
        )),
        ActivationResult::Activated { service_name, .. } => {
            let label = match app.clipboard_secs_remaining() {
                Some(n) => format!("Activated: {service_name} — clipboard clears in {n}s"),
                None => format!("Activated: {service_name} — copied to clipboard"),
            };
            Line::from(Span::styled(
                label,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))
        }
        ActivationResult::NoMatch => Line::from(Span::styled(
            "No match — unrecognised combo.",
            Style::default().fg(Color::Red),
        )),
        ActivationResult::InvalidInput => Line::from(Span::styled(
            "Nothing to test — enter some keys first.",
            Style::default().fg(Color::Red),
        )),
        ActivationResult::TimingMismatch => Line::from(Span::styled(
            "Sequence matched but rhythm was off.",
            Style::default().fg(Color::Yellow),
        )),
        ActivationResult::NoServiceForCombo { combo_name, .. } => Line::from(Span::styled(
            format!("No service assigned to combo: {combo_name}"),
            Style::default().fg(Color::Yellow),
        )),
        ActivationResult::SecretUnavailable { service_name, .. } => Line::from(Span::styled(
            format!("Secret unavailable for: {service_name}"),
            Style::default().fg(Color::Red),
        )),
    };

    let detail = Paragraph::new(vec![
        Line::from("Enter the combo you remember from muscle memory."),
        Line::from("On Enter, the system will check it against all saved combos."),
        Line::from(""),
        Line::from(vec![Span::raw("Input: "), input_span]),
        Line::from(""),
        result_line,
        Line::from(""),
        Line::from(Span::styled(
            "Arrows / u d l r a b x y  diagonals: 7=UL 9=UR 1=DL 3=DR",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Enter: test  |  Backspace: undo  |  c: clear  |  Esc: exit",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .title("Test Lab — blind combo entry")
            .borders(Borders::ALL),
    );
    frame.render_widget(detail, area);
}

fn render_settings(frame: &mut Frame<'_>, app: &App, area: Rect) {
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

fn render_record_combo(frame: &mut Frame<'_>, app: &App, area: Rect) {
    use crate::app::RecordPhase;

    let (phase_label, body_lines) = match app.record_phase {
        RecordPhase::NameEntry => {
            let name_display = if app.record_name_input.is_empty() {
                "_".to_string()
            } else {
                format!("{}_", app.record_name_input)
            };
            let lines = vec![
                Line::from("Step 1 of 2: Enter a name for this combo"),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Name: "),
                    Span::styled(
                        name_display,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Type a name, then press Enter to continue",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "Esc to cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            ("Name Entry", lines)
        }
        RecordPhase::TokenCapture => {
            let step_count = app.recorded_combo_tokens.len();
            let token_display = if step_count == 0 {
                Span::styled("(no tokens yet)", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(
                    format!("{step_count} steps captured"),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )
            };
            let gap_count = app.recorded_gaps_ms().len();
            let gap_label = match gap_count {
                0 => "No gaps yet".to_string(),
                1 => "1 gap captured".to_string(),
                n => format!("{n} gaps captured"),
            };
            let lines = vec![
                Line::from("Step 2 of 2: Record the combo sequence"),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Name:   "),
                    Span::styled(
                        app.record_name_input.clone(),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![Span::raw("Tokens: "), token_display]),
                Line::from(vec![
                    Span::raw("Timing: "),
                    Span::styled(gap_label, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Keys: u/d/l/r/a/b/x/y  diagonals: 7=UL 9=UR 1=DL 3=DR",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "Arrow keys also record steps.  Backspace: undo  c: clear",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(": save    "),
                    Span::styled("Esc", Style::default().fg(Color::Red)),
                    Span::raw(": cancel"),
                ]),
            ];
            ("Token Capture", lines)
        }
    };

    let title = format!("Record New Combo — {phase_label}");
    let detail = Paragraph::new(body_lines)
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(detail, area);
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
