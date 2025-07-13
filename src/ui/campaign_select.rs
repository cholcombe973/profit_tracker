use crate::app::App;
use crate::logic::{calculate_total_premium_sold, calculate_weekly_premium};
use ratatui::{prelude::*, widgets::*};

pub fn draw_campaign_select(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let total_premium = calculate_total_premium_sold(&app.trades);
    let weekly_premium = calculate_weekly_premium(&app.trades);

    // Create colored spans for the title
    let title_spans = vec![
        Span::raw("Select Campaign [n: new, ↑/↓: move, Enter: select, q: quit] | "),
        Span::styled(
            format!("Total Premium: ${total_premium:.2}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("This Week: ${weekly_premium:.2}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let block = Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL);
    let items: Vec<ListItem> = app
        .campaigns
        .iter()
        .map(|c| ListItem::new(c.name.clone()))
        .collect();
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_stateful_widget(list, size, &mut app.campaign_list_state);
}
