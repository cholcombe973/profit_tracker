use crate::models::{Action, OptionTrade};
use csv::Reader;
use std::fs::File;
use std::path::Path;
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

    pub fn process_csv<P: AsRef<Path>>(
        &self,
        file_path: P,
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
        let date_fmt = time::macros::format_description!(
            "[month]/[day]/[year] [hour]:[minute]:[second] [period]"
        );

        for result in reader.records() {
            let record = match result {
                Ok(r) if r.len() >= 8 => r,
                _ => continue,
            };

            let date_str = record[0].trim_matches('"').trim();
            let type_str = record[1].trim_matches('"').trim();
            let description = record[4].trim_matches('"').trim();
            let amount_str = record[7]
                .replace("$", "")
                .replace(",", "")
                .replace("(", "")
                .replace(")", "");
            let amount: f64 = if record[7].contains('(') {
                -amount_str.parse().unwrap_or(0.0)
            } else {
                amount_str.parse().unwrap_or(0.0)
            };

            // Split description on spaces to extract option trade details
            // Format: "15 Put NVTS 07/03/25 6.500 @ $0.18"
            let parts: Vec<&str> = description.split_whitespace().collect();

            // Only process if we have enough parts and it looks like an option trade
            if parts.len() >= 6 && (parts[1] == "Put" || parts[1] == "Call") {
                let qty: i32 = parts[0].parse().unwrap_or(0);
                let option_type = parts[1];
                let symbol = parts[2].to_string();
                let exp_str = parts[3];
                let strike: f64 = parts[4].parse().unwrap_or(0.0);
                // Price is after "@" symbol, so parts[6] should be the price
                let _price_per_contract: f64 = if parts.len() > 6 && parts[5] == "@" {
                    parts[6].trim_start_matches('$').parse().unwrap_or(0.0)
                } else {
                    0.0
                };

                // Parse expiration date (MM/DD/YY)
                let exp_parts: Vec<&str> = exp_str.split('/').collect();
                let expiration_date = if exp_parts.len() == 3 {
                    let month: u8 = exp_parts[0].parse().unwrap_or(1);
                    let day: u8 = exp_parts[1].parse().unwrap_or(1);
                    let year: u16 = exp_parts[2].parse().unwrap_or(0);
                    let year = if year < 100 {
                        2000 + year as i32
                    } else {
                        year as i32
                    };
                    Date::from_calendar_date(
                        year,
                        time::Month::try_from(month).unwrap_or(time::Month::January),
                        day,
                    )
                    .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date())
                } else {
                    OffsetDateTime::now_local().unwrap().date()
                };

                // Parse date of action
                let date_of_action = Date::parse(date_str, &date_fmt)
                    .unwrap_or_else(|_| OffsetDateTime::now_local().unwrap().date());

                // Map type_str and option_type to Action
                let action = match (type_str, option_type) {
                    ("Sold", "Put") => Action::SellPut,
                    ("Sold", "Call") => Action::SellCall,
                    ("Bought", "Put") => Action::BuyPut,
                    ("Bought", "Call") => Action::BuyCall,
                    ("Sold Short", "Put") => Action::SellPut,
                    ("Sold Short", "Call") => Action::SellCall,
                    ("Bought To Cover", "Put") => Action::BuyPut,
                    ("Bought To Cover", "Call") => Action::BuyCall,
                    _ => continue, // skip unknown
                };

                // Delta is not available
                let delta = 0.0;
                // Campaign: use symbol + year + month as a default
                let campaign = symbol.clone();

                let number_of_shares = qty * 100;
                let credit = amount / (qty as f64 * 100.0); // per share

                let trade = OptionTrade {
                    id: None,
                    symbol,
                    campaign,
                    action,
                    strike,
                    delta,
                    expiration_date,
                    date_of_action,
                    number_of_shares,
                    credit,
                };
                trades.push(trade);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Action;
    use time::macros::date;

    #[test]
    fn test_process_etrade_csv() {
        let processor = CsvProcessor::new(Broker::ETrade);
        let result = processor.process_csv("tests/etrade.csv");

        assert!(result.is_ok(), "Failed to process CSV: {:?}", result.err());

        let trades = result.unwrap();
        assert!(!trades.is_empty(), "No trades were parsed from the CSV");

        // Test specific trades from the CSV
        let put_trades: Vec<_> = trades
            .iter()
            .filter(|t| t.symbol == "NVTS" && t.action == Action::SellPut)
            .collect();

        assert!(!put_trades.is_empty(), "No NVTS Put trades found");

        // Check that we have the expected trade from the first line
        let nvts_trade = put_trades
            .iter()
            .find(|t| t.strike == 6.5 && t.number_of_shares == 1500)
            .expect("Expected NVTS Put trade with strike 6.5 and 1500 shares");

        assert_eq!(nvts_trade.symbol, "NVTS");
        assert_eq!(nvts_trade.action, Action::SellPut);
        assert_eq!(nvts_trade.strike, 6.5);
        assert_eq!(nvts_trade.number_of_shares, 1500);
        assert_eq!(nvts_trade.expiration_date, date!(2025 - 07 - 03));

        // Test RKLB trades
        let rklb_trades: Vec<_> = trades.iter().filter(|t| t.symbol == "RKLB").collect();

        assert!(!rklb_trades.is_empty(), "No RKLB trades found");

        // Test HOOD trades
        let hood_trades: Vec<_> = trades.iter().filter(|t| t.symbol == "HOOD").collect();

        assert!(!hood_trades.is_empty(), "No HOOD trades found");

        // Verify that non-option entries are filtered out
        let non_option_trades: Vec<_> = trades
            .iter()
            .filter(|t| t.symbol == "TSLA") // TSLA trades in CSV are stock trades, not options
            .collect();

        assert!(
            non_option_trades.is_empty(),
            "Stock trades should be filtered out"
        );

        println!(
            "Successfully parsed {} option trades from E*TRADE CSV",
            trades.len()
        );

        // Print some sample trades for debugging
        for (i, trade) in trades.iter().take(5).enumerate() {
            println!(
                "Trade {}: {} {} @ ${:.2} exp: {} shares: {} credit: ${:.2}",
                i + 1,
                trade.symbol,
                match trade.action {
                    Action::BuyPut => "BuyPut",
                    Action::SellPut => "SellPut",
                    Action::BuyCall => "BuyCall",
                    Action::SellCall => "SellCall",
                    Action::Exercised => "Exercised",
                    Action::Assigned => "Assigned",
                },
                trade.strike,
                trade.expiration_date,
                trade.number_of_shares,
                trade.credit
            );
        }
    }
}
