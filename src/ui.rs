use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;

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

    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let prefix = if index == app.selected_item {
                "> "
            } else {
                "  "
            };
            let style = if index == app.selected_item {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{}", item.label()),
                style,
            )))
        })
        .collect();

    let menu = List::new(items).block(Block::default().title("MVP Menu").borders(Borders::ALL));
    frame.render_widget(menu, chunks[1]);

    let demo_combo = app
        .demo_combo
        .as_ref()
        .map(|combo| format!("Demo combo loaded: {} steps.", combo.steps().len()))
        .unwrap_or_else(|| "No demo combo loaded.".to_string());

    let help = Paragraph::new(format!(
        "{demo_combo} Use Up/Down to move. Press q or Esc to quit."
    ))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}
