use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::ListState;
use ratatui::{prelude::*, widgets::*};
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::io::{self, Stdout};
use time::macros::format_description;
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Action {
    BuyPut,
    SellPut,
    BuyCall,
    SellCall,
    Exercised,
    Assigned,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OptionTrade {
    id: Option<i32>,
    symbol: String,
    campaign: String,
    action: Action,
    strike: f64,
    delta: f64,
    expiration_date: Date,
    date_of_action: Date,
    number_of_shares: i32,
    credit: f64,
}

impl OptionTrade {
    pub fn insert(&self, conn: &Connection) -> Result<usize> {
        conn.execute(
            "INSERT INTO option_trades (symbol, campaign, action, strike, delta, expiration_date, date_of_action, number_of_shares, credit)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                self.symbol,
                self.campaign,
                format!("{:?}", self.action),
                self.strike,
                self.delta,
                self.expiration_date.to_string(),
                self.date_of_action.to_string(),
                self.number_of_shares,
                self.credit,
            ],
        )
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<OptionTrade>> {
        let date_fmt = format_description!("[year]-[month]-[day]");
        let mut stmt = conn.prepare(
            "SELECT id, symbol, campaign, action, strike, delta, expiration_date, date_of_action, number_of_shares, credit FROM option_trades"
        )?;
        let trade_iter = stmt.query_map([], |row| {
            Ok(OptionTrade {
                id: row.get(0)?,
                symbol: row.get(1)?,
                campaign: row.get(2)?,
                action: match row.get::<_, String>(3)?.as_str() {
                    "BuyPut" => Action::BuyPut,
                    "SellPut" => Action::SellPut,
                    "BuyCall" => Action::BuyCall,
                    "SellCall" => Action::SellCall,
                    "Exercised" => Action::Exercised,
                    "Assigned" => Action::Assigned,
                    _ => Action::SellPut, // fallback
                },
                strike: row.get(4)?,
                delta: row.get(5)?,
                expiration_date: {
                    let s: String = row.get(6)?;
                    Date::parse(&s, &date_fmt).unwrap()
                },
                date_of_action: {
                    let s: String = row.get(7)?;
                    Date::parse(&s, &date_fmt).unwrap()
                },
                number_of_shares: row.get(8)?,
                credit: row.get(9)?,
            })
        })?;
        Ok(trade_iter.filter_map(Result::ok).collect())
    }

    pub fn update(&self, conn: &Connection) -> Result<usize> {
        conn.execute(
            "UPDATE option_trades SET symbol = ?1, campaign = ?2, action = ?3, strike = ?4, delta = ?5, expiration_date = ?6, date_of_action = ?7, number_of_shares = ?8, credit = ?9 WHERE id = ?10",
            params![
                self.symbol,
                self.campaign,
                format!("{:?}", self.action),
                self.strike,
                self.delta,
                self.expiration_date.to_string(),
                self.date_of_action.to_string(),
                self.number_of_shares,
                self.credit,
                self.id,
            ],
        )
    }
}

#[derive(Debug, Clone)]
struct Campaign {
    #[allow(unused)]
    id: i32,
    name: String,
    symbol: String,
    #[allow(unused)]
    created_at: String,
    target_exit_price: Option<f64>,
}

impl Campaign {
    fn get_all(conn: &Connection) -> Vec<Campaign> {
        let mut stmt = conn.prepare("SELECT id, name, symbol, created_at, target_exit_price FROM campaigns ORDER BY created_at DESC").unwrap();
        let iter = stmt
            .query_map([], |row| {
                Ok(Campaign {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    symbol: row.get(2)?,
                    created_at: row.get(3)?,
                    target_exit_price: row.get(4)?,
                })
            })
            .unwrap();
        iter.filter_map(Result::ok).collect()
    }
    fn insert(
        conn: &Connection,
        name: &str,
        symbol: &str,
        target_exit_price: Option<f64>,
    ) -> Option<Campaign> {
        let now = OffsetDateTime::now_local().unwrap().date().to_string();
        let _ = conn.execute(
            "INSERT INTO campaigns (name, symbol, created_at, target_exit_price) VALUES (?1, ?2, ?3, ?4)",
            params![name, symbol, now, target_exit_price],
        );
        let id = conn.last_insert_rowid();
        Some(Campaign {
            id: id as i32,
            name: name.to_string(),
            symbol: symbol.to_string(),
            created_at: now,
            target_exit_price,
        })
    }
}

enum AppScreen {
    #[allow(unused)]
    MainMenu,
    CampaignSelect,
    NewCampaign,
    CampaignDashboard,
    AddTrade,
    ViewTrades,
    EditTrade,
}

const ACTIONS: [&str; 6] = [
    "BuyPut",
    "SellPut",
    "BuyCall",
    "SellCall",
    "Exercised",
    "Assigned",
];

struct App {
    screen: AppScreen,
    // Campaign state
    campaigns: Vec<Campaign>,
    selected_campaign: Option<Campaign>,
    campaign_select_index: usize,
    campaign_list_state: ListState,
    new_campaign_name: String,
    new_campaign_symbol: String,
    new_campaign_target_price: String,
    new_campaign_field: usize, // 0 = name, 1 = symbol, 2 = target price
    // Add Trade form state
    form_fields: [String; 6], // strike, delta, expiration, date, shares, credit
    form_index: usize,
    action_index: usize,
    form_error: Option<String>,
    // View Trades state
    trades: Vec<OptionTrade>,
    table_scroll: usize,
    db_conn: Connection,
    edit_trade_fields: [String; 8], // symbol, action, strike, delta, expiration, date, shares, credit
    edit_action_index: usize,
    edit_form_index: usize,
    edit_trade_id: Option<i32>,
}

impl App {
    fn new() -> Self {
        let db_conn = Connection::open("options_trades.db").unwrap();
        db_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS campaigns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                symbol TEXT NOT NULL,
                created_at TEXT NOT NULL,
                target_exit_price REAL
            )",
                [],
            )
            .unwrap();
        db_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS option_trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                campaign TEXT NOT NULL,
                action TEXT NOT NULL,
                strike REAL NOT NULL,
                delta REAL NOT NULL,
                expiration_date TEXT NOT NULL,
                date_of_action TEXT NOT NULL,
                number_of_shares INTEGER NOT NULL,
                credit REAL NOT NULL
            )",
                [],
            )
            .unwrap();
        let campaigns = Campaign::get_all(&db_conn);
        let trades = OptionTrade::get_all(&db_conn).unwrap_or_default();
        let mut form_fields: [String; 6] = Default::default();
        // Set Date of Action (index 3) to today
        form_fields[3] = OffsetDateTime::now_utc().date().to_string();
        let mut campaign_list_state = ListState::default();
        campaign_list_state.select(Some(0));
        Self {
            screen: AppScreen::CampaignSelect,
            campaigns,
            selected_campaign: None,
            campaign_select_index: 0,
            campaign_list_state,
            new_campaign_name: String::new(),
            new_campaign_symbol: String::new(),
            new_campaign_target_price: String::new(),
            new_campaign_field: 0,
            form_fields,
            form_index: 0,
            action_index: 0,
            form_error: None,
            trades,
            table_scroll: 0,
            db_conn,
            edit_trade_fields: Default::default(),
            edit_action_index: 0,
            edit_form_index: 0,
            edit_trade_id: None,
        }
    }
    fn reset_form(&mut self) {
        self.form_fields = Default::default();
        self.form_index = 0;
        self.action_index = 0;
        self.form_error = None;
        // Set Date of Action (index 3) to today
        self.form_fields[3] = OffsetDateTime::now_utc().date().to_string();
    }
    fn reload_trades(&mut self) {
        self.trades = OptionTrade::get_all(&self.db_conn).unwrap_or_default();
    }
    fn reload_campaigns(&mut self) {
        self.campaigns = Campaign::get_all(&self.db_conn);
        if self.campaign_select_index >= self.campaigns.len() {
            self.campaign_select_index = self.campaigns.len().saturating_sub(1);
        }
        self.campaign_list_state
            .select(Some(self.campaign_select_index));
    }
    fn set_edit_trade(&mut self, trade: &OptionTrade) {
        self.edit_trade_id = trade.id;
        self.edit_trade_fields = [
            trade.symbol.clone(),
            format!("{:?}", trade.action),
            trade.strike.to_string(),
            trade.delta.to_string(),
            trade.expiration_date.to_string(),
            trade.date_of_action.to_string(),
            trade.number_of_shares.to_string(),
            trade.credit.to_string(),
        ];
        self.edit_action_index = match trade.action {
            Action::BuyPut => 0,
            Action::SellPut => 1,
            Action::BuyCall => 2,
            Action::SellCall => 3,
            Action::Exercised => 4,
            Action::Assigned => 5,
        };
        self.edit_form_index = 0;
    }
}

