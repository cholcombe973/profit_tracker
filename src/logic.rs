use crate::models::{Action, OptionTrade};
use time::OffsetDateTime;

pub fn calculate_campaign_summary(
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

    // Find the last open put option strike and shares
    let last_open_put = trades
        .iter()
        .filter(|t| {
            matches!(t.action, Action::SellPut)
                && !trades.iter().any(|other| {
                    matches!(other.action, Action::Assigned)
                        && other.symbol == t.symbol
                        && other.strike == t.strike
                        && other.expiration_date == t.expiration_date
                })
        })
        .max_by(|a, b| a.date_of_action.cmp(&b.date_of_action));

    let running_profit_loss = total_credits - total_debits;

    // Calculate break-even based on last open put strike
    let break_even = if let Some(last_put) = last_open_put {
        let last_strike = last_put.strike;
        let last_shares = last_put.number_of_shares;
        if last_shares > 0 {
            let price_per_share = running_profit_loss / last_shares as f64;
            Some(last_strike - price_per_share)
        } else {
            Some(last_strike)
        }
    } else {
        // Fallback to original calculation if no open puts
        if total_shares_assigned > 0 {
            Some((total_debits - total_credits) / total_shares_assigned as f64)
        } else {
            None
        }
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

    (
        break_even,
        weeks_running,
        profit_per_week,
        total_credits,
        running_profit_loss,
    )
}

pub fn calculate_total_premium_sold(trades: &[OptionTrade]) -> f64 {
    use std::collections::HashMap;

    // Group trades by (symbol, strike, expiration_date) using string key
    let mut contract_groups: HashMap<String, Vec<&OptionTrade>> = HashMap::new();

    for trade in trades {
        let key = format!(
            "{}_{}_{}",
            trade.symbol, trade.strike, trade.expiration_date
        );
        contract_groups.entry(key).or_default().push(trade);
    }

    let mut total_net_premium = 0.0;

    for (_, contract_trades) in contract_groups {
        let mut sold_premium = 0.0;
        let mut bought_premium = 0.0;

        for trade in contract_trades {
            let trade_premium = trade.credit * trade.number_of_shares as f64;

            match trade.action {
                Action::SellPut | Action::SellCall => {
                    sold_premium += trade_premium;
                }
                Action::BuyPut | Action::BuyCall => {
                    bought_premium += trade_premium;
                }
                Action::Exercised | Action::Assigned => {
                    // These are assignment/exercise events, not premium transactions
                    // They don't affect the premium calculation
                }
            }
        }

        // Net premium for this contract = sold - bought
        total_net_premium += sold_premium - bought_premium;
    }
    total_net_premium
}
