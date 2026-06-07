mod commands;
mod db;
mod error;
mod imap_client;
mod models;
mod classifier;
mod tray;

use std::sync::Arc;
use tauri::{Manager, SystemTray};
use tokio::sync::Mutex;

pub use error::AppError;
pub use models::*;

pub type AppResult<T> = Result<T, AppError>;

pub struct AppState {
    pub db: Arc<Mutex<db::Database>>,
    pub notifier: tray::Notifier,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let tray = tray::create_tray();

    tauri::Builder::default()
        .system_tray(tray)
        .on_system_tray_event(tray::handle_tray_event)
        .setup(|app| {
            let app_handle = app.handle();
            let db = tauri::async_runtime::block_on(async {
                db::Database::new().await.expect("Failed to initialize database")
            });

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
                notifier: tray::Notifier::new(app_handle),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_account,
            commands::update_account,
            commands::delete_account,
            commands::list_accounts,
            commands::sync_emails,
            commands::get_emails,
            commands::get_email_detail,
            commands::search_emails,
            commands::mark_email_read,
            commands::mark_email_unread,
            commands::add_email_tag,
            commands::remove_email_tag,
            commands::get_tags,
            commands::get_unread_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
