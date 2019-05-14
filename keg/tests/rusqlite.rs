mod rusqlite {
    use ttrusqlite::{Connection, NO_PARAMS};
    use chrono::{DateTime, Local};
    use keg::{Migration, Connection as _};

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

        let installed_on: DateTime<Local> = conn
            .query_row(
                "SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)",
                NO_PARAMS,
                |row| {
                    let _installed_on: String = row.get(0).unwrap();
                    Ok(DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local))
                }
            )
            .unwrap();
        assert_eq!(Local::today(), installed_on.date());
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

        let installed_on: DateTime<Local> = conn
            .query_row(
                "SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)",
                NO_PARAMS,
                |row| {
                    let _installed_on: String = row.get(0).unwrap();
                    Ok(DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local))
                }
            )
            .unwrap();
        assert_eq!(Local::today(), installed_on.date());
    }

    #[test]
    fn applies_new_migration() {
        let mut conn = Connection::open_in_memory().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        let migration = Migration::new("V4__add_year_field_to_cars", &"ALTER TABLE cars ADD year INTEGER;").unwrap();
        let mchecksum = migration.checksum();
        conn.migrate(&[migration]).unwrap();

        let (current, checksum): (u32, String) = conn
            .query_row(
                "SELECT version, checksum FROM keg_schema_history where version = (SELECT MAX(version) from keg_schema_history)",
                NO_PARAMS,
                |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())),
            )
            .unwrap();
        assert_eq!(4, current);
        assert_eq!(mchecksum.to_string(), checksum);
    }
}
