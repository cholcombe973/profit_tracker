use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    BuyPut,
    SellPut,
    BuyCall,
    SellCall,
    Exercised,
    Assigned,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionTrade {
    pub id: Option<i32>,
    pub symbol: String,
    pub campaign: String,
    pub action: Action,
    pub strike: f64,
    pub delta: f64,
    pub expiration_date: Date,
    pub date_of_action: Date,
    pub number_of_shares: i32,
    pub credit: f64,
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
        use time::macros::format_description;
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
pub struct Campaign {
    pub name: String,
    pub symbol: String,
    pub target_exit_price: Option<f64>,
}

impl Campaign {
    pub fn get_all(conn: &Connection) -> Vec<Campaign> {
        let mut stmt = conn
            .prepare(
                "SELECT name, symbol, target_exit_price FROM campaigns ORDER BY created_at DESC",
            )
            .unwrap();
        let iter = stmt
            .query_map([], |row| {
                Ok(Campaign {
                    name: row.get(0)?,
                    symbol: row.get(1)?,
                    target_exit_price: row.get(2)?,
                })
            })
            .unwrap();
        iter.filter_map(Result::ok).collect()
    }
    pub fn insert(
        conn: &Connection,
        name: &str,
        symbol: &str,
        target_exit_price: Option<f64>,
    ) -> Option<Campaign> {
        use time::OffsetDateTime;
        let now = OffsetDateTime::now_local().unwrap().date().to_string();
        let _ = conn.execute(
            "INSERT INTO campaigns (name, symbol, created_at, target_exit_price) VALUES (?1, ?2, ?3, ?4)",
            params![name, symbol, now, target_exit_price],
        );
        Some(Campaign {
            name: name.to_string(),
            symbol: symbol.to_string(),
            target_exit_price,
        })
    }
}
