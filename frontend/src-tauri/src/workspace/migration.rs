use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::Row;

use super::manager::WorkspaceManager;

/// Migrate an existing single-database installation to the workspace architecture.
///
/// Steps (in exact order):
/// 1. Backup original database
/// 2. Checkpoint WAL
/// 3. Create Default workspace
/// 4. Copy DB to workspace path
/// 5. Extract settings to global DB
/// 6. Clean workspace DB copy
/// 7. Verify audio file accessibility
/// 8. Switch to Default workspace
/// 9. Verify data integrity
pub async fn migrate_existing_database_to_workspace(
    workspace_mgr: &WorkspaceManager,
    existing_db_path: &Path,
) -> Result<String, String> {
    log::info!(
        "Starting migration of existing database: {:?}",
        existing_db_path
    );

    // ── Step 1: Backup the original database ──────────────────────────────
    log::info!("Step 1/9: Backing up original database...");
    let backup_path = existing_db_path.with_extension("sqlite.pre-workspace-backup");
    std::fs::copy(existing_db_path, &backup_path)
        .map_err(|e| format!("Failed to backup original database: {}", e))?;
    log::info!("Backed up database to {:?}", backup_path);

    // Also backup WAL and SHM files if they exist
    let wal_path = existing_db_path.with_extension("sqlite-wal");
    if wal_path.exists() {
        let wal_backup = existing_db_path.with_extension("sqlite-wal.pre-workspace-backup");
        std::fs::copy(&wal_path, &wal_backup)
            .map_err(|e| format!("Failed to backup WAL file: {}", e))?;
        log::info!("Backed up WAL file to {:?}", wal_backup);
    }

    let shm_path = existing_db_path.with_extension("sqlite-shm");
    if shm_path.exists() {
        let shm_backup = existing_db_path.with_extension("sqlite-shm.pre-workspace-backup");
        std::fs::copy(&shm_path, &shm_backup)
            .map_err(|e| format!("Failed to backup SHM file: {}", e))?;
        log::info!("Backed up SHM file to {:?}", shm_backup);
    }

    // ── Step 2: Checkpoint existing DB WAL ────────────────────────────────
    log::info!("Step 2/9: Checkpointing WAL...");
    {
        let ckpt_options = SqliteConnectOptions::new()
            .filename(existing_db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let ckpt_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ckpt_options)
            .await
            .map_err(|e| format!("Failed to open DB for WAL checkpoint: {}", e))?;

        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&ckpt_pool)
            .await
            .map_err(|e| format!("WAL checkpoint failed: {}", e))?;

        ckpt_pool.close().await;
    }
    log::info!("WAL checkpoint complete");

    // ── Step 3: Create Default workspace ──────────────────────────────────
    log::info!("Step 3/9: Creating Default workspace...");
    let default_id = workspace_mgr
        .create_workspace("Default".to_string())
        .await?;
    log::info!("Created Default workspace: {}", default_id);

    // ── Step 4: Copy existing DB to workspace path ────────────────────────
    log::info!("Step 4/9: Copying database to workspace...");
    let ws_db_path = workspace_mgr
        .workspaces_root()
        .join(&default_id)
        .join("db.sqlite");
    std::fs::copy(existing_db_path, &ws_db_path)
        .map_err(|e| format!("Failed to copy DB to workspace: {}", e))?;
    log::info!("Copied database to {:?}", ws_db_path);

    // ── Step 5: Extract settings from ORIGINAL database into global DB ───
    log::info!("Step 5/9: Extracting settings to global database...");
    {
        let orig_options = SqliteConnectOptions::new()
            .filename(existing_db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let orig_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(orig_options)
            .await
            .map_err(|e| {
                format!(
                    "Failed to open original DB for settings extraction: {}",
                    e
                )
            })?;

        let global_pool = workspace_mgr.global_pool();

        // Migrate settings table
        let settings_rows = sqlx::query("SELECT * FROM settings")
            .fetch_all(&orig_pool)
            .await
            .map_err(|e| format!("Failed to read settings: {}", e))?;

        for row in &settings_rows {
            let id: String = row.try_get("id").unwrap_or_default();
            let provider: String = row.try_get("provider").unwrap_or_default();
            let model: String = row.try_get("model").unwrap_or_default();
            let whisper_model: String = row.try_get("whisperModel").unwrap_or_default();
            let groq_key: Option<String> = row.try_get("groqApiKey").ok();
            let openai_key: Option<String> = row.try_get("openaiApiKey").ok();
            let anthropic_key: Option<String> = row.try_get("anthropicApiKey").ok();
            let ollama_key: Option<String> = row.try_get("ollamaApiKey").ok();
            let openrouter_key: Option<String> = row.try_get("openRouterApiKey").ok();
            let ollama_endpoint: Option<String> = row.try_get("ollamaEndpoint").ok();
            let custom_openai: Option<String> = row.try_get("customOpenAIConfig").ok();
            let gemini_key: Option<String> = row.try_get("geminiApiKey").ok();

            sqlx::query(
                "INSERT OR REPLACE INTO settings (id, provider, model, whisperModel, groqApiKey, openaiApiKey, anthropicApiKey, ollamaApiKey, openRouterApiKey, ollamaEndpoint, customOpenAIConfig, geminiApiKey) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id).bind(&provider).bind(&model).bind(&whisper_model)
            .bind(&groq_key).bind(&openai_key).bind(&anthropic_key).bind(&ollama_key)
            .bind(&openrouter_key).bind(&ollama_endpoint).bind(&custom_openai).bind(&gemini_key)
            .execute(global_pool)
            .await
            .map_err(|e| format!("Failed to insert settings row: {}", e))?;
        }
        log::info!("Migrated {} settings rows", settings_rows.len());

        // Migrate transcript_settings table
        let ts_rows = sqlx::query("SELECT * FROM transcript_settings")
            .fetch_all(&orig_pool)
            .await
            .map_err(|e| format!("Failed to read transcript_settings: {}", e))?;

        for row in &ts_rows {
            let id: String = row.try_get("id").unwrap_or_default();
            let provider: String = row.try_get("provider").unwrap_or_default();
            let model: String = row.try_get("model").unwrap_or_default();
            let whisper_key: Option<String> = row.try_get("whisperApiKey").ok();
            let deepgram_key: Option<String> = row.try_get("deepgramApiKey").ok();
            let eleven_key: Option<String> = row.try_get("elevenLabsApiKey").ok();
            let groq_key: Option<String> = row.try_get("groqApiKey").ok();
            let openai_key: Option<String> = row.try_get("openaiApiKey").ok();

            sqlx::query(
                "INSERT OR REPLACE INTO transcript_settings (id, provider, model, whisperApiKey, deepgramApiKey, elevenLabsApiKey, groqApiKey, openaiApiKey) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id).bind(&provider).bind(&model)
            .bind(&whisper_key).bind(&deepgram_key).bind(&eleven_key).bind(&groq_key).bind(&openai_key)
            .execute(global_pool)
            .await
            .map_err(|e| format!("Failed to insert transcript_settings row: {}", e))?;
        }
        log::info!("Migrated {} transcript_settings rows", ts_rows.len());

        // Migrate licensing table (if exists)
        let has_licensing: bool =
            sqlx::query_scalar::<_, i32>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='licensing'",
            )
            .fetch_one(&orig_pool)
            .await
            .map(|c| c > 0)
            .unwrap_or(false);

        if has_licensing {
            let lic_rows = sqlx::query("SELECT * FROM licensing")
                .fetch_all(&orig_pool)
                .await
                .unwrap_or_default();

            for row in &lic_rows {
                let license_key: String = row.try_get("license_key").unwrap_or_default();
                let encrypted_key: String = row.try_get("encrypted_key").unwrap_or_default();
                let signature_hash: String = row.try_get("signature_hash").unwrap_or_default();
                let activation_date: String = row.try_get("activation_date").unwrap_or_default();
                let expiry_date: String = row.try_get("expiry_date").unwrap_or_default();
                let soft_expiry_date: String = row.try_get("soft_expiry_date").unwrap_or_default();
                let max_activation_time: String =
                    row.try_get("max_activation_time").unwrap_or_default();
                let duration: i64 = row.try_get("duration").unwrap_or_default();
                let generated_on: String = row.try_get("generated_on").unwrap_or_default();
                let is_soft_expired: i32 = row.try_get("is_soft_expired").unwrap_or_default();
                let grace_period: i32 = row.try_get("grace_period").unwrap_or(604800);

                sqlx::query(
                    "INSERT OR REPLACE INTO licensing (license_key, encrypted_key, signature_hash, activation_date, expiry_date, soft_expiry_date, max_activation_time, duration, generated_on, is_soft_expired, grace_period) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&license_key).bind(&encrypted_key).bind(&signature_hash)
                .bind(&activation_date).bind(&expiry_date).bind(&soft_expiry_date)
                .bind(&max_activation_time).bind(duration).bind(&generated_on)
                .bind(is_soft_expired).bind(grace_period)
                .execute(global_pool)
                .await
                .map_err(|e| format!("Failed to insert licensing row: {}", e))?;
            }
            log::info!("Migrated {} licensing rows", lic_rows.len());
        } else {
            log::info!("No licensing table found in original database, skipping");
        }

        // Close original DB pool
        orig_pool.close().await;
    }

    // ── Step 6: Clean the workspace DB copy ───────────────────────────────
    log::info!("Step 6/9: Cleaning global tables from workspace DB copy...");
    {
        let ws_options = SqliteConnectOptions::new()
            .filename(&ws_db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let ws_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ws_options)
            .await
            .map_err(|e| format!("Failed to open workspace DB for cleaning: {}", e))?;

        let tables_to_drop = [
            "settings",
            "transcript_settings",
            "licensing",
            "custom_openai_config",
            "_sqlx_migrations",
        ];
        for table in &tables_to_drop {
            let sql = format!("DROP TABLE IF EXISTS {}", table);
            sqlx::query(&sql)
                .execute(&ws_pool)
                .await
                .map_err(|e| format!("Failed to drop table {} from workspace DB: {}", table, e))?;
        }
        log::info!(
            "Dropped global-only tables from workspace DB: {:?}",
            tables_to_drop
        );

        ws_pool.close().await;
    }

    // ── Step 7: Verify audio file accessibility ───────────────────────────
    log::info!("Step 7/9: Verifying audio file accessibility...");
    {
        let ws_options = SqliteConnectOptions::new()
            .filename(&ws_db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let ws_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ws_options)
            .await
            .map_err(|e| format!("Failed to open workspace DB for audio verification: {}", e))?;

        let rows = sqlx::query(
            "SELECT id, folder_path FROM meetings WHERE folder_path IS NOT NULL AND folder_path != ''",
        )
        .fetch_all(&ws_pool)
        .await
        .unwrap_or_default();

        let total = rows.len();
        let mut accessible = 0usize;
        let mut inaccessible = 0usize;

        for row in &rows {
            let id: String = row.try_get("id").unwrap_or_default();
            let folder_path: String = row.try_get("folder_path").unwrap_or_default();
            if Path::new(&folder_path).exists() {
                accessible += 1;
            } else {
                inaccessible += 1;
                log::warn!(
                    "Meeting {} has inaccessible folder_path: {}",
                    id,
                    folder_path
                );
            }
        }
        log::info!(
            "Audio file check: {}/{} accessible, {} inaccessible",
            accessible,
            total,
            inaccessible
        );

        ws_pool.close().await;
    }

    // ── Step 8: Switch to Default workspace ───────────────────────────────
    log::info!("Step 8/9: Switching to Default workspace...");
    workspace_mgr.switch_workspace(&default_id).await?;
    log::info!("Switched to Default workspace");

    // ── Step 9: Verify data integrity ─────────────────────────────────────
    log::info!("Step 9/9: Verifying data integrity...");
    {
        let ws_pool = workspace_mgr.active_pool().await?;
        let ws_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings")
            .fetch_one(&ws_pool)
            .await
            .map_err(|e| format!("Failed to count workspace meetings: {}", e))?;

        // Open original DB to compare counts
        let orig_options = SqliteConnectOptions::new()
            .filename(existing_db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let orig_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(orig_options)
            .await
            .map_err(|e| format!("Failed to open original DB for verification: {}", e))?;

        let orig_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings")
            .fetch_one(&orig_pool)
            .await
            .map_err(|e| format!("Failed to count original meetings: {}", e))?;

        orig_pool.close().await;

        if ws_count == orig_count {
            log::info!(
                "Data integrity verified: {} meetings in both original and workspace",
                ws_count
            );
        } else {
            log::warn!(
                "Meeting count mismatch! Original: {}, Workspace: {}",
                orig_count,
                ws_count
            );
        }
    }

    log::info!(
        "Migration complete. Default workspace ID: {}",
        default_id
    );
    Ok(default_id)
}
