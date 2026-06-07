use crate::{AppError, AppResult};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use rusqlite::{params, OptionalExtension};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::*;

/// 中文文本预处理：在中文字符之间插入空格，使 FTS5 unicode61 分词器能够正确分词
/// 例如："搜索功能" -> "搜 索 功 能"
pub fn tokenize_chinese(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let chars: Vec<char> = text.chars().collect();
    
    for i in 0..chars.len() {
        let c = chars[i];
        result.push(c);
        
        // 检查是否是中文字符（CJK 统一表意文字）
        let is_cjk = matches!(c as u32,
            0x4E00..=0x9FFF |   // CJK 统一表意文字
            0x3400..=0x4DBF |   // CJK 扩展 A
            0x20000..=0x2A6DF | // CJK 扩展 B
            0x2A700..=0x2B73F | // CJK 扩展 C
            0x2B740..=0x2B81F | // CJK 扩展 D
            0xF900..=0xFAFF     // CJK 兼容表意文字
        );
        
        // 如果当前字符是中文字符，且下一个字符也是中文字符，插入空格
        if is_cjk && i + 1 < chars.len() {
            let next_c = chars[i + 1];
            let next_is_cjk = matches!(next_c as u32,
                0x4E00..=0x9FFF |
                0x3400..=0x4DBF |
                0x20000..=0x2A6DF |
                0x2A700..=0x2B73F |
                0x2B740..=0x2B81F |
                0xF900..=0xFAFF
            );
            if next_is_cjk {
                result.push(' ');
            }
        }
    }
    
    result
}

static DB_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("mail-client");
    std::fs::create_dir_all(&path).ok();
    path.push("mail.db");
    path
});

