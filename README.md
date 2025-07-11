# Profit Tracker

A terminal-based options trading campaign tracker written in Rust, using a TUI (Text User Interface) built with [ratatui](https://github.com/ratatui-org/ratatui).

## Features
- Track multiple trading campaigns
- Add, view, and edit option trades
- Calculate campaign summary statistics (P/L, break-even, profit per week, etc.)
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
Run the application from your terminal:

```sh
cargo run --release
```

This will launch the TUI in your terminal window.

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

## License
MIT 