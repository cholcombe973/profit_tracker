use crate::models::{Action, OptionTrade};
use csv::Reader;
use std::fs::File;
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Broker {
    ETrade,
    Robinhood,
}

impl Broker {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "etrade" => Some(Broker::ETrade),
            "robinhood" => Some(Broker::Robinhood),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Broker::ETrade => "etrade",
            Broker::Robinhood => "robinhood",
        }
    }

    pub fn supported_brokers() -> Vec<&'static str> {
        vec!["etrade", "robinhood"]
    }
}

impl std::str::FromStr for Broker {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Broker::from_str(s).ok_or_else(|| {
            let supported = Broker::supported_brokers().join(", ");
            format!("Invalid broker: '{s}'. Supported brokers: {supported}")
        })
    }
}

impl std::fmt::Display for Broker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct CsvProcessor {
    broker: Broker,
}

impl CsvProcessor {
    pub fn new(broker: Broker) -> Self {
        Self { broker }
    }

    pub fn process_csv(
        &self,
        file_path: &str,
    ) -> Result<Vec<OptionTrade>, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = Reader::from_reader(file);

        match self.broker {
            Broker::ETrade => self.process_etrade_csv(reader),
            Broker::Robinhood => self.process_robinhood_csv(reader),
        }
    }

    fn process_etrade_csv(
        &self,
        mut reader: Reader<File>,
    ) -> Result<Vec<OptionTrade>, Box<dyn std::error::Error>> {
        let mut trades = Vec::new();

        // ETrade CSV format expected columns:
        // Symbol,Quantity,Price,Date,Action,Strike,Expiration,Delta,Campaign
        for result in reader.records() {
            let record = result?;
            if record.len() < 9 {
                continue; // Skip invalid records
            }

            let symbol = record[0].to_string();
            let quantity: i32 = record[1].parse().unwrap_or(0);
            let price: f64 = record[2].parse().unwrap_or(0.0);
            let date_str = &record[3];
            let action_str = &record[4];
            let strike: f64 = record[5].parse().unwrap_or(0.0);
            let expiration_str = &record[6];
            let delta: f64 = record[7].parse().unwrap_or(0.0);
            let campaign = record[8].to_string();

            // Parse dates
            let date_fmt = time::macros::format_description!("[year]-[month]-[day]");
            let date_of_action = Date::parse(date_str, &date_fmt)
                .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date());
            let expiration_date = Date::parse(expiration_str, &date_fmt)
                .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date());

            // Parse action
            let action = match action_str.to_lowercase().as_str() {
                "buy put" | "buyput" => Action::BuyPut,
                "sell put" | "sellput" => Action::SellPut,
                "buy call" | "buycall" => Action::BuyCall,
                "sell call" | "sellcall" => Action::SellCall,
                "exercised" => Action::Exercised,
                "assigned" => Action::Assigned,
                _ => continue, // Skip unknown actions
            };

            let trade = OptionTrade {
                id: None,
                symbol,
                campaign,
                action,
                strike,
                delta,
                expiration_date,
                date_of_action,
                number_of_shares: quantity,
                credit: price,
            };

            trades.push(trade);
        }

        Ok(trades)
    }

    fn process_robinhood_csv(
        &self,
        mut reader: Reader<File>,
    ) -> Result<Vec<OptionTrade>, Box<dyn std::error::Error>> {
        let mut trades = Vec::new();
        use regex::Regex;
        let option_re = Regex::new(r"(?P<symbol>\w+) (?P<exp>\d{1,2}/\d{1,2}/\d{4}) (?P<type>Call|Put) \$(?P<strike>[\d.]+)").unwrap();
        let date_fmt = time::macros::format_description!("%m/%d/%Y");
        // let ymd_fmt = time::macros::format_description!("[year]-[month]-[day]"); // removed unused
        for result in reader.records() {
            let record = match result {
                Ok(r) if r.len() >= 9 => r,
                _ => continue,
            };
            let activity_date = &record[0];
            // let instrument = &record[3]; // removed unused
            let description = &record[4];
            let trans_code = &record[5];
            let quantity: i32 = record[6].replace(",", "").parse().unwrap_or(0);
            let amount_str = record[7]
                .replace("$", "")
                .replace(",", "")
                .replace("(", "")
                .replace(")", "");
            let amount: f64 = if record[8].contains('(') {
                -amount_str.parse().unwrap_or(0.0)
            } else {
                amount_str.parse().unwrap_or(0.0)
            };

            // Only process option trades
            if let Some(caps) = option_re.captures(description) {
                let symbol = caps.name("symbol").unwrap().as_str().to_string();
                let exp_str = caps.name("exp").unwrap().as_str();
                let option_type = caps.name("type").unwrap().as_str();
                let strike: f64 = caps.name("strike").unwrap().as_str().parse().unwrap_or(0.0);

                // Parse expiration date
                let expiration_date = Date::parse(exp_str, &date_fmt)
                    .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date());
                // Parse activity date
                let date_of_action = Date::parse(activity_date, &date_fmt)
                    .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date());

                // Map trans_code + option_type to Action
                let action = match (trans_code, option_type) {
                    ("BTO", "Call") => Action::BuyCall,
                    ("BTO", "Put") => Action::BuyPut,
                    ("STO", "Call") => Action::SellCall,
                    ("STO", "Put") => Action::SellPut,
                    ("BTC", "Call") => Action::BuyCall, // closing a short call
                    ("BTC", "Put") => Action::BuyPut,   // closing a short put
                    ("STC", "Call") => Action::SellCall, // closing a long call
                    ("STC", "Put") => Action::SellPut,  // closing a long put
                    ("OASGN", _) => Action::Assigned,
                    _ => continue, // skip unknown
                };

                // Delta is not available in Robinhood CSV
                let delta = 0.0;
                // Campaign: use symbol + year + month as a default
                let campaign = format!("{symbol}_{expiration_date}");

                let trade = OptionTrade {
                    id: None,
                    symbol,
                    campaign,
                    action,
                    strike,
                    delta,
                    expiration_date,
                    date_of_action,
                    number_of_shares: quantity * 100, // contracts to shares
                    credit: amount / (quantity as f64 * 100.0), // per share
                };
                trades.push(trade);
            }
        }
        Ok(trades)
    }
}
