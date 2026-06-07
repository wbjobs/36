mod commands;
mod db;
mod error;
mod imap_client;
mod models;
mod classifier;
mod tray;
mod sync_manager;

use std::sync::Arc;
use tauri::{Manager, SystemTray};
use tokio::sync::Mutex;

pub use error::AppError;
pub use models::*;
pub use sync_manager::{SyncManager, SyncStatus};

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

            let app_state = AppState {
                db: Arc::new(Mutex::new(db)),
                notifier: tray::Notifier::new(app_handle.clone()),
            };

            // 创建并注册同步管理器
            let sync_manager = SyncManager::new(app_handle.clone());
            
            // 创建一个 Arc<Mutex<AppState>> 供同步管理器使用
            let app_state_for_sync = Arc::new(Mutex::new(AppState {
                db: app_state.db.clone(),
                notifier: app_state.notifier.clone(),
            }));

            // 启动后台定时同步
            let sync_manager_clone = sync_manager.clone();
            tauri::async_runtime::spawn(async move {
                // 延迟3秒启动，让应用先初始化完成
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                sync_manager_clone.start_background_sync(app_state_for_sync).await;
            });

            app.manage(app_state);
            app.manage(sync_manager);

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
            commands::get_sync_status,
            commands::trigger_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
