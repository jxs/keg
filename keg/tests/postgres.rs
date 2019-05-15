mod postgres {
    use chrono::{DateTime, Local};
    use keg::{Connection as _, Migration};
    use ttpostgres::{Connection, TlsMode};

    mod embedded {
        use keg::embed_migrations;
        embed_migrations!("./keg/tests/sql_migrations");
    }

    fn clean_database() {
        let conn = Connection::connect(
            "postgres://postgres@localhost:5432/template1",
            TlsMode::None,
        )
        .unwrap();

        conn.execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='postgres'",
            &[],
        )
        .unwrap();
        conn.execute("DROP DATABASE postgres", &[])
            .unwrap();
        conn.execute("CREATE DATABASE POSTGRES", &[])
            .unwrap();
    }

    #[test]
    fn embedded_creates_migration_table() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();
        embedded::migrations::run(&mut conn).unwrap();
        for row in &conn
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_name='keg_schema_history'", &[]
            )
            .unwrap()
        {
            let table_name: String = row.get(0);
            assert_eq!("keg_schema_history", table_name);
        }
        clean_database();
    }

    #[test]
    fn embedded_applies_migration() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();
        embedded::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO persons (name, city) VALUES ($1, $2)",
            &[&"John Legend", &"New York"],
        )
        .unwrap();
        for row in &conn
            .query("SELECT name, city FROM persons", &[])
            .unwrap()
        {
            let name: String = row.get(0);
            let city: String = row.get(1);
            assert_eq!("John Legend", name);
            assert_eq!("New York", city);
        }
        clean_database();
    }

    #[test]
    fn embedded_updates_schema_history() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();

        embedded::migrations::run(&mut conn).unwrap();

        for row in &conn
            .query("SELECT MAX(version) FROM keg_schema_history", &[])
            .unwrap()
        {
            let current = row.get(0);
            assert_eq!(3, current);
        }

        for row in &conn
            .query("SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)", &[])
            .unwrap()
        {
            let _installed_on: String = row.get(0);
            let installed_on = DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local);
            assert_eq!(Local::today(), installed_on.date());
        }
        clean_database();
    }

    #[test]
    fn mod_creates_migration_table() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();
        mod_migrations::migrations::run(&mut conn).unwrap();
        for row in &conn
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_name='keg_schema_history'", &[]
            )
            .unwrap()
        {
            let table_name: String = row.get(0);
            assert_eq!("keg_schema_history", table_name);
        }
        clean_database();
    }

    #[test]
    fn mod_applies_migration() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO persons (name, city) VALUES ($1, $2)",
            &[&"John Legend", &"New York"],
        )
        .unwrap();
        for row in &conn.query("SELECT name, city FROM persons", &[]).unwrap() {
            let name: String = row.get(0);
            let city: String = row.get(1);
            assert_eq!("John Legend", name);
            assert_eq!("New York", city);
        }
        clean_database();
    }

    #[test]
    fn mod_updates_schema_history() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        for row in &conn
            .query("SELECT MAX(version) FROM keg_schema_history", &[])
            .unwrap()
        {
            let current = row.get(0);
            assert_eq!(3, current);
        }

        for row in &conn
            .query("SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)", &[])
            .unwrap()
        {
            let _installed_on: String = row.get(0);
            let installed_on = DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local);
            assert_eq!(Local::today(), installed_on.date());
        }
        clean_database();
    }

    #[test]
    fn applies_new_migration() {
        let mut conn =
            Connection::connect("postgres://postgres@localhost:5432/postgres", TlsMode::None)
                .unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        let migration = Migration::new(
            "V4__add_year_field_to_cars",
            &"ALTER TABLE cars ADD year INTEGER;",
        )
        .unwrap();
        let mchecksum = migration.checksum();
        conn.migrate(&[migration]).unwrap();

        for row in &conn
            .query("SELECT version, checksum FROM keg_schema_history where version = (SELECT MAX(version) from keg_schema_history)", &[])
            .unwrap()
        {
            let current = row.get(0);
            let checksum: String = row.get(1);
            assert_eq!(4, current);
            assert_eq!(mchecksum.to_string(), checksum);
        }
    }
}
