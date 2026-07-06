use chrono::Utc;
use lifeguard_migrate::migration_writer::{
    write_per_table_migration_file, EmissionOutcome, MigrationHeader,
};
use lifeguard_migrate::sql_dependency_order::{
    order_migrations_by_foreign_key_sql, write_apply_order_file, write_seed_order_file,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Recursively collect `.sql` files under `microservices/<svc>/impl/seeds/` that setup-db.sh
/// applies. We use the same glob shape as the legacy `find` invocation so the generated order
/// always matches what would otherwise be the alphabetical fallback.
fn discover_seed_files(microservices_root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let Ok(entries) = std::fs::read_dir(microservices_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let svc_dir = entry.path();
        if !svc_dir.is_dir() {
            continue;
        }
        let seeds_dir = svc_dir.join("impl").join("seeds");
        if !seeds_dir.is_dir() {
            continue;
        }
        let Ok(sub) = std::fs::read_dir(&seeds_dir) else {
            continue;
        };
        for e in sub.flatten() {
            let p = e.path();
            if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("sql") {
                if p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("._"))
                {
                    continue;
                }
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Generate migrations for a single service into `migrations/<service>/`.
///
/// Uses `lifeguard_migrate::migration_writer` so identity detection (merged baselines,
/// normalized SQL comparison) and view handling are centralized — no more duplicate files on
/// re-runs that didn't change any schema, no more view rewrites when the SELECT is identical.
fn write_service_migrations(
    migrations_root: &Path,
    service_name: &str,
    sql_results: Vec<(String, String)>,
    run_timestamp: &str,
) {
    let service_dir = migrations_root.join(service_name);
    if let Err(e) = std::fs::create_dir_all(&service_dir) {
        eprintln!("❌ Could not create {}: {}", service_dir.display(), e);
        return;
    }

    for (table_name, sql) in sql_results {
        let header = MigrationHeader {
            migration_name: &table_name,
            generated_timestamp: run_timestamp,
        };
        match write_per_table_migration_file(
            &service_dir,
            &table_name,
            &sql,
            run_timestamp,
            Some(header),
        ) {
            Ok(EmissionOutcome::Initial { path }) => {
                println!(
                    "✅ Generated initial SQL migration for {}: {}",
                    service_name,
                    path.display()
                );
            }
            Ok(EmissionOutcome::Delta { path }) => {
                println!(
                    "🔄 Generated additive schema evolution for {}: {}",
                    service_name,
                    path.display()
                );
            }
            Ok(EmissionOutcome::Skipped) => {
                println!("⏭️  Skipped identical schema for {service_name}: {table_name}");
            }
            Err(e) => {
                eprintln!("❌ Failed to write migration for {service_name}.{table_name}: {e}");
            }
        }
    }
}

fn main() {
    println!("🚀 Starting Centralized Sesame-IDAM Migrator");

    // Single run-wide timestamp so every file written in this pass sorts together, independent of
    // how long the loop takes for individual tables.
    let run_timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();

    #[derive(Clone)]
    #[allow(clippy::items_after_statements)]
    struct Row {
        service: &'static str,
        table: String,
        sql: String,
    }

    let mut rows: Vec<Row> = Vec::new();

    if let Ok(sql) =
        sesame_idam_identity_login_service::models::entity_registry::generate_sql_for_all()
    {
        for (t, s) in sql {
            rows.push(Row {
                service: "identity-login-service",
                table: t,
                sql: s,
            });
        }
    }

    if let Ok(sql) =
        sesame_idam_identity_session_service::models::entity_registry::generate_sql_for_all()
    {
        for (t, s) in sql {
            rows.push(Row {
                service: "identity-session-service",
                table: t,
                sql: s,
            });
        }
    }

    if let Ok(sql) =
        sesame_idam_identity_user_mgmt_service::models::entity_registry::generate_sql_for_all()
    {
        for (t, s) in sql {
            rows.push(Row {
                service: "identity-user-mgmt-service",
                table: t,
                sql: s,
            });
        }
    }

    if let Ok(sql) = sesame_idam_authz_core::models::entity_registry::generate_sql_for_all() {
        for (t, s) in sql {
            rows.push(Row {
                service: "authz-core",
                table: t,
                sql: s,
            });
        }
    }

    if let Ok(sql) = sesame_idam_api_keys::models::entity_registry::generate_sql_for_all() {
        for (t, s) in sql {
            rows.push(Row {
                service: "api-keys",
                table: t,
                sql: s,
            });
        }
    }

    if let Ok(sql) = sesame_idam_org_mgmt::models::entity_registry::generate_sql_for_all() {
        for (t, s) in sql {
            rows.push(Row {
                service: "org-mgmt",
                table: t,
                sql: s,
            });
        }
    }

    let table_to_service: HashMap<String, &'static str> =
        rows.iter().map(|r| (r.table.clone(), r.service)).collect();

    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.table.clone(), r.sql.clone()))
        .collect();

    eprintln!(
        "DEBUG: {tables_total} tables total",
        tables_total = pairs.len()
    );
    for (name, sql) in &pairs {
        eprintln!("  Table: {name}");
        // Look for REFERENCES in SQL
        if let Some(ref_start) = sql.to_lowercase().find("references") {
            let snippet = &sql[ref_start..std::cmp::min(ref_start + 100, sql.len())];
            eprintln!("    REFERENCES: {snippet}");
        }
    }

    let ordered = match order_migrations_by_foreign_key_sql(pairs) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ FK ordering failed: {e}");
            std::process::exit(1);
        }
    };

    // CARGO_MANIFEST_DIR = <repo>/microservices/migrator → <repo>/migrations
    let migrations_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations");

    for (table, sql) in ordered {
        let service = table_to_service[&table];
        write_service_migrations(
            &migrations_root,
            service,
            vec![(table, sql)],
            &run_timestamp,
        );
    }

    match write_apply_order_file(&migrations_root) {
        Ok(()) => println!(
            "📋 Wrote {}",
            migrations_root.join("apply_order.txt").display()
        ),
        Err(e) => eprintln!("⚠️ Could not write apply_order.txt: {e}"),
    }

    // Seed order — FK-aware, so dependent tables are populated first.
    // Service dirs (containing impl/seeds/) live under <repo>/microservices/idam/.
    let microservices_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let seeds_root = microservices_root.join("idam");
    let seed_files = discover_seed_files(&seeds_root);
    if seed_files.is_empty() {
        println!(
            "(no seeds under {} — skipping seed_order.txt)",
            seeds_root.display()
        );
    } else {
        let seed_order_path = seeds_root.join("seed_order.txt");
        match write_seed_order_file(&migrations_root, &seeds_root, &seed_files, &seed_order_path) {
            Ok(()) => println!("🌱 Wrote {}", seed_order_path.display()),
            Err(e) => eprintln!("⚠️ Could not write seed_order.txt: {e}"),
        }
    }
}
