use crate::{AppResult, AppState};
use crate::classifier::auto_classify_email;
use crate::imap_client::ImapClient;
use crate::models::{Account, SyncResult};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{interval, Duration};
use tauri::AppHandle;

const SYNC_INTERVAL_SECS: u64 = 300; // 5分钟
const BATCH_SIZE: u32 = 200; // 每批拉取200封邮件
const MAX_CONCURRENT_SYNCS: usize = 2; // 最多同时同步2个账户

/// 同步状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum SyncStatus {
    Idle,
    Syncing { account_id: Option<i64> },
    Completed { results: Vec<SyncResult> },
    Error { message: String },
}

/// 同步管理器
pub struct SyncManager {
    app_handle: AppHandle,
    status: Arc<Mutex<SyncStatus>>,
    sync_semaphore: Arc<Semaphore>,
    is_running: Arc<Mutex<bool>>,
}

impl SyncManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            status: Arc::new(Mutex::new(SyncStatus::Idle)),
            sync_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_SYNCS)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// 启动后台定时同步任务
    pub async fn start_background_sync(&self, app_state: Arc<Mutex<AppState>>) {
        let mut running = self.is_running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let app_handle = self.app_handle.clone();
        let status = self.status.clone();
        let sync_semaphore = self.sync_semaphore.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
            
            loop {
                interval.tick().await;
                
                if !*is_running.lock().await {
                    break;
                }

                log::info!("Starting scheduled background sync");
                *status.lock().await = SyncStatus::Syncing { account_id: None };

                match Self::sync_all_accounts(
                    app_state.clone(),
                    app_handle.clone(),
                    sync_semaphore.clone(),
                ).await {
                    Ok(results) => {
                        let has_new = results.iter().any(|r| r.new_emails > 0);
                        if has_new {
                            let total_new: usize = results.iter().map(|r| r.new_emails).sum();
                            Self::send_sync_notification(&app_handle, total_new, &results);
                        }
                        *status.lock().await = SyncStatus::Completed { results };
                        log::info!("Background sync completed successfully");
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        log::error!("Background sync failed: {}", err_msg);
                        *status.lock().await = SyncStatus::Error { message: err_msg.clone() };
                        Self::send_error_notification(&app_handle, &err_msg);
                    }
                }
            }

            log::info!("Background sync stopped");
        });
    }

    /// 停止后台同步
    pub async fn stop_background_sync(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
        *self.status.lock().await = SyncStatus::Idle;
    }

    /// 获取当前同步状态
    pub async fn get_status(&self) -> SyncStatus {
        self.status.lock().await.clone()
    }

    /// 手动触发同步
    pub async fn trigger_sync(
        &self,
        account_id: Option<i64>,
        app_state: Arc<Mutex<AppState>>,
    ) -> AppResult<Vec<SyncResult>> {
        let mut status = self.status.lock().await;
        if matches!(*status, SyncStatus::Syncing { .. }) {
            return Err(crate::AppError::InvalidInput("Sync already in progress".to_string()));
        }
        *status = SyncStatus::Syncing { account_id };
        drop(status);

        let results = Self::sync_all_accounts(
            app_state,
            self.app_handle.clone(),
            self.sync_semaphore.clone(),
        ).await;

        let mut status = self.status.lock().await;
        match &results {
            Ok(r) => {
                *status = SyncStatus::Completed { results: r.clone() };
            }
            Err(e) => {
                *status = SyncStatus::Error { message: e.to_string() };
            }
        }

        results
    }

    /// 同步所有账户
    async fn sync_all_accounts(
        app_state: Arc<Mutex<AppState>>,
        app_handle: AppHandle,
        sync_semaphore: Arc<Semaphore>,
    ) -> AppResult<Vec<SyncResult>> {
        let accounts = {
            let state = app_state.lock().await;
            let db = state.db.lock().await;
            db.list_accounts().await?
        };

        if accounts.is_empty() {
            return Ok(Vec::new());
        }

        let mut handles = Vec::new();

        for account in accounts {
            let app_state_clone = app_state.clone();
            let app_handle_clone = app_handle.clone();
            let permit = sync_semaphore.clone().acquire_owned().await.unwrap();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                let result = Self::sync_single_account(
                    account,
                    app_state_clone.clone(),
                    app_handle_clone.clone(),
                ).await;
                result
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    log::error!("Account sync failed: {}", e);
                }
                Err(e) => {
                    log::error!("Sync task panicked: {}", e);
                }
            }
        }

        // 更新未读计数
        let state = app_state.lock().await;
        let db = state.db.lock().await;
        if let Ok(unread) = db.get_unread_count().await {
            state.notifier.update_unread_count(unread.total);
        }

        Ok(results)
    }

    /// 同步单个账户
    async fn sync_single_account(
        account: Account,
        app_state: Arc<Mutex<AppState>>,
        app_handle: AppHandle,
    ) -> AppResult<SyncResult> {
        log::info!("Starting sync for account: {}", account.email);

        let client = ImapClient::new(account.clone());
        let mut total_new = 0;
        let mut total_updated = 0;
        let mut last_uid: Option<u32> = None;

        loop {
            // 获取当前最大UID
            let current_max_uid = {
                let state = app_state.lock().await;
                let db = state.db.lock().await;
                let conn = db.conn.lock().await;
                conn.query_row(
                    "SELECT COALESCE(MAX(uid), 0) FROM emails WHERE account_id = ?",
                    [account.id],
                    |row| row.get::<_, i64>(0).map(|v| if v > 0 { Some(v as u32) } else { None }),
                )
                .unwrap_or(None)
            };

            // 如果UID没有变化，退出循环
            if last_uid == current_max_uid && last_uid.is_some() {
                break;
            }
            last_uid = current_max_uid;

            // 批量拉取邮件
            let emails = client.fetch_emails(current_max_uid, BATCH_SIZE).await?;
            
            if emails.is_empty() {
                break;
            }

            let tags = {
                let state = app_state.lock().await;
                let db = state.db.lock().await;
                db.get_tags().await?
            };

            for (email, attachments) in emails {
                let is_new = {
                    let state = app_state.lock().await;
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
                    let state = app_state.lock().await;
                    let db = state.db.lock().await;
                    db.insert_email(account.id, &email, &attachments).await?
                };

                if is_new {
                    total_new += 1;

                    let auto_tags = auto_classify_email(&email, &tags);
                    for tag_id in auto_tags {
                        let state = app_state.lock().await;
                        let db = state.db.lock().await;
                        db.add_email_tag(email_id, tag_id).await.ok();
                    }

                    if !email.is_read {
                        let state = app_state.lock().await;
                        state.notifier.notify_new_email(&email.subject, &email.sender_name);
                    }
                } else {
                    total_updated += 1;
                }
            }

            // 如果拉取的邮件少于批次大小，说明已经拉完了
            if emails.len() < BATCH_SIZE as usize {
                break;
            }
        }

        log::info!(
            "Sync completed for {}: {} new, {} updated",
            account.email,
            total_new,
            total_updated
        );

        Ok(SyncResult {
            account_id: account.id,
            account_name: account.name,
            new_emails: total_new,
            updated_emails: total_updated,
        })
    }

    /// 发送同步完成通知
    fn send_sync_notification(app_handle: &AppHandle, total_new: usize, results: &[SyncResult]) {
        if total_new == 0 {
            return;
        }

        let body = if results.len() == 1 {
            let result = &results[0];
            format!("{} 收到 {} 封新邮件", result.account_name, result.new_emails)
        } else {
            format!("共收到 {} 封新邮件", total_new)
        };

        tauri::Notification::new(app_handle)
            .title("邮件同步完成")
            .body(body)
            .show()
            .ok();
    }

    /// 发送错误通知
    fn send_error_notification(app_handle: &AppHandle, error: &str) {
        tauri::Notification::new(app_handle)
            .title("邮件同步失败")
            .body(&format!("错误: {}", error))
            .show()
            .ok();
    }
}