pub struct Database {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl Database {
    pub async fn new() -> AppResult<Self> {
        let path = DB_PATH.clone();
        let conn = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(path)?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            Ok::<_, rusqlite::Error>(conn)
        })
        .await
        .map_err(|e| AppError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))??;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables().await?;
        db.init_system_tags().await?;
        db.run_migrations().await?;
        Ok(db)
    }

    /// 数据库迁移：重建 FTS5 索引以支持中文搜索
    async fn run_migrations(&self) -> AppResult<()> {
        let conn = self.conn.lock().await;
        
        // 检查数据库版本
        let current_version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM db_version",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        const TARGET_VERSION: i64 = 1;
        
        if current_version >= TARGET_VERSION {
            return Ok(());
        }

        // 迁移到版本 1：重建 FTS5 索引以支持中文搜索
        if current_version < 1 {
            log::info!("Running database migration to version 1: rebuild FTS5 index for Chinese search support");
            
            // 清空 FTS5 表
            conn.execute("DELETE FROM emails_fts", [])?;
            
            // 为所有现有邮件重建 FTS5 索引
            let mut stmt = conn.prepare(
                "SELECT id, subject, sender_name, sender_email, body_text FROM emails"
            )?;
            
            let email_rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            
            let mut count = 0;
            for email_result in email_rows {
                let (id, subject, sender_name, sender_email, body_text) = email_result?;
                
                let subject_fts = tokenize_chinese(&subject);
                let sender_name_fts = tokenize_chinese(&sender_name);
                let sender_email_fts = tokenize_chinese(&sender_email);
                let body_text_fts = tokenize_chinese(&body_text);
                
                conn.execute(
                    "INSERT INTO emails_fts (rowid, subject, sender_name, sender_email, body_text)
                     VALUES (?, ?, ?, ?, ?)",
                    params![id, subject_fts, sender_name_fts, sender_email_fts, body_text_fts],
                )?;
                count += 1;
            }
            
            // 更新版本号
            conn.execute(
                "INSERT INTO db_version (version, applied_at) VALUES (?, ?)",
                params![TARGET_VERSION, Utc::now().to_rfc3339()],
            )?;
            
            log::info!("Migration complete: rebuilt FTS5 index for {} emails", count);
        }

        Ok(())
    }

    async fn init_tables(&self) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS db_version (
                version INTEGER PRIMARY KEY,
                applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                imap_server TEXT NOT NULL,
                imap_port INTEGER NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                use_ssl INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(email)
            );

            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL,
                message_id TEXT NOT NULL,
                subject TEXT NOT NULL,
                sender_name TEXT,
                sender_email TEXT NOT NULL,
                recipients TEXT NOT NULL,
                date DATETIME NOT NULL,
                body_text TEXT NOT NULL DEFAULT '',
                body_html TEXT NOT NULL DEFAULT '',
                is_read INTEGER NOT NULL DEFAULT 0,
                is_flagged INTEGER NOT NULL DEFAULT 0,
                uid INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
                UNIQUE(account_id, message_id)
            );

            CREATE INDEX IF NOT EXISTS idx_emails_account_id ON emails(account_id);
            CREATE INDEX IF NOT EXISTS idx_emails_date ON emails(date DESC);
            CREATE INDEX IF NOT EXISTS idx_emails_is_read ON emails(is_read);

            CREATE TABLE IF NOT EXISTS attachments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email_id INTEGER NOT NULL,
                filename TEXT NOT NULL,
                content_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                content_id TEXT,
                file_path TEXT NOT NULL,
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_attachments_email_id ON attachments(email_id);

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT NOT NULL DEFAULT '#607D8B',
                is_system INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS email_tags (
                email_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (email_id, tag_id),
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_email_tags_tag_id ON email_tags(tag_id);

            CREATE VIRTUAL TABLE IF NOT EXISTS emails_fts USING fts5(
                subject,
                sender_name,
                sender_email,
                body_text,
                tokenize='unicode61'
            );
            "#,
        )?;
        Ok(())
    }

    async fn init_system_tags(&self) -> AppResult<()> {
        let system_tags = [
            ("工作", "#4CAF50", true),
            ("个人", "#2196F3", true),
            ("订阅", "#9C27B0", true),
            ("垃圾", "#F44336", true),
        ];

        let conn = self.conn.lock().await;
        for (name, color, is_system) in system_tags.iter() {
            conn.execute(
                "INSERT OR IGNORE INTO tags (name, color, is_system) VALUES (?, ?, ?)",
                params![name, color, if *is_system { 1 } else { 0 }],
            )?;
        }
        Ok(())
    }

    pub async fn add_account(&self, account: &AccountInput) -> AppResult<Account> {
        let conn = self.conn.lock().await;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
        if count >= 5 {
            return Err(AppError::AccountLimit);
        }

        let now = Utc::now();
        conn.execute(
            "INSERT INTO accounts (name, email, imap_server, imap_port, username, password, use_ssl, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                account.name,
                account.email,
                account.imap_server,
                account.imap_port,
                account.username,
                account.password,
                if account.use_ssl { 1 } else { 0 },
                now.to_rfc3339(),
                now.to_rfc3339()
            ],
        )?;

        let id = conn.last_insert_rowid();
        self.get_account(id).await
    }

    pub async fn get_account(&self, id: i64) -> AppResult<Account> {
        let conn = self.conn.lock().await;
        conn.query_row(
            "SELECT id, name, email, imap_server, imap_port, username, password, use_ssl, created_at, updated_at FROM accounts WHERE id = ?",
            params![id],
            |row| {
                let created_at: String = row.get(8)?;
                let updated_at: String = row.get(9)?;
                Ok(Account {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    imap_server: row.get(3)?,
                    imap_port: row.get(4)?,
                    username: row.get(5)?,
                    password: row.get(6)?,
                    use_ssl: row.get::<_, i64>(7)? == 1,
                    created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::AccountNotFound,
            e => AppError::Database(e),
        })
    }

    pub async fn list_accounts(&self) -> AppResult<Vec<Account>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, email, imap_server, imap_port, username, password, use_ssl, created_at, updated_at FROM accounts ORDER BY created_at ASC",
        )?;

        let accounts = stmt.query_map([], |row| {
            let created_at: String = row.get(8)?;
            let updated_at: String = row.get(9)?;
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                imap_server: row.get(3)?,
                imap_port: row.get(4)?,
                username: row.get(5)?,
                password: row.get(6)?,
                use_ssl: row.get::<_, i64>(7)? == 1,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
            })
        })?;

        Ok(accounts.collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn update_account(&self, id: i64, account: &AccountInput) -> AppResult<Account> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        let result = conn.execute(
            "UPDATE accounts SET name = ?, email = ?, imap_server = ?, imap_port = ?, username = ?, password = ?, use_ssl = ?, updated_at = ? WHERE id = ?",
            params![
                account.name,
                account.email,
                account.imap_server,
                account.imap_port,
                account.username,
                account.password,
                if account.use_ssl { 1 } else { 0 },
                now.to_rfc3339(),
                id
            ],
        )?;

        if result == 0 {
            return Err(AppError::AccountNotFound);
        }

        self.get_account(id).await
    }

    pub async fn delete_account(&self, id: i64) -> AppResult<()> {
        let conn = self.conn.lock().await;
        
        // 先删除相关的 FTS5 记录
        conn.execute(
            "DELETE FROM emails_fts WHERE rowid IN (SELECT id FROM emails WHERE account_id = ?)",
            params![id],
        )?;
        
        let result = conn.execute("DELETE FROM accounts WHERE id = ?", params![id])?;
        if result == 0 {
            return Err(AppError::AccountNotFound);
        }
        Ok(())
    }

    pub async fn insert_email(&self, account_id: i64, email: &Email, attachments: &[Attachment]) -> AppResult<i64> {
        let conn = self.conn.lock().await;

        let result = conn.execute(
            "INSERT OR IGNORE INTO emails (account_id, message_id, subject, sender_name, sender_email, recipients, date, body_text, body_html, is_read, is_flagged, uid)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                account_id,
                email.message_id,
                email.subject,
                email.sender_name,
                email.sender_email,
                email.recipients,
                email.date.to_rfc3339(),
                email.body_text,
                email.body_html,
                if email.is_read { 1 } else { 0 },
                if email.is_flagged { 1 } else { 0 },
                email.uid,
            ],
        )?;

        if result == 0 {
            let email_id: i64 = conn.query_row(
                "SELECT id FROM emails WHERE account_id = ? AND message_id = ?",
                params![account_id, email.message_id],
                |row| row.get(0),
            )?;
            return Ok(email_id);
        }

        let email_id = conn.last_insert_rowid();

        for attachment in attachments {
            conn.execute(
                "INSERT INTO attachments (email_id, filename, content_type, size, content_id, file_path) VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    email_id,
                    attachment.filename,
                    attachment.content_type,
                    attachment.size,
                    attachment.content_id,
                    attachment.file_path,
                ],
            )?;
        }

        // 手动更新 FTS5 表，使用中文预处理后的文本
        let subject_fts = tokenize_chinese(&email.subject);
        let sender_name_fts = tokenize_chinese(&email.sender_name);
        let sender_email_fts = tokenize_chinese(&email.sender_email);
        let body_text_fts = tokenize_chinese(&email.body_text);
        
        conn.execute(
            "INSERT INTO emails_fts (rowid, subject, sender_name, sender_email, body_text)
             VALUES (?, ?, ?, ?, ?)",
            params![
                email_id,
                subject_fts,
                sender_name_fts,
                sender_email_fts,
                body_text_fts,
            ],
        )?;

        Ok(email_id)
    }

    pub async fn get_emails(&self, account_id: Option<i64>, tag_id: Option<i64>, limit: i64, offset: i64) -> AppResult<EmailListResult> {
        let conn = self.conn.lock().await;

        let (where_clause, params_vec) = match (account_id, tag_id) {
            (Some(acc_id), Some(t_id)) => (
                "WHERE e.account_id = ?1 AND et.tag_id = ?2".to_string(),
                vec![acc_id, t_id],
            ),
            (Some(acc_id), None) => (
                "WHERE e.account_id = ?1".to_string(),
                vec![acc_id],
            ),
            (None, Some(t_id)) => (
                "WHERE et.tag_id = ?1".to_string(),
                vec![t_id],
            ),
            (None, None) => ("".to_string(), vec![]),
        };

        let join_clause = if tag_id.is_some() {
            "INNER JOIN email_tags et ON e.id = et.email_id"
        } else {
            "LEFT JOIN email_tags et ON e.id = et.email_id"
        };

        let count_query = format!(
            "SELECT COUNT(DISTINCT e.id) FROM emails e {} {}",
            join_clause, where_clause
        );
        let total: i64 = conn.query_row(&count_query, rusqlite::params_from_iter(params_vec.iter()), |row| row.get(0))?;

        let limit_param_pos = params_vec.len() + 1;
        let offset_param_pos = params_vec.len() + 2;
        let query = format!(
            "SELECT DISTINCT e.id, e.account_id, e.message_id, e.subject, e.sender_name, e.sender_email, 
                    e.recipients, e.date, e.body_text, e.body_html, e.is_read, e.is_flagged, e.uid, e.created_at
             FROM emails e {} {}
             ORDER BY e.date DESC LIMIT ?{} OFFSET ?{}",
            join_clause, where_clause, limit_param_pos, offset_param_pos
        );

        let mut params_for_query = params_vec.clone();
        params_for_query.push(limit);
        params_for_query.push(offset);

        let mut stmt = conn.prepare(&query)?;
        let email_rows = stmt.query_map(rusqlite::params_from_iter(params_for_query.iter()), |row| {
            let date: String = row.get(7)?;
            let created_at: String = row.get(13)?;
            Ok(Email {
                id: row.get(0)?,
                account_id: row.get(1)?,
                message_id: row.get(2)?,
                subject: row.get(3)?,
                sender_name: row.get(4)?,
                sender_email: row.get(5)?,
                recipients: row.get(6)?,
                date: DateTime::parse_from_rfc3339(&date)?.with_timezone(&Utc),
                body_text: row.get(8)?,
                body_html: row.get(9)?,
                is_read: row.get::<_, i64>(10)? == 1,
                is_flagged: row.get::<_, i64>(11)? == 1,
                uid: row.get(12)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
            })
        })?;

        let mut emails: Vec<EmailWithTags> = Vec::new();
        for email_result in email_rows {
            let email = email_result?;
            let tags = self.get_tags_for_email(&conn, email.id)?;
            emails.push(EmailWithTags { email, tags });
        }

        Ok(EmailListResult { total, emails })
    }

    fn get_tags_for_email(&self, conn: &rusqlite::Connection, email_id: i64) -> AppResult<Vec<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color, t.is_system FROM tags t
             INNER JOIN email_tags et ON t.id = et.tag_id
             WHERE et.email_id = ?
             ORDER BY t.name ASC",
        )?;

        let tags = stmt.query_map(params![email_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                is_system: row.get::<_, i64>(3)? == 1,
            })
        })?;

        Ok(tags.collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn get_email_detail(&self, id: i64) -> AppResult<EmailDetail> {
        let conn = self.conn.lock().await;

        let email = conn.query_row(
            "SELECT id, account_id, message_id, subject, sender_name, sender_email, recipients, date, 
                    body_text, body_html, is_read, is_flagged, uid, created_at 
             FROM emails WHERE id = ?",
            params![id],
            |row| {
                let date: String = row.get(7)?;
                let created_at: String = row.get(13)?;
                Ok(Email {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    message_id: row.get(2)?,
                    subject: row.get(3)?,
                    sender_name: row.get(4)?,
                    sender_email: row.get(5)?,
                    recipients: row.get(6)?,
                    date: DateTime::parse_from_rfc3339(&date)?.with_timezone(&Utc),
                    body_text: row.get(8)?,
                    body_html: row.get(9)?,
                    is_read: row.get::<_, i64>(10)? == 1,
                    is_flagged: row.get::<_, i64>(11)? == 1,
                    uid: row.get(12)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::EmailNotFound,
            e => AppError::Database(e),
        })?;

        let attachments = {
            let mut stmt = conn.prepare(
                "SELECT id, email_id, filename, content_type, size, content_id, file_path FROM attachments WHERE email_id = ?",
            )?;
            let attachments = stmt.query_map(params![id], |row| {
                Ok(Attachment {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    filename: row.get(2)?,
                    content_type: row.get(3)?,
                    size: row.get(4)?,
                    content_id: row.get(5)?,
                    file_path: row.get(6)?,
                })
            })?;
            attachments.collect::<Result<Vec<_>, _>>()?
        };

        let tags = self.get_tags_for_email(&conn, id)?;

        Ok(EmailDetail { email, attachments, tags })
    }

    pub async fn search_emails(&self, query: &str, limit: i64, offset: i64) -> AppResult<EmailListResult> {
        let conn = self.conn.lock().await;

        // 对搜索查询进行中文预处理
        let processed_query = tokenize_chinese(query);

        // 构建 FTS5 搜索查询，对每个词使用前缀匹配
        let fts_query = processed_query
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{}\"*", s.replace("\"", "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");

        let total: i64 = if fts_query.is_empty() {
            0
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM emails_fts WHERE emails_fts MATCH ?",
                params![fts_query],
                |row| row.get(0),
            ).unwrap_or(0)
        };

        let query_sql = "SELECT e.id, e.account_id, e.message_id, e.subject, e.sender_name, e.sender_email, 
                         e.recipients, e.date, e.body_text, e.body_html, e.is_read, e.is_flagged, e.uid, e.created_at
                         FROM emails e
                         INNER JOIN emails_fts ON e.id = emails_fts.rowid
                         WHERE emails_fts MATCH ?
                         ORDER BY rank, e.date DESC LIMIT ? OFFSET ?";

        let mut stmt = conn.prepare(query_sql)?;
        let email_rows = stmt.query_map(params![fts_query, limit, offset], |row| {
            let date: String = row.get(7)?;
            let created_at: String = row.get(13)?;
            Ok(Email {
                id: row.get(0)?,
                account_id: row.get(1)?,
                message_id: row.get(2)?,
                subject: row.get(3)?,
                sender_name: row.get(4)?,
                sender_email: row.get(5)?,
                recipients: row.get(6)?,
                date: DateTime::parse_from_rfc3339(&date)?.with_timezone(&Utc),
                body_text: row.get(8)?,
                body_html: row.get(9)?,
                is_read: row.get::<_, i64>(10)? == 1,
                is_flagged: row.get::<_, i64>(11)? == 1,
                uid: row.get(12)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
            })
        })?;

        let mut emails: Vec<EmailWithTags> = Vec::new();
        for email_result in email_rows {
            let email = email_result?;
            let tags = self.get_tags_for_email(&conn, email.id)?;
            emails.push(EmailWithTags { email, tags });
        }

        Ok(EmailListResult { total, emails })
    }

    pub async fn mark_email_read(&self, id: i64, read: bool) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let result = conn.execute(
            "UPDATE emails SET is_read = ? WHERE id = ?",
            params![if read { 1 } else { 0 }, id],
        )?;
        if result == 0 {
            return Err(AppError::EmailNotFound);
        }
        Ok(())
    }

    pub async fn add_email_tag(&self, email_id: i64, tag_id: i64) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.query_row("SELECT 1 FROM tags WHERE id = ?", params![tag_id], |_| Ok(()))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::TagNotFound,
                e => AppError::Database(e),
            })?;

        conn.query_row("SELECT 1 FROM emails WHERE id = ?", params![email_id], |_| Ok(()))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::EmailNotFound,
                e => AppError::Database(e),
            })?;

        conn.execute(
            "INSERT OR IGNORE INTO email_tags (email_id, tag_id) VALUES (?, ?)",
            params![email_id, tag_id],
        )?;
        Ok(())
    }

    pub async fn remove_email_tag(&self, email_id: i64, tag_id: i64) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "DELETE FROM email_tags WHERE email_id = ? AND tag_id = ?",
            params![email_id, tag_id],
        )?;
        Ok(())
    }

    pub async fn get_tags(&self) -> AppResult<Vec<Tag>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, color, is_system FROM tags ORDER BY is_system DESC, name ASC")?;
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                is_system: row.get::<_, i64>(3)? == 1,
            })
        })?;
        Ok(tags.collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn get_unread_count(&self) -> AppResult<UnreadCount> {
        let conn = self.conn.lock().await;

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM emails WHERE is_read = 0", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, COUNT(e.id) 
             FROM accounts a 
             LEFT JOIN emails e ON a.id = e.account_id AND e.is_read = 0 
             GROUP BY a.id, a.name",
        )?;

        let by_account = stmt.query_map([], |row| {
            let count: i64 = row.get(2)?;
            Ok((row.get(0)?, row.get(1)?, count))
        })?;

        Ok(UnreadCount {
            total,
            by_account: by_account.collect::<Result<Vec<_>, _>>()?,
        })
    }
}
