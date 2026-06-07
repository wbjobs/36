use crate::{AppError, AppResult};
use crate::models::*;
use chrono::{DateTime, Utc};
use imap::Session;
use mail_parser::{HeaderValue, Message};
use std::net::TcpStream;
use std::path::PathBuf;
use native_tls::TlsStream;

type ImapSession = Session<TlsStream<TcpStream>>;

pub struct ImapClient {
    account: Account,
    attachments_dir: PathBuf,
}

impl ImapClient {
    pub fn new(account: Account) -> Self {
        let mut attachments_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        attachments_dir.push("mail-client");
        attachments_dir.push("attachments");
        attachments_dir.push(account.id.to_string());
        std::fs::create_dir_all(&attachments_dir).ok();

        Self {
            account,
            attachments_dir,
        }
    }

    fn connect(&self) -> AppResult<ImapSession> {
        let tls = native_tls::TlsConnector::builder().build()?;
        let addr = format!("{}:{}", self.account.imap_server, self.account.imap_port);
        let tcp_stream = TcpStream::connect(&addr)?;
        let tls_stream = tls.connect(&self.account.imap_server, tcp_stream)?;
        let client = imap::Client::new(tls_stream);
        let session = client.login(&self.account.username, &self.account.password).map_err(|e| AppError::Imap(e.to_string()))?;
        Ok(session)
    }

    pub async fn fetch_emails(&self, since_uid: Option<u32>, limit: u32) -> AppResult<Vec<(Email, Vec<Attachment>)>> {
        let account = self.account.clone();
        let attachments_dir = self.attachments_dir.clone();
        
        let result = tokio::task::spawn_blocking(move || {
            let mut session = {
                let tls = native_tls::TlsConnector::builder().build()?;
                let addr = format!("{}:{}", account.imap_server, account.imap_port);
                let tcp_stream = TcpStream::connect(&addr)?;
                let tls_stream = tls.connect(&account.imap_server, tcp_stream)?;
                let client = imap::Client::new(tls_stream);
                client.login(&account.username, &account.password).map_err(|e| AppError::Imap(e.to_string()))?
            };

            session.select("INBOX").map_err(|e| AppError::Imap(e.to_string()))?;

            let uid_search = match since_uid {
                Some(uid) => format!("UID {}:*", uid + 1),
                None => "ALL".to_string(),
            };

            let uids = session.uid_search(uid_search).map_err(|e| AppError::Imap(e.to_string()))?;
            let mut uids_vec: Vec<u32> = uids.into_iter().collect();
            uids_vec.sort();
            if limit > 0 {
                uids_vec.truncate(limit as usize);
            }

            let mut result = Vec::new();
            for uid in uids_vec {
                let fetch = session.uid_fetch(format!("{}", uid), "(RFC822 UID FLAGS)").map_err(|e| AppError::Imap(e.to_string()))?;
                if let Some(message) = fetch.first() {
                    if let Some(body) = message.body() {
                        let flags: Vec<imap::types::Flag> = message.flags().cloned().collect();
                        match parse_email(uid, body, &flags, account.id, &attachments_dir) {
                            Ok(parsed) => result.push(parsed),
                            Err(e) => log::warn!("Failed to parse email {}: {}", uid, e),
                        }
                    }
                }
            }

            session.logout().map_err(|e| AppError::Imap(e.to_string()))?;
            Ok::<_, AppError>(result)
        }).await.map_err(|e| AppError::Imap(e.to_string()))??;

        Ok(result)
    }
}

fn parse_email(
    uid: u32,
    body: &[u8],
    flags: &[imap::types::Flag],
    account_id: i64,
    attachments_dir: &PathBuf,
) -> AppResult<(Email, Vec<Attachment>)> {
    let parsed = Message::parse(body)?;

    let message_id = parsed
        .get_message_id()
        .and_then(|v| v.as_text())
        .unwrap_or_else(|| format!("{}-{}", account_id, uid));

    let subject = parsed
        .get_subject()
        .unwrap_or_default()
        .to_string();

    let (sender_name, sender_email) = parsed
        .get_from()
        .and_then(|v| v.as_addr())
        .map(|addr| {
            (
                addr.get_name().unwrap_or_default().to_string(),
                addr.get_address().unwrap_or_default().to_string(),
            )
        })
        .unwrap_or_default();

    let recipients = parsed
        .get_to()
        .map(|to| {
            match to {
                HeaderValue::Address(addr) => {
                    addr.get_address().unwrap_or_default().to_string()
                }
                HeaderValue::AddressList(addrs) => {
                    addrs.iter()
                        .filter_map(|a| a.get_address())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                _ => String::new(),
            }
        })
        .unwrap_or_default();

    let date = parsed
        .get_date()
        .and_then(|d| DateTime::from_timestamp(d.to_timestamp(), 0))
        .unwrap_or_else(|| Utc::now());

    let body_text = parsed.get_text_body(0).unwrap_or_default().to_string();
    let body_html = parsed.get_html_body(0).unwrap_or_default().to_string();

    let is_read = flags.iter().any(|f| matches!(f, imap::types::Flag::Seen));
    let is_flagged = flags.iter().any(|f| matches!(f, imap::types::Flag::Flagged));

    let attachments = save_attachments(&parsed, attachments_dir)?;

    let email = Email {
        id: 0,
        account_id,
        message_id: message_id.to_string(),
        subject,
        sender_name,
        sender_email,
        recipients,
        date,
        body_text,
        body_html,
        is_read,
        is_flagged,
        uid,
        created_at: Utc::now(),
    };

    Ok((email, attachments))
}

fn save_attachments(message: &Message, attachments_dir: &PathBuf) -> AppResult<Vec<Attachment>> {
    let mut attachments = Vec::new();

    for part in message.get_attachments() {
        if let Some(filename) = part.get_attachment_name() {
            let content_type = part.get_content_type().map(|c| c.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());
            let content_id = part.get_content_id().map(|c| c.to_string());
            let body = part.get_body_raw().unwrap_or_default();
            let size = body.len() as i64;

            let file_name = format!("{}-{}", Utc::now().timestamp(), filename);
            let file_path = attachments_dir.join(&file_name);
            std::fs::write(&file_path, &body)?;

            attachments.push(Attachment {
                id: 0,
                email_id: 0,
                filename: filename.to_string(),
                content_type,
                size,
                content_id,
                file_path: file_path.to_string_lossy().to_string(),
            });
        }
    }

    Ok(attachments)
}
