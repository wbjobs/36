import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api'
import type { Account, AccountInput, EmailWithTags, EmailDetail, Tag, SyncResult, UnreadCount } from '@/types'

export const useMailStore = defineStore('mail', () => {
  const accounts = ref<Account[]>([])
  const tags = ref<Tag[]>([])
  const emails = ref<EmailWithTags[]>([])
  const totalEmails = ref(0)
  const currentEmail = ref<EmailDetail | null>(null)
  const selectedAccountId = ref<number | undefined>(undefined)
  const selectedTagId = ref<number | undefined>(undefined)
  const searchQuery = ref('')
  const unreadCount = ref<UnreadCount>({ total: 0, by_account: [] })
  const loading = ref(false)
  const syncing = ref(false)

  const hasAccounts = computed(() => accounts.value.length > 0)

  async function loadAccounts() {
    accounts.value = await api.listAccounts()
    await loadUnreadCount()
  }

  async function addAccount(input: AccountInput) {
    const account = await api.addAccount(input)
    accounts.value.push(account)
    return account
  }

  async function updateAccount(id: number, input: AccountInput) {
    const account = await api.updateAccount(id, input)
    const idx = accounts.value.findIndex(a => a.id === id)
    if (idx !== -1) {
      accounts.value[idx] = account
    }
    return account
  }

  async function deleteAccount(id: number) {
    await api.deleteAccount(id)
    accounts.value = accounts.value.filter(a => a.id !== id)
    if (selectedAccountId.value === id) {
      selectedAccountId.value = undefined
    }
  }

  async function loadTags() {
    tags.value = await api.getTags()
  }

  async function loadEmails(limit = 50, offset = 0) {
    loading.value = true
    try {
      if (searchQuery.value.trim()) {
        const result = await api.searchEmails(searchQuery.value.trim(), limit, offset)
        emails.value = result.emails
        totalEmails.value = result.total
      } else {
        const result = await api.getEmails(selectedAccountId.value, selectedTagId.value, limit, offset)
        emails.value = result.emails
        totalEmails.value = result.total
      }
    } finally {
      loading.value = false
    }
  }

  async function loadEmailDetail(id: number) {
    currentEmail.value = await api.getEmailDetail(id)
    const idx = emails.value.findIndex(e => e.id === id)
    if (idx !== -1) {
      emails.value[idx].is_read = true
    }
    await loadUnreadCount()
  }

  async function syncEmails(accountId?: number): Promise<SyncResult[]> {
    syncing.value = true
    try {
      const results = await api.syncEmails(accountId)
      await loadEmails()
      await loadUnreadCount()
      return results
    } finally {
      syncing.value = false
    }
  }

  async function markAsRead(id: number) {
    await api.markEmailRead(id)
    const idx = emails.value.findIndex(e => e.id === id)
    if (idx !== -1) {
      emails.value[idx].is_read = true
    }
    if (currentEmail.value?.id === id) {
      currentEmail.value.is_read = true
    }
    await loadUnreadCount()
  }

  async function markAsUnread(id: number) {
    await api.markEmailUnread(id)
    const idx = emails.value.findIndex(e => e.id === id)
    if (idx !== -1) {
      emails.value[idx].is_read = false
    }
    if (currentEmail.value?.id === id) {
      currentEmail.value.is_read = false
    }
    await loadUnreadCount()
  }

  async function addTagToEmail(emailId: number, tagId: number) {
    await api.addEmailTag(emailId, tagId)
    const idx = emails.value.findIndex(e => e.id === emailId)
    if (idx !== -1) {
      const tag = tags.value.find(t => t.id === tagId)
      if (tag && !emails.value[idx].tags.find(t => t.id === tagId)) {
        emails.value[idx].tags.push(tag)
      }
    }
    if (currentEmail.value?.id === emailId) {
      const tag = tags.value.find(t => t.id === tagId)
      if (tag && !currentEmail.value.tags.find(t => t.id === tagId)) {
        currentEmail.value.tags.push(tag)
      }
    }
  }

  async function removeTagFromEmail(emailId: number, tagId: number) {
    await api.removeEmailTag(emailId, tagId)
    const idx = emails.value.findIndex(e => e.id === emailId)
    if (idx !== -1) {
      emails.value[idx].tags = emails.value[idx].tags.filter(t => t.id !== tagId)
    }
    if (currentEmail.value?.id === emailId) {
      currentEmail.value.tags = currentEmail.value.tags.filter(t => t.id !== tagId)
    }
  }

  async function loadUnreadCount() {
    unreadCount.value = await api.getUnreadCount()
  }

  function setSearchQuery(query: string) {
    searchQuery.value = query
  }

  function setSelectedAccount(id: number | undefined) {
    selectedAccountId.value = id
    selectedTagId.value = undefined
  }

  function setSelectedTag(id: number | undefined) {
    selectedTagId.value = id
  }

  function clearCurrentEmail() {
    currentEmail.value = null
  }

  return {
    accounts,
    tags,
    emails,
    totalEmails,
    currentEmail,
    selectedAccountId,
    selectedTagId,
    searchQuery,
    unreadCount,
    loading,
    syncing,
    hasAccounts,
    loadAccounts,
    addAccount,
    updateAccount,
    deleteAccount,
    loadTags,
    loadEmails,
    loadEmailDetail,
    syncEmails,
    markAsRead,
    markAsUnread,
    addTagToEmail,
    removeTagFromEmail,
    loadUnreadCount,
    setSearchQuery,
    setSelectedAccount,
    setSelectedTag,
    clearCurrentEmail
  }
})