fn calculate_campaign_summary(
    trades: &[&OptionTrade],
    target_exit_price: Option<f64>,
) -> (Option<f64>, i32, Option<f64>, f64, f64) {
    // Break-even calculation
    let total_debits: f64 = trades
        .iter()
        .filter(|t| {
            matches!(
                t.action,
                Action::Assigned | Action::BuyCall | Action::BuyPut
            )
        })
        .map(|t| t.credit * t.number_of_shares as f64)
        .sum();

    let total_credits: f64 = trades
        .iter()
        .filter(|t| matches!(t.action, Action::SellPut | Action::SellCall))
        .map(|t| t.credit * t.number_of_shares as f64)
        .sum();

    let total_shares_assigned: i32 = trades
        .iter()
        .filter(|t| matches!(t.action, Action::Assigned))
        .map(|t| t.number_of_shares)
        .sum();

    let break_even = if total_shares_assigned > 0 {
        Some((total_debits - total_credits) / total_shares_assigned as f64)
    } else {
        None
    };

    // Weeks running calculation
    let first_trade_date = trades.iter().map(|t| t.date_of_action).min();

    let weeks_running = if let Some(first_date) = first_trade_date {
        let today = OffsetDateTime::now_local().unwrap().date();
        let days_diff = (today - first_date).whole_days();
        (days_diff / 7) as i32
    } else {
        0
    };

    // Profit per week calculation
    let profit_per_week = if let Some(target_price) = target_exit_price {
        if total_shares_assigned > 0 && weeks_running > 0 {
            let target_profit =
                (target_price - break_even.unwrap_or(0.0)) * total_shares_assigned as f64;
            Some(target_profit / weeks_running as f64)
        } else {
            None
        }
    } else {
        None
    };

    let running_profit_loss = total_credits - total_debits;
    (
        break_even,
        weeks_running,
        profit_per_week,
        total_credits,
        running_profit_loss,
    )
}

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
    terminal.show_cursor().unwrap();

    if let Err(err) = res {
        println!("Error: {err:?}");
    }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| match app.screen {
            AppScreen::CampaignSelect => draw_campaign_select(f, app),
            AppScreen::NewCampaign => draw_new_campaign(f, app),
            AppScreen::CampaignDashboard => draw_campaign_dashboard(f, app),
            AppScreen::MainMenu => draw_main_menu(f),
            AppScreen::AddTrade => draw_add_trade(f, app),
            AppScreen::ViewTrades => draw_view_trades(f, app),
            AppScreen::EditTrade => draw_edit_trade(f, app),
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.screen {
                    AppScreen::CampaignSelect => match key.code {
                        KeyCode::Char('n') => {
                            app.new_campaign_name.clear();
                            app.new_campaign_symbol.clear();
                            app.new_campaign_target_price.clear();
                            app.screen = AppScreen::NewCampaign;
                        }
                        KeyCode::Down => {
                            if app.campaign_select_index + 1 < app.campaigns.len() {
                                app.campaign_select_index += 1;
                                app.campaign_list_state
                                    .select(Some(app.campaign_select_index));
                            }
                        }
                        KeyCode::Up => {
                            if app.campaign_select_index > 0 {
                                app.campaign_select_index -= 1;
                                app.campaign_list_state
                                    .select(Some(app.campaign_select_index));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(camp) =
                                app.campaigns.get(app.campaign_select_index).cloned()
                            {
                                app.selected_campaign = Some(camp);
                                app.screen = AppScreen::CampaignDashboard;
                            }
                        }
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    },
                    AppScreen::NewCampaign => match key.code {
                        KeyCode::Esc => app.screen = AppScreen::CampaignSelect,
                        KeyCode::Tab => {
                            app.new_campaign_field = (app.new_campaign_field + 1) % 3;
                        }
                        KeyCode::BackTab => {
                            app.new_campaign_field = (app.new_campaign_field + 2) % 3;
                        }
                        KeyCode::Enter => {
                            if !app.new_campaign_name.trim().is_empty()
                                && !app.new_campaign_symbol.trim().is_empty()
                            {
                                let target_price =
                                    app.new_campaign_target_price.trim().parse().ok();
                                if let Some(camp) = Campaign::insert(
                                    &app.db_conn,
                                    &app.new_campaign_name,
                                    &app.new_campaign_symbol,
                                    target_price,
                                ) {
                                    app.reload_campaigns();
                                    app.selected_campaign = Some(camp);
                                    app.screen = AppScreen::CampaignDashboard;
                                }
                            }
                        }
                        KeyCode::Char(c) => match app.new_campaign_field {
                            0 => app.new_campaign_name.push(c),
                            1 => app.new_campaign_symbol.push(c),
                            2 => app.new_campaign_target_price.push(c),
                            _ => {}
                        },
                        KeyCode::Backspace => match app.new_campaign_field {
                            0 => {
                                app.new_campaign_name.pop();
                            }
                            1 => {
                                app.new_campaign_symbol.pop();
                            }
                            2 => {
                                app.new_campaign_target_price.pop();
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    AppScreen::CampaignDashboard => match key.code {
                        KeyCode::Char('a') => app.screen = AppScreen::AddTrade,
                        KeyCode::Char('v') => app.screen = AppScreen::ViewTrades,
                        KeyCode::Esc => {
                            app.selected_campaign = None;
                            app.screen = AppScreen::CampaignSelect;
                        }
                        _ => {}
                    },
                    AppScreen::MainMenu => match key.code {
                        KeyCode::Char('1') => {
                            app.reset_form();
                            app.screen = AppScreen::AddTrade;
                        }
                        KeyCode::Char('2') => {
                            app.reload_trades();
                            app.screen = AppScreen::ViewTrades;
                        }
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    },
                    AppScreen::AddTrade => match key.code {
                        KeyCode::Esc => app.screen = AppScreen::CampaignDashboard,
                        KeyCode::Tab => {
                            app.form_index = (app.form_index + 1) % (app.form_fields.len() + 1)
                        }
                        KeyCode::BackTab => {
                            app.form_index = (app.form_index + app.form_fields.len())
                                % (app.form_fields.len() + 1)
                        }
                        KeyCode::Left | KeyCode::Char('h') if app.form_index == 0 => {
                            if app.action_index == 0 {
                                app.action_index = ACTIONS.len() - 1;
                            } else {
                                app.action_index -= 1;
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') if app.form_index == 0 => {
                            app.action_index = (app.action_index + 1) % ACTIONS.len();
                        }
                        KeyCode::Enter => {
                            if let Some(trade) = try_build_trade(
                                &app.form_fields,
                                app.action_index,
                                app.selected_campaign.as_ref().unwrap(),
                            ) {
                                let _ = trade.insert(&app.db_conn);
                                app.reload_trades();
                                app.screen = AppScreen::CampaignDashboard;
                                app.form_error = None;
                            } else {
                                app.form_error =
                                    Some("Invalid input. Please check all fields.".to_string());
                            }
                        }
                        KeyCode::Char(c) if app.form_index != 1 => {
                            app.form_fields[if app.form_index < 1 {
                                app.form_index
                            } else {
                                app.form_index - 1
                            }]
                            .push(c);
                        }
                        KeyCode::Backspace if app.form_index != 1 => {
                            app.form_fields[if app.form_index < 1 {
                                app.form_index
                            } else {
                                app.form_index - 1
                            }]
                            .pop();
                        }
                        _ => {}
                    },
                    AppScreen::ViewTrades => match key.code {
                        KeyCode::Esc => app.screen = AppScreen::CampaignDashboard,
                        KeyCode::Down => {
                            if app.table_scroll + 1 < app.trades.len() {
                                app.table_scroll += 1;
                            }
                        }
                        KeyCode::Up => {
                            if app.table_scroll > 0 {
                                app.table_scroll -= 1;
                            }
                        }
                        KeyCode::Char('e') => {
                            let trade = app.trades.get(app.table_scroll).cloned();
                            if let Some(trade) = trade {
                                app.set_edit_trade(&trade);
                                app.screen = AppScreen::EditTrade;
                            }
                        }
                        _ => {}
                    },
                    AppScreen::EditTrade => match key.code {
                        KeyCode::Esc => app.screen = AppScreen::ViewTrades,
                        KeyCode::Tab => app.edit_form_index = (app.edit_form_index + 1) % 8,
                        KeyCode::BackTab => app.edit_form_index = (app.edit_form_index + 7) % 8,
                        KeyCode::Left | KeyCode::Char('h') if app.edit_form_index == 1 => {
                            if app.edit_action_index == 0 {
                                app.edit_action_index = ACTIONS.len() - 1;
                            } else {
                                app.edit_action_index -= 1;
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') if app.edit_form_index == 1 => {
                            app.edit_action_index = (app.edit_action_index + 1) % ACTIONS.len();
                        }
                        KeyCode::Char(c) if app.edit_form_index != 1 => {
                            app.edit_trade_fields[if app.edit_form_index < 1 {
                                app.edit_form_index
                            } else {
                                app.edit_form_index - 1
                            }]
                            .push(c);
                        }
                        KeyCode::Backspace if app.edit_form_index != 1 => {
                            app.edit_trade_fields[if app.edit_form_index < 1 {
                                app.edit_form_index
                            } else {
                                app.edit_form_index - 1
                            }]
                            .pop();
                        }
                        KeyCode::Enter => {
                            if let Some(id) = app.edit_trade_id {
                                if let Some(trade) = try_build_trade_edit(
                                    &app.edit_trade_fields,
                                    app.edit_action_index,
                                    app.selected_campaign.as_ref().unwrap().name.as_str(),
                                    id,
                                ) {
                                    let _ = trade.update(&app.db_conn);
                                    app.reload_trades();
                                    app.screen = AppScreen::ViewTrades;
                                }
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

fn draw_main_menu(f: &mut Frame) {
    let size = f.area();
    let block = Block::default()
        .title("Options Tracker")
        .borders(Borders::ALL);
    let items = vec![
        ListItem::new("1. Add Trade"),
        ListItem::new("2. View Trades"),
        ListItem::new("q. Quit"),
    ];
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_widget(list, size);
}

fn draw_campaign_select(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default()
        .title("Select Campaign [n: new, ↑/↓: move, Enter: select, q: quit]")
        .borders(Borders::ALL);
    let items: Vec<ListItem> = app
        .campaigns
        .iter()
        .map(|c| ListItem::new(c.name.clone()))
        .collect();
    let list = List::new(items).block(block).highlight_symbol("> ");
    f.render_stateful_widget(list, size, &mut app.campaign_list_state.clone());
}

fn draw_new_campaign(f: &mut Frame, app: &App) {
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

fn draw_campaign_dashboard(f: &mut Frame, app: &App) {
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
    let campaign_trades: Vec<&OptionTrade> = app
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
    ];
    let para = Paragraph::new(summary_lines)
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(para, size);
}

fn draw_add_trade(f: &mut Frame, app: &App) {
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

fn draw_view_trades(f: &mut Frame, app: &App) {
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
    rows.extend(
        app.trades
            .iter()
            .filter(|t| {
                t.campaign == app.selected_campaign.as_ref().unwrap().name
                    && t.symbol == app.selected_campaign.as_ref().unwrap().symbol
            })
            .skip(app.table_scroll)
            .take((size.height as usize).saturating_sub(3))
            .map(|t| {
                let pl = t.number_of_shares as f64 * t.credit;
                let pl_color = if pl >= 0.0 { Color::Green } else { Color::Red };
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

fn draw_edit_trade(f: &mut Frame, app: &mut App) {
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

fn try_build_trade(
    fields: &[String; 6],
    action_index: usize,
    campaign: &Campaign,
) -> Option<OptionTrade> {
    use Action::*;
    let action = match ACTIONS[action_index] {
        "BuyPut" => BuyPut,
        "SellPut" => SellPut,
        "BuyCall" => BuyCall,
        "SellCall" => SellCall,
        "Exercised" => Exercised,
        "Assigned" => Assigned,
        _ => return None,
    };
    let strike = fields[0].parse().ok()?;
    let delta = fields[1].parse().ok()?;
    let date_fmt = format_description!("[year]-[month]-[day]");
    let expiration_date = Date::parse(&fields[2], &date_fmt).ok()?;
    let date_of_action = Date::parse(&fields[3], &date_fmt).ok()?;
    let number_of_shares = fields[4].parse().ok()?;
    let credit = fields[5].parse().ok()?;
    Some(OptionTrade {
        id: None,
        symbol: campaign.symbol.clone(),
        campaign: campaign.name.clone(),
        action,
        strike,
        delta,
        expiration_date,
        date_of_action,
        number_of_shares,
        credit,
    })
}

fn try_build_trade_edit(
    fields: &[String; 8],
    action_index: usize,
    campaign: &str,
    id: i32,
) -> Option<OptionTrade> {
    use Action::*;
    let action = match ACTIONS[action_index] {
        "BuyPut" => BuyPut,
        "SellPut" => SellPut,
        "BuyCall" => BuyCall,
        "SellCall" => SellCall,
        "Exercised" => Exercised,
        "Assigned" => Assigned,
        _ => return None,
    };
    let strike = fields[2].parse().ok()?;
    let delta = fields[3].parse().ok()?;
    let date_fmt = format_description!("[year]-[month]-[day]");
    let expiration_date = Date::parse(&fields[4], &date_fmt).ok()?;
    let date_of_action = Date::parse(&fields[5], &date_fmt).ok()?;
    let number_of_shares = fields[6].parse().ok()?;
    let credit = fields[7].parse().ok()?;
    Some(OptionTrade {
        id: Some(id),
        symbol: fields[0].clone(),
        campaign: campaign.to_string(),
        action,
        strike,
        delta,
        expiration_date,
        date_of_action,
        number_of_shares,
        credit,
    })
}
