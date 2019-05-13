mod rusqlite {
    use ttrusqlite::{Connection, NO_PARAMS};

    mod embedded {
        use keg::embed_migrations;
        embed_migrations!("./keg/tests/sql_migrations");
    }

    #[test]
    fn embedded_creates_migration_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        embedded::migrations::run(&mut conn).unwrap();
        let table_name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='keg_schema_history'",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("keg_schema_history", table_name);
    }

    #[test]
    fn embedded_applies_migration() {
        let mut conn = Connection::open_in_memory().unwrap();

        embedded::migrations::run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO persons (name, city) VALUES (?, ?)",
            &[&"John Legend", &"New York"],
        )
        .unwrap();
        let (name, city): (String, String) = conn
            .query_row("SELECT name, city FROM persons", NO_PARAMS, |row| {
                Ok((row.get(0).unwrap(), row.get(1).unwrap()))
            })
            .unwrap();
        assert_eq!("John Legend", name);
        assert_eq!("New York", city);
    }

    #[test]
    fn embedded_updates_schema_history() {
        let mut conn = Connection::open_in_memory().unwrap();

        embedded::migrations::run(&mut conn).unwrap();

        let current: u32 = conn
            .query_row(
                "SELECT MAX(version) FROM keg_schema_history",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(3, current);
    }

    #[test]
    fn mod_creates_migration_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        mod_migrations::migrations::run(&mut conn).unwrap();
        let table_name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='keg_schema_history'",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("keg_schema_history", table_name);
    }

    #[test]
    fn mod_applies_migration() {
        let mut conn = Connection::open_in_memory().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO persons (name, city) VALUES (?, ?)",
            &[&"John Legend", &"New York"],
        )
        .unwrap();
        let (name, city): (String, String) = conn
            .query_row("SELECT name, city FROM persons", NO_PARAMS, |row| {
                Ok((row.get(0).unwrap(), row.get(1).unwrap()))
            })
            .unwrap();
        assert_eq!("John Legend", name);
        assert_eq!("New York", city);
    }

    #[test]
    fn mod_updates_schema_history() {
        let mut conn = Connection::open_in_memory().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();

        let current: u32 = conn
            .query_row(
                "SELECT MAX(version) FROM keg_schema_history",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(3, current);
    }
}