use crate::app::{ACTIONS, App};
use ratatui::{prelude::*, widgets::*};

pub fn draw_edit_trade(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let block = Block::default()
        .title(
            "Edit Trade [Tab: next, Shift+Tab: prev, ←/→: change action, Enter: save, ESC: cancel]",
        )
        .borders(Borders::ALL);
    let fields = [
        "Symbol",
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
            let mut content = if i == 1 {
                format!("{}: < {} >", label, ACTIONS[app.edit_action_index])
            } else {
                let idx = if i < 1 { i } else { i - 1 };
                format!("{}: {}", label, app.edit_trade_fields[idx])
            };
            if i == app.edit_form_index {
                content.push_str(" <");
            }
            ListItem::new(content)
        })
        .collect();
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_widget(list, size);
}
