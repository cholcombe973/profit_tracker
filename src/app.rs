use crate::db;
use crate::models::{Action, Campaign, OptionTrade};
use ratatui::widgets::ListState;
use rusqlite::Connection;
use time::{Duration, OffsetDateTime};

pub enum AppScreen {
    Summary, // Added summary screen
    #[allow(unused)]
    MainMenu,
    CampaignSelect,
    NewCampaign,
    CampaignDashboard,
    AddTrade,
    ViewTrades,
    EditTrade,
}

pub const ACTIONS: [&str; 6] = [
    "BuyPut",
    "SellPut",
    "BuyCall",
    "SellCall",
    "Exercised",
    "Assigned",
];

pub struct App {
    pub screen: AppScreen,
    pub campaigns: Vec<Campaign>,
    pub selected_campaign: Option<Campaign>,
    pub campaign_select_index: usize,
    pub campaign_list_state: ListState,
    pub new_campaign_name: String,
    pub new_campaign_symbol: String,
    pub new_campaign_target_price: String,
    pub new_campaign_field: usize, // 0 = name, 1 = symbol, 2 = target price
    pub form_fields: [String; 6],  // strike, delta, expiration, date, shares, credit
    pub form_index: usize,
    pub action_index: usize,
    pub form_error: Option<String>,
    pub trades: Vec<OptionTrade>,
    pub table_scroll: usize,
    pub db_conn: Connection,
    pub edit_trade_fields: [String; 8], // symbol, action, strike, delta, expiration, date, shares, credit
    pub edit_action_index: usize,
    pub edit_form_index: usize,
    pub edit_trade_id: Option<i32>,
}

impl App {
    pub fn new() -> Self {
        let db_conn = Connection::open("options_trades.db").unwrap();
        db::init_database(&db_conn).unwrap();
        let mut campaigns = Campaign::get_all(&db_conn);
        campaigns.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let trades = OptionTrade::get_all(&db_conn).unwrap_or_default();
        let mut form_fields: [String; 6] = Default::default();
        // Set Date of Action (index 3) to today
        form_fields[3] = OffsetDateTime::now_local().unwrap().date().to_string();
        let mut campaign_list_state = ListState::default();
        campaign_list_state.select(Some(0));
        Self {
            screen: AppScreen::Summary, // Set summary as default
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
    pub fn reset_form(&mut self) {
        self.form_fields = Default::default();
        self.form_index = 0;
        self.action_index = 0;
        self.form_error = None;
        // Set Date of Action (index 3) to today
        self.form_fields[3] = OffsetDateTime::now_local().unwrap().date().to_string();
    }
    pub fn reload_trades(&mut self) {
        let mut trades = OptionTrade::get_all(&self.db_conn).unwrap_or_default();
        // Sort trades by expiration date (earliest first), then by date of action
        trades.sort_by(|a, b| a.expiration_date.cmp(&b.expiration_date));
        self.trades = trades;
    }
    pub fn reload_campaigns(&mut self) {
        self.campaigns = Campaign::get_all(&self.db_conn);
        self.campaigns
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        if self.campaign_select_index >= self.campaigns.len() {
            self.campaign_select_index = self.campaigns.len().saturating_sub(1);
        }
        self.campaign_list_state
            .select(Some(self.campaign_select_index));
    }
    pub fn set_edit_trade(&mut self, trade: &OptionTrade) {
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

    pub fn total_pnl(&self) -> f64 {
        use crate::logic::calculate_total_premium_sold;
        calculate_total_premium_sold(&self.trades)
    }

    pub fn trades_in_progress_this_week(&self) -> Vec<&crate::models::OptionTrade> {
        let today = OffsetDateTime::now_local().unwrap().date();
        let start_of_week = today - Duration::days(today.weekday().number_from_monday() as i64 - 1);
        let end_of_week = start_of_week + Duration::days(6);
        self.trades
            .iter()
            .filter(|t| t.expiration_date >= start_of_week && t.expiration_date <= end_of_week)
            .collect()
    }

    pub fn free_cash(&self) -> f64 {
        // Net premium received (credits - debits)
        let credits: f64 = self
            .trades
            .iter()
            .filter(|t| {
                matches!(
                    t.action,
                    crate::models::Action::SellPut | crate::models::Action::SellCall
                )
            })
            .map(|t| t.credit * t.number_of_shares as f64)
            .sum();
        let debits: f64 = self
            .trades
            .iter()
            .filter(|t| {
                matches!(
                    t.action,
                    crate::models::Action::BuyPut
                        | crate::models::Action::BuyCall
                        | crate::models::Action::Assigned
                )
            })
            .map(|t| t.credit * t.number_of_shares as f64)
            .sum();
        credits - debits
    }

    pub fn roic(&self) -> Option<f64> {
        // Return on Invested Capital = total P&L / total capital at risk
        // capital at risk as sum of (strike * shares) for open short puts/calls
        let capital_at_risk: f64 = self
            .trades
            .iter()
            .filter(|t| {
                matches!(
                    t.action,
                    crate::models::Action::SellPut | crate::models::Action::SellCall
                )
            })
            .map(|t| t.strike * t.number_of_shares as f64)
            .sum();
        if capital_at_risk > 0.0 {
            Some(self.total_pnl() / capital_at_risk)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn recent_trades(&self, n: usize) -> Vec<&crate::models::OptionTrade> {
        let mut trades: Vec<&crate::models::OptionTrade> = self.trades.iter().collect();
        trades.sort_by(|a, b| b.date_of_action.cmp(&a.date_of_action));
        trades.into_iter().take(n).collect()
    }
}
