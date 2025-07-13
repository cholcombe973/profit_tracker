use crate::db;
use crate::models::{Action, Campaign, OptionTrade};
use ratatui::widgets::ListState;
use rusqlite::Connection;
use time::OffsetDateTime;

pub enum AppScreen {
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
        let campaigns = Campaign::get_all(&db_conn);
        let trades = OptionTrade::get_all(&db_conn).unwrap_or_default();
        let mut form_fields: [String; 6] = Default::default();
        // Set Date of Action (index 3) to today
        form_fields[3] = OffsetDateTime::now_local().unwrap().date().to_string();
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
}
