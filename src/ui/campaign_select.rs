use crate::app::App;
use crate::logic::calculate_total_premium_sold;
use ratatui::{prelude::*, widgets::*};

pub fn draw_campaign_select(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let total_premium = calculate_total_premium_sold(&app.trades);
    let block = Block::default()
        .title(format!("Select Campaign [n: new, ↑/↓: move, Enter: select, q: quit] | Total Premium Sold: ${total_premium:.2}"))
        .borders(Borders::ALL);
    let items: Vec<ListItem> = app
        .campaigns
        .iter()
        .map(|c| ListItem::new(c.name.clone()))
        .collect();
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_stateful_widget(list, size, &mut app.campaign_list_state);
}
