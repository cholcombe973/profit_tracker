# Profit Tracker

A terminal-based options trading campaign tracker written in Rust, using a TUI (Text User Interface) built with [ratatui](https://github.com/ratatui-org/ratatui).

## Features
- Track multiple trading campaigns
- Add, view, and edit option trades
- Calculate campaign summary statistics (P/L, break-even, profit per week, etc.)
- Import trades from CSV files (supports ETrade and Robinhood formats)
- Persistent storage using SQLite (via rusqlite)
- Intuitive keyboard navigation

## Requirements
- Rust (edition 2021, recommended latest stable)
- SQLite3 (for database file access)

## Building
Clone the repository and build with Cargo:

```sh
cargo build --release
```

## Running

### Interactive TUI Mode
Run the application from your terminal:

```sh
cargo run --release
```

This will launch the TUI in your terminal window.

### CSV Import Mode
Import trades from a CSV file:

```sh
cargo run --release -- import etrade --file etrade.csv --campaign "My Campaign" --symbol AAPL
```

Or for Robinhood:

```sh
cargo run --release -- import robinhood --file robinhood.csv --campaign "My Campaign" --symbol APLD
```

#### Supported Brokers
- **ETrade**: `etrade`
- **Robinhood**: `robinhood`

#### CSV Format Examples

**ETrade Format**

The ETrade CSV should have the following columns:

```
Symbol,Quantity,Price,Date,Action,Strike,Expiration,Delta,Campaign
```

Example:
```
Symbol,Quantity,Price,Date,Action,Strike,Expiration,Delta,Campaign
AAPL,100,2.50,2024-01-15,SellPut,150.00,2024-01-19,-0.30,AAPL_Jan2024
AAPL,100,1.75,2024-01-20,BuyPut,145.00,2024-01-26,-0.25,AAPL_Jan2024
TSLA,50,5.00,2024-01-10,SellCall,250.00,2024-01-12,0.45,TSLA_Jan2024
```

**Robinhood Format**

The Robinhood CSV should have the following columns (as exported from Robinhood):

```
"Activity Date","Process Date","Settle Date","Instrument","Description","Trans Code","Quantity","Price","Amount"
```

Example:
```
"Activity Date","Process Date","Settle Date","Instrument","Description","Trans Code","Quantity","Price","Amount"
"6/25/2025","6/25/2025","6/26/2025","APLD","APLD 6/27/2025 Call $10.00","BTC","3","$0.23","($69.13)"
"6/25/2025","6/25/2025","6/26/2025","APLD","APLD 7/3/2025 Call $11.00","STO","3","$0.20","$59.86"
"6/25/2025","6/25/2025","6/26/2025","NKTR","NKTR 7/18/2025 Call $40.00","STO","1","$6.20","$619.95"
```

- Only option trades (rows where the Description matches the pattern for options) will be imported.
- The parser will extract symbol, expiration, strike, type, and action from the Description and Trans Code fields.

## Usage
- **Campaign Select Screen**: Use `↑`/`↓` to select a campaign. Press `n` to create a new campaign. Press `Enter` to open the selected campaign. Press `q` to quit.
- **New Campaign**: Fill in the name, symbol, and (optionally) target exit price. Use `Tab`/`Shift+Tab` to switch fields. Press `Enter` to save.
- **Campaign Dashboard**: View campaign summary. Press `a` to add a trade, `v` to view trades, or `Esc` to go back.
- **Add Trade**: Fill in trade details. Use `Tab`/`Shift+Tab` to switch fields, `←`/`→` to change action, `Enter` to submit, `Esc` to cancel.
- **View Trades**: Scroll with `↑`/`↓`. Press `e` to edit a trade, `Esc` to return.
- **Edit Trade**: Edit fields as in Add Trade. Press `Enter` to save, `Esc` to cancel.

## Database
- The app creates a SQLite database file named `options_trades.db` in the working directory.
- All campaigns and trades are stored persistently.

## Keyboard Shortcuts
| Screen            | Key(s)         | Action                        |
|-------------------|----------------|-------------------------------|
| Campaign Select   | n              | New campaign                  |
|                   | ↑/↓            | Move selection                |
|                   | Enter          | Select campaign               |
|                   | q              | Quit                          |
| New Campaign      | Tab/Shift+Tab  | Switch field                  |
|                   | Enter          | Save campaign                 |
|                   | Esc            | Cancel                        |
| Dashboard         | a              | Add trade                     |
|                   | v              | View trades                   |
|                   | Esc            | Back to campaign select       |
| Add/Edit Trade    | Tab/Shift+Tab  | Switch field                  |
|                   | ←/→            | Change action (Action field)  |
|                   | Enter          | Save trade                    |
|                   | Esc            | Cancel                        |
| View Trades       | ↑/↓            | Scroll trades                 |
|                   | e              | Edit selected trade           |
|                   | Esc            | Back to dashboard             |

## Troubleshooting
- If you encounter issues with the terminal display, try resizing your terminal window or running in a different terminal emulator.
- The database file must be writable in the current directory.
- For CSV import issues, ensure the file format matches the expected structure and the broker is correctly specified.

## License
MIT 