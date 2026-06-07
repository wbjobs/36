export interface Account {
  id: number
  name: string
  email: string
  imap_server: string
  imap_port: number
  username: string
  password: string
  use_ssl: boolean
  created_at: string
  updated_at: string
}

export interface AccountInput {
  name: string
  email: string
  imap_server: string
  imap_port: number
  username: string
  password: string
  use_ssl: boolean
}

export interface Email {
  id: number
  account_id: number
  message_id: string
  subject: string
  sender_name: string
  sender_email: string
  recipients: string
  date: string
  body_text: string
  body_html: string
  is_read: boolean
  is_flagged: boolean
  uid: number
  created_at: string
}

export interface Attachment {
  id: number
  email_id: number
  filename: string
  content_type: string
  size: number
  content_id?: string
  file_path: string
}

export interface Tag {
  id: number
  name: string
  color: string
  is_system: boolean
}

export interface EmailWithTags {
  id: number
  account_id: number
  message_id: string
  subject: string
  sender_name: string
  sender_email: string
  recipients: string
  date: string
  body_text: string
  body_html: string
  is_read: boolean
  is_flagged: boolean
  uid: number
  created_at: string
  tags: Tag[]
}

export interface EmailDetail {
  id: number
  account_id: number
  message_id: string
  subject: string
  sender_name: string
  sender_email: string
  recipients: string
  date: string
  body_text: string
  body_html: string
  is_read: boolean
  is_flagged: boolean
  uid: number
  created_at: string
  attachments: Attachment[]
  tags: Tag[]
}

export interface EmailListResult {
  total: number
  emails: EmailWithTags[]
}

export interface SyncResult {
  account_id: number
  account_name: string
  new_emails: number
  updated_emails: number
}

export interface UnreadCount {
  total: number
  by_account: Array<[number, string, number]>
}

export type SyncStatus =
  | { status: 'Idle' }
  | { status: 'Syncing'; data: { account_id: number | null } }
  | { status: 'Completed'; data: { results: SyncResult[] } }
  | { status: 'Error'; data: { message: string } }
