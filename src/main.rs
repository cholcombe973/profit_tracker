mod app;
mod csv_processor;
mod logic;
mod models;
mod ui;

use app::{App, AppScreen};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use csv_processor::{Broker, CsvProcessor};
use models::{Campaign, OptionTrade};
use ratatui::prelude::*;
use std::io::{self, Stdout};
use time::Date;

#[derive(Parser)]
#[command(name = "profit_tracker")]
#[command(about = "A terminal-based options trading campaign tracker")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Import trades from a CSV file
    Import {
        /// The broker format (etrade or robinhood)
        broker: String,

        /// Path to the CSV file
        #[arg(short, long)]
        file: String,

        /// Campaign name for the imported trades
        #[arg(short, long)]
        campaign: String,

        /// Symbol for the imported trades
        #[arg(short, long)]
        symbol: String,
    },
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Import {
            broker,
            file,
            campaign,
            symbol,
        }) => {
            // Handle CSV import
            import_csv(&broker, &file, &campaign, &symbol)?;
        }
        None => {
            // Run the normal TUI application
            run_tui()?;
        }
    }

    Ok(())
}

fn import_csv(
    broker_str: &str,
    file_path: &str,
    campaign_name: &str,
    symbol: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse broker
    let broker: Broker = broker_str.parse()?;

    // Create CSV processor
    let processor = CsvProcessor::new(broker);

    // Process CSV file
    let trades = processor.process_csv(file_path)?;

    if trades.is_empty() {
        println!("No valid trades found in CSV file");
        return Ok(());
    }

    // Create database connection
    let db_conn = rusqlite::Connection::open("options_trades.db")?;

    // Create tables if they don't exist
    db_conn.execute(
        "CREATE TABLE IF NOT EXISTS campaigns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            symbol TEXT NOT NULL,
            created_at TEXT NOT NULL,
            target_exit_price REAL
        )",
        [],
    )?;

    db_conn.execute(
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
    )?;

    // Create campaign if it doesn't exist
    let _campaign = Campaign::insert(&db_conn, campaign_name, symbol, None);

    // Import trades
    let mut imported_count = 0;
    for mut trade in trades {
        // Override campaign and symbol from CLI arguments
        trade.campaign = campaign_name.to_string();
        trade.symbol = symbol.to_string();

        if trade.insert(&db_conn).is_ok() {
            imported_count += 1;
        }
    }

    println!(
        "Successfully imported {imported_count} trades from {file_path} for campaign '{campaign_name}' ({symbol})"
    );

    Ok(())
}

