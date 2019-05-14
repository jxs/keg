mod rusqlite {
    use ttmysql as my;
    use chrono::{DateTime, Local};
    use keg::{Migration, Connection as _};

    mod embedded {
        use keg::embed_migrations;
        embed_migrations!("./keg/tests/sql_migrations");
    }

    fn clean_database() {
        let mut conn =
            my::Conn::new("mysql://keg:root@localhost:3306/keg_test").unwrap();

        conn.prep_exec("DROP DATABASE keg_test", ()).expect("drop database");
        conn.prep_exec("CREATE DATABASE keg_test", ()).expect("create database");
    }

    #[test]
    fn embedded_creates_migration_table() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();
        embedded::migrations::run(&mut conn).unwrap();
        for row in conn
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_name='keg_schema_history'"
            )
            .expect("queryy")
        {
            let table_name: String = row.unwrap().get(0).unwrap();
            assert_eq!("keg_schema_history", table_name);
        }
        clean_database();
    }
    
    #[test]
    fn embedded_applies_migration() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();

        embedded::migrations::run(&mut conn).unwrap();
        conn.prep_exec(
            "INSERT INTO persons (name, city) VALUES (:a, :b)",
            (&"John Legend", &"New York"),
        )
        .expect("query query");
        for _row in conn.query("SELECT name, city FROM persons").unwrap() {
            let row = _row.unwrap(); 
            let name: String = row.get(0).unwrap();
            let city: String = row.get(1).unwrap();
            assert_eq!("John Legend", name);
            assert_eq!("New York", city);
        }
        clean_database();
    }

    #[test]
    fn embedded_updates_schema_history() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();

        embedded::migrations::run(&mut conn).unwrap();

        for _row in conn
            .query("SELECT MAX(version) FROM keg_schema_history")
            .unwrap()
        {
            let row = _row.unwrap();
            let current: i32 = row.get(0).unwrap();
            assert_eq!(3, current);
        }

        for _row in conn
            .query("SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)")
            .unwrap()
        {
            let row = _row.unwrap();
            let _installed_on: String = row.get(0).unwrap();
            let installed_on = DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local);
            assert_eq!(Local::today(), installed_on.date());
        }
        clean_database();
    }

    #[test]
    fn mod_creates_migration_table() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();
        mod_migrations::migrations::run(&mut conn).unwrap();
        for row in conn
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_name='keg_schema_history'"
            )
            .expect("queryy")
        {
            let table_name: String = row.unwrap().get(0).unwrap();
            assert_eq!("keg_schema_history", table_name);
        }
        clean_database();
    }

    #[test]
    fn mod_applies_migration() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        conn.prep_exec(
            "INSERT INTO persons (name, city) VALUES (:a, :b)",
            (&"John Legend", &"New York"),
        )
        .expect("query query");
        for _row in conn.query("SELECT name, city FROM persons").unwrap() {
            let row = _row.unwrap(); 
            let name: String = row.get(0).unwrap();
            let city: String = row.get(1).unwrap();
            assert_eq!("John Legend", name);
            assert_eq!("New York", city);
        }
        clean_database();
    }

    #[test]
    fn mod_updates_schema_history() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();

        for _row in conn
            .query("SELECT MAX(version) FROM keg_schema_history")
            .unwrap()
        {
            let row = _row.unwrap();
            let current: i32 = row.get(0).unwrap();
            assert_eq!(3, current);
        }

        for _row in conn
            .query("SELECT installed_on FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)")
            .unwrap()
        {
            let row = _row.unwrap();
            let _installed_on: String = row.get(0).unwrap();
            let installed_on = DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local);
            assert_eq!(Local::today(), installed_on.date());
        }
        clean_database();
    }

    #[test]
    fn applies_new_migration() {
        let pool = my::Pool::new("mysql://keg:root@localhost:3306/keg_test").unwrap();
        let mut conn = pool.get_conn().unwrap();

        mod_migrations::migrations::run(&mut conn).unwrap();
        let migration = Migration::new("V4__add_year_field_to_cars", &"ALTER TABLE cars ADD year INTEGER;").unwrap();
        conn.migrate(&[migration]).unwrap();

        for _row in conn
            .query("SELECT MAX(version) FROM keg_schema_history")
            .unwrap()
        {
            let row = _row.unwrap();
            let current: i32 = row.get(0).unwrap();
            assert_eq!(4, current);
        }
    }
}