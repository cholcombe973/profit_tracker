use crate::app::{ACTIONS, App};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::*,
};

pub fn draw_add_trade(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default().title("Add Trade [Tab: next, Shift+Tab: prev, ←/→: change action, Enter: submit, ESC: return]").borders(Borders::ALL).style(Style::default().fg(Color::Cyan));
    let fields = [
        "Action",
        "Strike",
        "Delta",
        "Expiration (YYYY-MM-DD)",
        "Date of Action (YYYY-MM-DD)",
        "Shares",
        "Credit",
    ];
    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let content = if i == 0 {
                format!("{}: < {} >", label, ACTIONS[app.action_index])
            } else {
                let idx = i - 1;
                format!("{}: {}", label, app.form_fields[idx])
            };
            let style = if i == app.form_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_widget(list, size);
    if let Some(ref err) = app.form_error {
        let area = Rect {
            x: size.x + 2,
            y: size.y + size.height.saturating_sub(2),
            width: size.width.saturating_sub(4),
            height: 1,
        };
        let error_paragraph = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
        f.render_widget(error_paragraph, area);
    }
}