fn run_tui() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
    }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| match app.screen {
            AppScreen::CampaignSelect => ui::campaign_select::draw_campaign_select(f, app),
            AppScreen::NewCampaign => ui::new_campaign::draw_new_campaign(f, app),
            AppScreen::CampaignDashboard => ui::campaign_dashboard::draw_campaign_dashboard(f, app),
            AppScreen::MainMenu => draw_main_menu(f),
            AppScreen::AddTrade => ui::add_trade::draw_add_trade(f, app),
            AppScreen::ViewTrades => ui::view_trades::draw_view_trades(f, app),
            AppScreen::EditTrade => ui::edit_trade::draw_edit_trade(f, app),
        })?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match app.screen {
                AppScreen::CampaignSelect => match key.code {
                    crossterm::event::KeyCode::Down => {
                        if app.campaign_select_index + 1 < app.campaigns.len() {
                            app.campaign_select_index += 1;
                            app.campaign_list_state
                                .select(Some(app.campaign_select_index));
                        }
                    }
                    crossterm::event::KeyCode::Up => {
                        if app.campaign_select_index > 0 {
                            app.campaign_select_index -= 1;
                            app.campaign_list_state
                                .select(Some(app.campaign_select_index));
                        }
                    }
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    crossterm::event::KeyCode::Char('n') => {
                        app.screen = AppScreen::NewCampaign;
                    }
                    crossterm::event::KeyCode::Enter => {
                        if let Some(camp) = app.campaigns.get(app.campaign_select_index).cloned() {
                            app.selected_campaign = Some(camp);
                            app.screen = AppScreen::CampaignDashboard;
                        }
                    }
                    _ => {}
                },
                AppScreen::CampaignDashboard => match key.code {
                    crossterm::event::KeyCode::Esc => {
                        app.selected_campaign = None;
                        app.screen = AppScreen::CampaignSelect;
                    }
                    crossterm::event::KeyCode::Char('a') => {
                        app.screen = AppScreen::AddTrade;
                    }
                    crossterm::event::KeyCode::Char('v') => {
                        app.screen = AppScreen::ViewTrades;
                    }
                    _ => {}
                },
                AppScreen::ViewTrades => match key.code {
                    crossterm::event::KeyCode::Esc => {
                        app.screen = AppScreen::CampaignDashboard;
                    }
                    crossterm::event::KeyCode::Down => {
                        if app.table_scroll + 1 < app.trades.len() {
                            app.table_scroll += 1;
                        }
                    }
                    crossterm::event::KeyCode::Up => {
                        if app.table_scroll > 0 {
                            app.table_scroll -= 1;
                        }
                    }
                    crossterm::event::KeyCode::Char('e') => {
                        if let Some(trade) = app.trades.get(app.table_scroll).cloned() {
                            app.set_edit_trade(&trade);
                            app.screen = AppScreen::EditTrade;
                        }
                    }
                    _ => {}
                },
                AppScreen::NewCampaign => match key.code {
                    crossterm::event::KeyCode::Tab => {
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::SHIFT)
                        {
                            app.new_campaign_field = if app.new_campaign_field == 0 {
                                2
                            } else {
                                app.new_campaign_field - 1
                            };
                        } else {
                            app.new_campaign_field = (app.new_campaign_field + 1) % 3;
                        }
                    }
                    crossterm::event::KeyCode::Char(ch) => match app.new_campaign_field {
                        0 => app.new_campaign_name.push(ch),
                        1 => app.new_campaign_symbol.push(ch),
                        2 => app.new_campaign_target_price.push(ch),
                        _ => {}
                    },
                    crossterm::event::KeyCode::Backspace => match app.new_campaign_field {
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
                    crossterm::event::KeyCode::Enter => {
                        if !app.new_campaign_name.is_empty() && !app.new_campaign_symbol.is_empty()
                        {
                            let target_price = app.new_campaign_target_price.parse::<f64>().ok();
                            Campaign::insert(
                                &app.db_conn,
                                &app.new_campaign_name,
                                &app.new_campaign_symbol,
                                target_price,
                            );
                            app.reload_campaigns();
                            app.new_campaign_name.clear();
                            app.new_campaign_symbol.clear();
                            app.new_campaign_target_price.clear();
                            app.new_campaign_field = 0;
                            app.screen = AppScreen::CampaignSelect;
                        }
                    }
                    crossterm::event::KeyCode::Esc => {
                        app.new_campaign_name.clear();
                        app.new_campaign_symbol.clear();
                        app.new_campaign_target_price.clear();
                        app.new_campaign_field = 0;
                        app.screen = AppScreen::CampaignSelect;
                    }
                    _ => {}
                },
                AppScreen::AddTrade => match key.code {
                    crossterm::event::KeyCode::Tab => {
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::SHIFT)
                        {
                            app.form_index = if app.form_index == 0 {
                                6
                            } else {
                                app.form_index - 1
                            };
                        } else {
                            app.form_index = (app.form_index + 1) % 7;
                        }
                    }
                    crossterm::event::KeyCode::Left => {
                        if app.form_index == 0 {
                            // Action field
                            app.action_index = if app.action_index == 0 {
                                5
                            } else {
                                app.action_index - 1
                            };
                        }
                    }
                    crossterm::event::KeyCode::Right => {
                        if app.form_index == 0 {
                            // Action field
                            app.action_index = (app.action_index + 1) % 6;
                        }
                    }
                    crossterm::event::KeyCode::Char(ch) => {
                        if app.form_index > 0 {
                            let idx = app.form_index - 1;
                            if idx < app.form_fields.len() {
                                app.form_fields[idx].push(ch);
                            }
                        }
                    }
                    crossterm::event::KeyCode::Backspace => {
                        if app.form_index > 0 {
                            let idx = app.form_index - 1;
                            if idx < app.form_fields.len() {
                                app.form_fields[idx].pop();
                            }
                        }
                    }
                    crossterm::event::KeyCode::Enter => {
                        if let Some(campaign) = &app.selected_campaign {
                            let action = match app.action_index {
                                0 => crate::models::Action::BuyPut,
                                1 => crate::models::Action::SellPut,
                                2 => crate::models::Action::BuyCall,
                                3 => crate::models::Action::SellCall,
                                4 => crate::models::Action::Exercised,
                                5 => crate::models::Action::Assigned,
                                _ => crate::models::Action::BuyPut,
                            };

                            use time::macros::format_description;
                            let date_fmt = format_description!("[year]-[month]-[day]");
                            let expiration_date = Date::parse(&app.form_fields[2], &date_fmt)
                                .unwrap_or_else(|_| {
                                    time::OffsetDateTime::now_local().unwrap().date()
                                });
                            let date_of_action = Date::parse(&app.form_fields[3], &date_fmt)
                                .unwrap_or_else(|_| {
                                    time::OffsetDateTime::now_local().unwrap().date()
                                });

                            let trade = OptionTrade {
                                id: None,
                                symbol: campaign.symbol.clone(),
                                campaign: campaign.name.clone(),
                                action,
                                strike: app.form_fields[0].parse().unwrap_or(0.0),
                                delta: app.form_fields[1].parse().unwrap_or(0.0),
                                expiration_date,
                                date_of_action,
                                number_of_shares: app.form_fields[4].parse().unwrap_or(0),
                                credit: app.form_fields[5].parse().unwrap_or(0.0),
                            };

                            if trade.insert(&app.db_conn).is_ok() {
                                app.reset_form();
                                app.reload_trades();
                                app.screen = AppScreen::CampaignDashboard;
                            } else {
                                app.form_error = Some("Failed to save trade".to_string());
                            }
                        }
                    }
                    crossterm::event::KeyCode::Esc => {
                        app.reset_form();
                        app.screen = AppScreen::CampaignDashboard;
                    }
                    _ => {}
                },
                AppScreen::EditTrade => match key.code {
                    crossterm::event::KeyCode::Tab => {
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::SHIFT)
                        {
                            app.edit_form_index = if app.edit_form_index == 0 {
                                7
                            } else {
                                app.edit_form_index - 1
                            };
                        } else {
                            app.edit_form_index = (app.edit_form_index + 1) % 8;
                        }
                    }
                    crossterm::event::KeyCode::Left => {
                        if app.edit_form_index == 1 {
                            // Action field
                            app.edit_action_index = if app.edit_action_index == 0 {
                                5
                            } else {
                                app.edit_action_index - 1
                            };
                        }
                    }
                    crossterm::event::KeyCode::Right => {
                        if app.edit_form_index == 1 {
                            // Action field
                            app.edit_action_index = (app.edit_action_index + 1) % 6;
                        }
                    }
                    crossterm::event::KeyCode::Char(ch) => {
                        if app.edit_form_index != 1 {
                            // Not action field
                            app.edit_trade_fields[app.edit_form_index].push(ch);
                        }
                    }
                    crossterm::event::KeyCode::Backspace => {
                        if app.edit_form_index != 1 {
                            // Not action field
                            app.edit_trade_fields[app.edit_form_index].pop();
                        }
                    }
                    crossterm::event::KeyCode::Enter => {
                        if let Some(trade_id) = app.edit_trade_id {
                            let action = match app.edit_action_index {
                                0 => crate::models::Action::BuyPut,
                                1 => crate::models::Action::SellPut,
                                2 => crate::models::Action::BuyCall,
                                3 => crate::models::Action::SellCall,
                                4 => crate::models::Action::Exercised,
                                5 => crate::models::Action::Assigned,
                                _ => crate::models::Action::BuyPut,
                            };

                            use time::macros::format_description;
                            let date_fmt = format_description!("[year]-[month]-[day]");
                            let expiration_date = Date::parse(&app.edit_trade_fields[4], &date_fmt)
                                .unwrap_or_else(|_| {
                                    time::OffsetDateTime::now_local().unwrap().date()
                                });
                            let date_of_action = Date::parse(&app.edit_trade_fields[5], &date_fmt)
                                .unwrap_or_else(|_| {
                                    time::OffsetDateTime::now_local().unwrap().date()
                                });

                            let updated_trade = OptionTrade {
                                id: Some(trade_id),
                                symbol: app.edit_trade_fields[0].clone(),
                                campaign: app.edit_trade_fields[1].clone(),
                                action,
                                strike: app.edit_trade_fields[2].parse().unwrap_or(0.0),
                                delta: app.edit_trade_fields[3].parse().unwrap_or(0.0),
                                expiration_date,
                                date_of_action,
                                number_of_shares: app.edit_trade_fields[6].parse().unwrap_or(0),
                                credit: app.edit_trade_fields[7].parse().unwrap_or(0.0),
                            };

                            if updated_trade.update(&app.db_conn).is_ok() {
                                app.reload_trades();
                                app.edit_trade_id = None;
                                app.screen = AppScreen::ViewTrades;
                            }
                        }
                    }
                    crossterm::event::KeyCode::Esc => {
                        app.edit_trade_id = None;
                        app.screen = AppScreen::ViewTrades;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn draw_main_menu(f: &mut Frame) {
    use ratatui::widgets::*;
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
