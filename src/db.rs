use rusqlite::Connection;

pub fn init_database(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Create campaigns table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS campaigns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            symbol TEXT NOT NULL,
            created_at TEXT NOT NULL,
            target_exit_price REAL
        )",
        [],
    )?;

    // Create option_trades table
    conn.execute(
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

    Ok(())
}
