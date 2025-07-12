use crate::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn draw_new_campaign(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default()
        .title("New Campaign [Tab: switch, Enter: save, ESC: cancel]")
        .borders(Borders::ALL);
    let name_focus = if app.new_campaign_field == 0 {
        " <"
    } else {
        ""
    };
    let symbol_focus = if app.new_campaign_field == 1 {
        " <"
    } else {
        ""
    };
    let price_focus = if app.new_campaign_field == 2 {
        " <"
    } else {
        ""
    };
    let content = format!(
        "Name: {}{}\nSymbol: {}{}\nTarget Exit Price: {}{}",
        app.new_campaign_name,
        name_focus,
        app.new_campaign_symbol,
        symbol_focus,
        app.new_campaign_target_price,
        price_focus
    );
    let para = Paragraph::new(content).block(block);
    f.render_widget(para, size);
}
