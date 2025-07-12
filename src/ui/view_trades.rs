use crate::app::App;
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::*,
};

pub fn draw_view_trades(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default()
        .title("View Trades [Up/Down: scroll, ESC: return]")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let header = Row::new(vec![
        Cell::from("Symbol"),
        Cell::from("Campaign"),
        Cell::from("Action"),
        Cell::from("Strike"),
        Cell::from("Delta"),
        Cell::from("Exp."),
        Cell::from("Date"),
        Cell::from("Shares"),
        Cell::from("Credit"),
        Cell::from("Total Credit"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    let mut rows: Vec<Row> = vec![header];
    // Filter and sort trades for current campaign
    let mut campaign_trades: Vec<&crate::models::OptionTrade> = app
        .trades
        .iter()
        .filter(|t| {
            t.campaign == app.selected_campaign.as_ref().unwrap().name
                && t.symbol == app.selected_campaign.as_ref().unwrap().symbol
        })
        .collect();

    // Sort by expiration date (earliest first)
    campaign_trades.sort_by(|a, b| a.expiration_date.cmp(&b.expiration_date));

    rows.extend(
        campaign_trades
            .iter()
            .skip(app.table_scroll)
            .take((size.height as usize).saturating_sub(3))
            .map(|t| {
                let pl = t.number_of_shares as f64 * t.credit;
                let pl_color = match t.action {
                    crate::models::Action::BuyPut => Color::Red,
                    _ => {
                        if pl >= 0.0 {
                            Color::Green
                        } else {
                            Color::Red
                        }
                    }
                };
                Row::new(vec![
                    Cell::from(t.symbol.clone()),
                    Cell::from(t.campaign.clone()),
                    Cell::from(format!("{:?}", t.action)),
                    Cell::from(t.strike.to_string()),
                    Cell::from(t.delta.to_string()),
                    Cell::from(t.expiration_date.to_string()),
                    Cell::from(t.date_of_action.to_string()),
                    Cell::from(t.number_of_shares.to_string()),
                    Cell::from(t.credit.to_string()),
                    Cell::from(format!("{pl:.2}")).style(Style::default().fg(pl_color)),
                ])
            }),
    );
    let widths = [
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Length(12),
    ];
    let table = Table::new(rows, widths).block(block);
    f.render_widget(table, size);
}
