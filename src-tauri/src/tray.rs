use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

pub struct Notifier {
    app_handle: AppHandle,
}

impl Notifier {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn notify_new_email(&self, subject: &str, sender: &str) {
        tauri::Notification::new(&self.app_handle)
            .title("新邮件")
            .body(format!("{} - {}", sender, subject))
            .show()
            .ok();
    }

    pub fn update_unread_count(&self, count: i64) {
        if let Some(tray) = self.app_handle.tray_handle_by_id("main") {
            let tooltip = if count > 0 {
                format!("邮件客户端 - {} 封未读", count)
            } else {
                "邮件客户端".to_string()
            };
            tray.set_tooltip(&tooltip).ok();
        }
    }
}

pub fn create_tray() -> SystemTray {
    let show = CustomMenuItem::new("show".to_string(), "显示主窗口");
    let sync = CustomMenuItem::new("sync".to_string(), "同步邮件");
    let quit = CustomMenuItem::new("quit".to_string(), "退出");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(sync)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new()
        .with_id("main")
        .with_menu(tray_menu)
}

pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            show_main_window(app);
        }
        SystemTrayEvent::DoubleClick { .. } => {
            show_main_window(app);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "show" => {
                show_main_window(app);
            }
            "sync" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<crate::AppState>();
                    let db = state.db.lock().await;
                    match db.list_accounts().await {
                        Ok(accounts) => {
                            for account in accounts {
                                if let Err(e) = sync_account(&app_handle, &account).await {
                                    log::error!("Sync failed for {}: {}", account.email, e);
                                }
                            }
                        }
                        Err(e) => log::error!("Failed to list accounts: {}", e),
                    }
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        window.show().ok();
        window.unminimize().ok();
        window.set_focus().ok();
    }
}

async fn sync_account(app: &AppHandle, account: &crate::models::Account) -> crate::AppResult<()> {
    use crate::imap_client::ImapClient;
    use crate::classifier::auto_classify_email;

    let state = app.state::<crate::AppState>();

    let max_uid: Option<u32> = {
        let db = state.db.lock().await;
        let conn = db.conn.lock().await;
        conn.query_row(
            "SELECT COALESCE(MAX(uid), 0) FROM emails WHERE account_id = ?",
            [account.id],
            |row| row.get::<_, i64>(0).map(|v| if v > 0 { Some(v as u32) } else { None }),
        )
        .unwrap_or(None)
    };

    let client = ImapClient::new(account.clone());
    let emails = client.fetch_emails(max_uid, 50).await?;

    let tags = {
        let db = state.db.lock().await;
        db.get_tags().await?
    };
    
    let mut new_count = 0;

    for (email, attachments) in emails {
        let is_new = {
            let db = state.db.lock().await;
            let conn = db.conn.lock().await;
            conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM emails WHERE account_id = ? AND message_id = ?",
                [account.id, &email.message_id],
                |row| row.get(0),
            )
            .unwrap_or(0) == 0
        };

        let email_id = {
            let db = state.db.lock().await;
            db.insert_email(account.id, &email, &attachments).await?
        };

        if is_new {
            new_count += 1;

            let auto_tags = auto_classify_email(&email, &tags);
            for tag_id in auto_tags {
                let db = state.db.lock().await;
                db.add_email_tag(email_id, tag_id).await.ok();
            }

            if !email.is_read {
                state.notifier.notify_new_email(&email.subject, &email.sender_name);
            }
        }
    }

    let unread = {
        let db = state.db.lock().await;
        db.get_unread_count().await?
    };
    state.notifier.update_unread_count(unread.total);

    Ok(())
}
