use crate::AppState;
use crate::AppResult;
use crate::SyncManager;
use crate::SyncStatus;
use crate::classifier::auto_classify_email;
use crate::imap_client::ImapClient;
use crate::models::*;
use tauri::{command, State};

#[command]
pub async fn add_account(account: AccountInput, state: State<'_, AppState>) -> AppResult<Account> {
    let db = state.db.lock().await;
    db.add_account(&account).await
}

#[command]
pub async fn update_account(
    id: i64,
    account: AccountInput,
    state: State<'_, AppState>,
) -> AppResult<Account> {
    let db = state.db.lock().await;
    db.update_account(id, &account).await
}

#[command]
pub async fn delete_account(id: i64, state: State<'_, AppState>) -> AppResult<()> {
    let db = state.db.lock().await;
    db.delete_account(id).await
}

#[command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    let db = state.db.lock().await;
    db.list_accounts().await
}

#[command]
pub async fn sync_emails(
    account_id: Option<i64>,
    state: State<'_, AppState>,
) -> AppResult<Vec<SyncResult>> {
    let accounts = {
        let db = state.db.lock().await;
        match account_id {
            Some(id) => vec![db.get_account(id).await?],
            None => db.list_accounts().await?,
        }
    };

    let mut results = Vec::new();

    for account in accounts {
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
        let mut updated_count = 0;

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
            } else {
                updated_count += 1;
            }
        }

        results.push(SyncResult {
            account_id: account.id,
            account_name: account.name.clone(),
            new_emails: new_count,
            updated_emails: updated_count,
        });
    }

    let unread = {
        let db = state.db.lock().await;
        db.get_unread_count().await?
    };
    state.notifier.update_unread_count(unread.total);

    Ok(results)
}

#[command]
pub async fn get_emails(
    account_id: Option<i64>,
    tag_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AppState>,
) -> AppResult<EmailListResult> {
    let db = state.db.lock().await;
    db.get_emails(account_id, tag_id, limit.unwrap_or(50), offset.unwrap_or(0)).await
}

#[command]
pub async fn get_email_detail(id: i64, state: State<'_, AppState>) -> AppResult<EmailDetail> {
    let db = state.db.lock().await;
    let detail = db.get_email_detail(id).await?;
    if !detail.email.is_read {
        db.mark_email_read(id, true).await?;
        let unread = db.get_unread_count().await?;
        state.notifier.update_unread_count(unread.total);
    }
    Ok(detail)
}

#[command]
pub async fn search_emails(
    query: String,
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AppState>,
) -> AppResult<EmailListResult> {
    let db = state.db.lock().await;
    db.search_emails(&query, limit.unwrap_or(50), offset.unwrap_or(0)).await
}

#[command]
pub async fn mark_email_read(id: i64, state: State<'_, AppState>) -> AppResult<()> {
    let db = state.db.lock().await;
    db.mark_email_read(id, true).await?;
    let unread = db.get_unread_count().await?;
    state.notifier.update_unread_count(unread.total);
    Ok(())
}

#[command]
pub async fn mark_email_unread(id: i64, state: State<'_, AppState>) -> AppResult<()> {
    let db = state.db.lock().await;
    db.mark_email_read(id, false).await?;
    let unread = db.get_unread_count().await?;
    state.notifier.update_unread_count(unread.total);
    Ok(())
}

#[command]
pub async fn add_email_tag(
    email_id: i64,
    tag_id: i64,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state.db.lock().await;
    db.add_email_tag(email_id, tag_id).await
}

#[command]
pub async fn remove_email_tag(
    email_id: i64,
    tag_id: i64,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state.db.lock().await;
    db.remove_email_tag(email_id, tag_id).await
}

#[command]
pub async fn get_tags(state: State<'_, AppState>) -> AppResult<Vec<Tag>> {
    let db = state.db.lock().await;
    db.get_tags().await
}

#[command]
pub async fn get_unread_count(state: State<'_, AppState>) -> AppResult<UnreadCount> {
    let db = state.db.lock().await;
    db.get_unread_count().await
}

#[command]
pub async fn get_sync_status(sync_manager: State<'_, SyncManager>) -> AppResult<SyncStatus> {
    Ok(sync_manager.get_status().await)
}

#[command]
pub async fn trigger_sync(
    account_id: Option<i64>,
    app_state: State<'_, AppState>,
    sync_manager: State<'_, SyncManager>,
) -> AppResult<Vec<SyncResult>> {
    // 创建一个 Arc<Mutex<AppState>> 供同步管理器使用
    let shared_state = std::sync::Arc::new(tokio::sync::Mutex::new(AppState {
        db: app_state.db.clone(),
        notifier: app_state.notifier.clone(),
    }));
    
    sync_manager.trigger_sync(account_id, shared_state).await
}
