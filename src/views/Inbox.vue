<template>
  <div class="inbox-container">
    <div class="email-list-panel">
      <div class="search-bar">
        <input
          v-model="searchInput"
          type="text"
          class="search-input"
          placeholder="🔍 搜索邮件（主题、发件人、正文）"
          @keyup.enter="doSearch"
        />
        <button class="search-btn" @click="doSearch">搜索</button>
      </div>

      <div class="list-header">
        <span class="list-title">{{ getListTitle() }}</span>
        <span class="list-count">共 {{ store.totalEmails }} 封</span>
      </div>

      <div class="email-list" v-loading="store.loading">
        <div
          v-for="email in store.emails"
          :key="email.id"
          class="email-item"
          :class="{ active: store.currentEmail?.id === email.id, unread: !email.is_read }"
          @click="openEmail(email)"
        >
          <div class="email-item-header">
            <span class="email-sender" :title="email.sender_email">
              {{ email.sender_name || email.sender_email }}
            </span>
            <span class="email-date">{{ formatDate(email.date) }}</span>
          </div>
          <div class="email-subject">{{ email.subject || '(无主题)' }}</div>
          <div class="email-preview">{{ getPreview(email.body_text) }}</div>
          <div class="email-tags" v-if="email.tags.length > 0">
            <span
              v-for="tag in email.tags"
              :key="tag.id"
              class="email-tag"
              :style="{ backgroundColor: tag.color + '20', color: tag.color, borderColor: tag.color }"
            >
              {{ tag.name }}
            </span>
          </div>
        </div>

        <div v-if="!store.loading && store.emails.length === 0" class="empty-state">
          <div class="empty-icon">📭</div>
          <div class="empty-text">暂无邮件</div>
        </div>
      </div>
    </div>

    <div class="email-detail-panel" v-if="store.currentEmail">
      <div class="detail-header">
        <h2 class="detail-subject">{{ store.currentEmail.subject || '(无主题)' }}</h2>
        <div class="detail-actions">
          <button @click="toggleReadStatus">
            {{ store.currentEmail.is_read ? '⭕ 标记未读' : '✅ 标记已读' }}
          </button>
          <button @click="syncCurrent">🔄 同步</button>
          <button @click="closeDetail" class="close-btn">✕</button>
        </div>
      </div>

      <div class="detail-meta">
        <div class="meta-row">
          <span class="meta-label">发件人:</span>
          <span class="meta-value">
            {{ store.currentEmail.sender_name }}
            &lt;{{ store.currentEmail.sender_email }}&gt;
          </span>
        </div>
        <div class="meta-row">
          <span class="meta-label">收件人:</span>
          <span class="meta-value">{{ store.currentEmail.recipients }}</span>
        </div>
        <div class="meta-row">
          <span class="meta-label">日期:</span>
          <span class="meta-value">{{ formatFullDate(store.currentEmail.date) }}</span>
        </div>
        <div class="meta-row" v-if="store.currentEmail.tags.length > 0">
          <span class="meta-label">标签:</span>
          <div class="meta-tags">
            <span
              v-for="tag in store.currentEmail.tags"
              :key="tag.id"
              class="detail-tag"
              :style="{ backgroundColor: tag.color + '20', color: tag.color, borderColor: tag.color }"
            >
              {{ tag.name }}
              <button class="remove-tag" @click.stop="removeTag(tag.id)">×</button>
            </span>
            <div class="add-tag-wrapper">
              <button class="add-tag-btn" @click.stop="showTagPicker = !showTagPicker">+ 添加标签</button>
              <div v-if="showTagPicker" class="tag-picker">
                <button
                  v-for="tag in availableTags"
                  :key="tag.id"
                  class="tag-picker-item"
                  :style="{ borderLeftColor: tag.color }"
                  @click.stop="addTag(tag.id)"
                >
                  {{ tag.name }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="detail-attachments" v-if="store.currentEmail.attachments.length > 0">
        <div class="attachments-title">📎 附件 ({{ store.currentEmail.attachments.length }})</div>
        <div class="attachments-list">
          <div
            v-for="att in store.currentEmail.attachments"
            :key="att.id"
            class="attachment-item"
          >
            <span class="attachment-icon">📄</span>
            <span class="attachment-name">{{ att.filename }}</span>
            <span class="attachment-size">({{ formatSize(att.size) }})</span>
          </div>
        </div>
      </div>

      <div class="detail-body">
        <div v-if="store.currentEmail.body_html" v-html="store.currentEmail.body_html" class="html-body"></div>
        <pre v-else class="text-body">{{ store.currentEmail.body_text }}</pre>
      </div>
    </div>

    <div class="email-detail-panel empty-detail" v-else>
      <div class="empty-detail-content">
        <div class="empty-detail-icon">📧</div>
        <div class="empty-detail-text">选择一封邮件查看详情</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import dayjs from 'dayjs'
import { useMailStore } from '@/stores/mail'
import type { EmailWithTags } from '@/types'
import { shell } from '@tauri-apps/api'

const store = useMailStore()
const searchInput = ref('')
const showTagPicker = ref(false)

const availableTags = computed(() => {
  if (!store.currentEmail) return []
  const currentTagIds = new Set(store.currentEmail.tags.map(t => t.id))
  return store.tags.filter(t => !currentTagIds.has(t.id))
})

onMounted(() => {
  if (store.emails.length === 0) {
    store.loadEmails()
  }
})

function getListTitle(): string {
  if (store.searchQuery.trim()) {
    return `搜索: "${store.searchQuery}"`
  }
  if (store.selectedTagId) {
    const tag = store.tags.find(t => t.id === store.selectedTagId)
    return tag ? tag.name : '邮件列表'
  }
  if (store.selectedAccountId) {
    const account = store.accounts.find(a => a.id === store.selectedAccountId)
    return account ? account.name : '邮件列表'
  }
  return '全部邮件'
}

function formatDate(dateStr: string): string {
  const date = dayjs(dateStr)
  if (date.isToday()) {
    return date.format('HH:mm')
  }
  if (date.isSame(dayjs(), 'year')) {
    return date.format('MM-DD')
  }
  return date.format('YYYY-MM-DD')
}

function formatFullDate(dateStr: string): string {
  return dayjs(dateStr).format('YYYY-MM-DD HH:mm:ss')
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function getPreview(text: string): string {
  const clean = text.replace(/\s+/g, ' ').trim()
  return clean.length > 100 ? clean.slice(0, 100) + '...' : clean
}

async function openEmail(email: EmailWithTags) {
  await store.loadEmailDetail(email.id)
  showTagPicker.value = false
}

function closeDetail() {
  store.clearCurrentEmail()
}

async function toggleReadStatus() {
  if (!store.currentEmail) return
  if (store.currentEmail.is_read) {
    await store.markAsUnread(store.currentEmail.id)
  } else {
    await store.markAsRead(store.currentEmail.id)
  }
}

function syncCurrent() {
  if (store.selectedAccountId) {
    store.syncEmails(store.selectedAccountId)
  } else {
    store.syncEmails()
  }
}

function doSearch() {
  store.setSearchQuery(searchInput.value)
  store.clearCurrentEmail()
  store.loadEmails()
}

async function addTag(tagId: number) {
  if (!store.currentEmail) return
  await store.addTagToEmail(store.currentEmail.id, tagId)
  showTagPicker.value = false
}

async function removeTag(tagId: number) {
  if (!store.currentEmail) return
  await store.removeTagFromEmail(store.currentEmail.id, tagId)
}
</script>

<style scoped>
.inbox-container {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.email-list-panel {
  width: var(--email-list-width);
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-right: 1px solid var(--border);
}

.search-bar {
  display: flex;
  padding: 12px;
  gap: 8px;
  border-bottom: 1px solid var(--border);
}

.search-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  outline: none;
  transition: border-color 0.15s;
}

.search-input:focus {
  border-color: var(--primary);
}

.search-btn {
  padding: 8px 16px;
  background: var(--primary);
  color: white;
  border-radius: 6px;
  font-weight: 500;
  transition: background-color 0.15s;
}

.search-btn:hover {
  background: var(--primary-dark);
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.list-title {
  font-weight: 600;
  color: var(--text-primary);
}

.list-count {
  font-size: 12px;
  color: var(--text-secondary);
}

.email-list {
  flex: 1;
  overflow-y: auto;
}

.email-item {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background-color 0.15s;
}

.email-item:hover {
  background-color: var(--background);
}

.email-item.active {
  background-color: rgba(25, 118, 210, 0.1);
}

.email-item.unread {
  background-color: rgba(25, 118, 210, 0.05);
}

.email-item.unread .email-sender,
.email-item.unread .email-subject {
  font-weight: 600;
}

.email-item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.email-sender {
  font-size: 14px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}

.email-date {
  font-size: 12px;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.email-subject {
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.email-preview {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.email-tags {
  display: flex;
  gap: 4px;
  margin-top: 8px;
  flex-wrap: wrap;
}

.email-tag {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-secondary);
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.empty-text {
  font-size: 14px;
}

.email-detail-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  overflow: hidden;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 20px;
  border-bottom: 1px solid var(--border);
  gap: 16px;
}

.detail-subject {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
  flex: 1;
}

.detail-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.detail-actions button {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-secondary);
  transition: all 0.15s;
  font-size: 13px;
}

.detail-actions button:hover {
  background: var(--background);
  color: var(--text-primary);
}

.close-btn {
  font-size: 18px;
  padding: 4px 10px !important;
}

.detail-meta {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--background);
}

.meta-row {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;
  font-size: 13px;
}

.meta-row:last-child {
  margin-bottom: 0;
}

.meta-label {
  color: var(--text-secondary);
  min-width: 60px;
  flex-shrink: 0;
}

.meta-value {
  color: var(--text-primary);
  flex: 1;
}

.meta-tags {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  align-items: center;
}

.detail-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 4px;
  border: 1px solid;
}

.remove-tag {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0;
  opacity: 0.6;
}

.remove-tag:hover {
  opacity: 1;
}

.add-tag-wrapper {
  position: relative;
}

.add-tag-btn {
  font-size: 12px;
  padding: 3px 10px;
  border: 1px dashed var(--border);
  border-radius: 4px;
  color: var(--text-secondary);
  background: transparent;
}

.add-tag-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}

.tag-picker {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
  min-width: 120px;
}

.tag-picker-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 8px 12px;
  border-left: 3px solid;
  font-size: 13px;
  transition: background-color 0.15s;
}

.tag-picker-item:hover {
  background: var(--background);
}

.detail-attachments {
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
}

.attachments-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.attachments-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.attachment-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--background);
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.attachment-item:hover {
  background: var(--border);
}

.attachment-icon {
  font-size: 16px;
}

.attachment-name {
  flex: 1;
  color: var(--primary);
}

.attachment-size {
  color: var(--text-secondary);
  font-size: 12px;
}

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.html-body {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
}

.html-body :deep(img) {
  max-width: 100%;
  height: auto;
}

.html-body :deep(a) {
  color: var(--primary);
}

.text-body {
  font-family: inherit;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: var(--text-primary);
}

.empty-detail {
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-detail-content {
  text-align: center;
  color: var(--text-secondary);
}

.empty-detail-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.empty-detail-text {
  font-size: 16px;
}
</style>
