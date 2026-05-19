use std::sync::LazyLock;

pub struct TestDatabase {
    pub db_url: String,
}

pub static TEST_DB: LazyLock<TestDatabase> = LazyLock::new(|| {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("TEST_DB_USER").unwrap_or_else(|_| "postgres".to_string());
    let pass = std::env::var("TEST_DB_PASS").unwrap_or_else(|_| "postgres".to_string());
    let db_name = std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "idam".to_string());

    let db_url = format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db_name);

    // Set DB environment variables for any code that reads from env
    std::env::set_var("DB_HOST", &host);
    std::env::set_var("DB_PORT", &port);
    std::env::set_var("DB_USER", &user);
    std::env::set_var("DB_PASS", &pass);
    std::env::set_var("DB_NAME", &db_name);

    // Poll for DB availability
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let client = loop {
        match may_postgres::connect(&db_url) {
            Ok(c) => break c,
            Err(e) => {
                eprintln!("Waiting for DB to accept connections... {e:?}");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    };

    // Create schema for authz-core
    client
        .batch_execute("CREATE SCHEMA IF NOT EXISTS sesame_idam;")
        .unwrap_or_else(|e| eprintln!("Schema warning: {e}"));
    client
        .batch_execute("SET search_path TO sesame_idam, public;")
        .unwrap_or_else(|e| eprintln!("Search path warning: {e}"));

    // Run seed SQL files if present
    let seeds_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("seeds");
    if seeds_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&seeds_dir)
            .expect("read seeds dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().map(|e| e == "sql").unwrap_or(false))
            .collect();
        entries.sort();
        for seed in &entries {
            let sql = std::fs::read_to_string(seed)
                .unwrap_or_else(|e| panic!("read seed {:?}: {e}", seed));
            let full_sql = format!("SET search_path TO sesame_idam, public;\n{sql}");
            if let Err(e) = client.batch_execute(&full_sql) {
                eprintln!("Seed {seed:?} warning: {e}");
            } else {
                eprintln!("Applied seed: {:?}", seed.file_name());
            }
        }
    }

    TestDatabase { db_url }
});
