import { invoke } from '@tauri-apps/api/tauri'
import type {
  Account,
  AccountInput,
  EmailListResult,
  EmailDetail,
  SyncResult,
  Tag,
  UnreadCount,
  SyncStatus
} from '@/types'

export const api = {
  addAccount(account: AccountInput): Promise<Account> {
    return invoke('add_account', { account })
  },

  updateAccount(id: number, account: AccountInput): Promise<Account> {
    return invoke('update_account', { id, account })
  },

  deleteAccount(id: number): Promise<void> {
    return invoke('delete_account', { id })
  },

  listAccounts(): Promise<Account[]> {
    return invoke('list_accounts')
  },

  syncEmails(accountId?: number): Promise<SyncResult[]> {
    return invoke('sync_emails', { accountId })
  },

  getEmails(
    accountId?: number,
    tagId?: number,
    limit = 50,
    offset = 0
  ): Promise<EmailListResult> {
    return invoke('get_emails', { accountId, tagId, limit, offset })
  },

  getEmailDetail(id: number): Promise<EmailDetail> {
    return invoke('get_email_detail', { id })
  },

  searchEmails(
    query: string,
    limit = 50,
    offset = 0
  ): Promise<EmailListResult> {
    return invoke('search_emails', { query, limit, offset })
  },

  markEmailRead(id: number): Promise<void> {
    return invoke('mark_email_read', { id })
  },

  markEmailUnread(id: number): Promise<void> {
    return invoke('mark_email_unread', { id })
  },

  addEmailTag(emailId: number, tagId: number): Promise<void> {
    return invoke('add_email_tag', { emailId, tagId })
  },

  removeEmailTag(emailId: number, tagId: number): Promise<void> {
    return invoke('remove_email_tag', { emailId, tagId })
  },

  getTags(): Promise<Tag[]> {
    return invoke('get_tags')
  },

  getUnreadCount(): Promise<UnreadCount> {
    return invoke('get_unread_count')
  },

  getSyncStatus(): Promise<SyncStatus> {
    return invoke('get_sync_status')
  },

  triggerSync(accountId?: number): Promise<SyncResult[]> {
    return invoke('trigger_sync', { accountId })
  }
}
