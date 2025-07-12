use crate::app::App;
use crate::logic::{calculate_campaign_summary, calculate_weekly_premium};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::*,
};

pub fn draw_campaign_dashboard(f: &mut Frame, app: &App) {
    let size = f.area();
    let title = if let Some(camp) = &app.selected_campaign {
        format!(
            "Campaign: {} [a: add trade, v: view trades, ESC: back]",
            camp.name
        )
    } else {
        "Campaign Dashboard".to_string()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    // Calculate campaign summary
    let campaign_trades: Vec<&crate::models::OptionTrade> = app
        .trades
        .iter()
        .filter(|t| {
            t.campaign == app.selected_campaign.as_ref().unwrap().name
                && t.symbol == app.selected_campaign.as_ref().unwrap().symbol
        })
        .collect();

    let (break_even, weeks_running, profit_per_week, total_credits, running_profit_loss) =
        calculate_campaign_summary(
            &campaign_trades,
            app.selected_campaign.as_ref().unwrap().target_exit_price,
        );

    // Calculate weekly premium for this campaign
    let campaign_trades_vec: Vec<crate::models::OptionTrade> = app
        .trades
        .iter()
        .filter(|t| {
            t.campaign == app.selected_campaign.as_ref().unwrap().name
                && t.symbol == app.selected_campaign.as_ref().unwrap().symbol
        })
        .cloned()
        .collect();

    let weekly_premium = calculate_weekly_premium(&campaign_trades_vec);

    let pl_color = if running_profit_loss >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };
    let summary_lines = vec![
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            "Campaign Summary:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw(format!(
            "Target Exit Price: {}",
            app.selected_campaign
                .as_ref()
                .unwrap()
                .target_exit_price
                .map(|p| format!("${p:.2}"))
                .unwrap_or_else(|| "N/A".to_string())
        ))]),
        Line::from(vec![Span::raw(format!(
            "Total Credits: ${total_credits:.2}"
        ))]),
        Line::from(vec![
            Span::raw("Running P/L: "),
            Span::styled(
                format!("${running_profit_loss:.2}"),
                Style::default().fg(pl_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::raw(format!(
            "Break Even: {}",
            break_even
                .map(|be| format!("${be:.2}"))
                .unwrap_or_else(|| "N/A".to_string())
        ))]),
        Line::from(vec![Span::raw(format!("Weeks Running: {weeks_running}"))]),
        Line::from(vec![Span::raw(format!(
            "Profit per Week: {}",
            profit_per_week
                .map(|ppw| format!("${ppw:.2}"))
                .unwrap_or_else(|| "N/A".to_string())
        ))]),
        Line::from(vec![Span::styled(
            format!("This Week's Premium: ${weekly_premium:.2}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
    ];
    let para = Paragraph::new(summary_lines)
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(para, size);
}
