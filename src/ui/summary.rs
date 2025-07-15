use ratatui::prelude::*;
use crate::app::App;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Style, Color, Modifier};

pub fn draw_summary(f: &mut Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .title("Summary Dashboard")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    // Gather metrics
    let total_pnl = app.total_pnl();
    let trades_in_progress = app.trades_in_progress_this_week();
    // TODO: Add free cash calculation
    let _free_cash = app.free_cash();
    let roic = app.roic();

    let pnl_color = if total_pnl >= 0.0 { Color::Green } else { Color::Red };
    let roic_str = roic.map(|r| format!("{:.2}%", r * 100.0)).unwrap_or_else(|| "N/A".to_string());

    let weekly_premium = crate::logic::calculate_weekly_premium(&app.trades);

    let mut lines = vec![
        Line::from(vec![Span::styled("Total P&L: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("${:.2}", total_pnl), Style::default().fg(pnl_color))]),
        Line::from(vec![Span::styled("ROIC: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(roic_str)]),
        Line::from(vec![Span::styled("Trades in Progress This Week: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(format!("{}", trades_in_progress.len()))]),
        Line::from(vec![Span::styled("Premium Expiring This Week: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(format!("${:.2}", weekly_premium), Style::default().fg(Color::Yellow))]),
        Line::from(vec![Span::styled("Trades in Progress:", Style::default().add_modifier(Modifier::BOLD))]),
    ];

    for trade in trades_in_progress {
        lines.push(Line::from(vec![
            Span::raw(format!("{} {} {} {} @ ${:.2} exp {} shares {} credit ${:.2}",
                trade.date_of_action,
                trade.symbol,
                format!("{:?}", trade.action),
                trade.strike,
                trade.credit,
                trade.expiration_date,
                trade.number_of_shares,
                trade.credit * trade.number_of_shares as f64
            ))
        ]));
    }

    lines.push(Line::from(vec![Span::raw("")]));
    lines.push(Line::from(vec![Span::styled("Hotkeys:", Style::default().add_modifier(Modifier::BOLD))]));
    lines.push(Line::from(vec![Span::raw("c: Campaigns   n: New Campaign   q: Quit")]));
    lines.push(Line::from(vec![Span::styled("Press a hotkey to navigate.", Style::default().fg(Color::DarkGray))]));

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(para, area);
} 