<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1 class="app-title">📧 邮件客户端</h1>
    </div>

    <div class="sidebar-section">
      <div class="sidebar-section-header">
        <span class="section-title">标签</span>
      </div>
      <nav class="nav-list">
        <button
          class="nav-item"
          :class="{ active: store.selectedTagId === undefined && store.selectedAccountId === undefined }"
          @click="selectAll"
        >
          <span class="nav-icon">📥</span>
          <span class="nav-label">全部邮件</span>
          <span class="nav-badge" v-if="store.unreadCount.total > 0">
            {{ store.unreadCount.total }}
          </span>
        </button>
        <button
          v-for="tag in store.tags"
          :key="tag.id"
          class="nav-item"
          :class="{ active: store.selectedTagId === tag.id }"
          @click="selectTag(tag.id)"
        >
          <span class="tag-dot" :style="{ backgroundColor: tag.color }"></span>
          <span class="nav-label">{{ tag.name }}</span>
        </button>
      </nav>
    </div>

    <div class="sidebar-section" v-if="store.accounts.length > 0">
      <div class="sidebar-section-header">
        <span class="section-title">账户</span>
      </div>
      <nav class="nav-list">
        <button
          v-for="account in store.accounts"
          :key="account.id"
          class="nav-item"
          :class="{ active: store.selectedAccountId === account.id }"
          @click="selectAccount(account.id)"
        >
          <span class="nav-icon">👤</span>
          <span class="nav-label">{{ account.name }}</span>
          <span class="nav-badge" v-if="getAccountUnread(account.id) > 0">
            {{ getAccountUnread(account.id) }}
          </span>
        </button>
      </nav>
    </div>

    <div class="sidebar-footer">
      <button class="sync-btn" @click="syncAll" :disabled="store.syncing">
        <span v-if="store.syncing">⌛ 同步中...</span>
        <span v-else>🔄 同步邮件</span>
      </button>
      <router-link to="/accounts" class="settings-link">
        ⚙️ 账户设置
      </router-link>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useMailStore } from '@/stores/mail'

const store = useMailStore()
const router = useRouter()

function selectAll() {
  store.setSelectedAccount(undefined)
  store.setSelectedTag(undefined)
  store.clearCurrentEmail()
  router.push('/inbox')
  store.loadEmails()
}

function selectTag(tagId: number) {
  store.setSelectedAccount(undefined)
  store.setSelectedTag(tagId)
  store.clearCurrentEmail()
  router.push('/inbox')
  store.loadEmails()
}

function selectAccount(accountId: number) {
  store.setSelectedAccount(accountId)
  store.setSelectedTag(undefined)
  store.clearCurrentEmail()
  router.push('/inbox')
  store.loadEmails()
}

function getAccountUnread(accountId: number): number {
  const found = store.unreadCount.by_account.find(([id]) => id === accountId)
  return found ? found[2] : 0
}

async function syncAll() {
  try {
    await store.syncEmails()
  } catch (e) {
    console.error('Sync failed:', e)
    alert('同步失败: ' + e)
  }
}
</script>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  background: var(--surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.sidebar-header {
  padding: 20px;
  border-bottom: 1px solid var(--border);
}

.app-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.sidebar-section {
  padding: 16px 0;
  border-bottom: 1px solid var(--border);
}

.sidebar-section-header {
  padding: 0 20px 8px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.nav-list {
  display: flex;
  flex-direction: column;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 20px;
  text-align: left;
  transition: background-color 0.15s;
}

.nav-item:hover {
  background-color: var(--background);
}

.nav-item.active {
  background-color: rgba(25, 118, 210, 0.1);
  color: var(--primary);
}

.nav-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
}

.tag-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.nav-label {
  flex: 1;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-badge {
  background-color: var(--primary);
  color: white;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
  min-width: 20px;
  text-align: center;
}

.sidebar-footer {
  margin-top: auto;
  padding: 16px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sync-btn {
  width: 100%;
  padding: 10px 16px;
  background-color: var(--primary);
  color: white;
  border-radius: 6px;
  font-weight: 500;
  transition: background-color 0.15s;
}

.sync-btn:hover:not(:disabled) {
  background-color: var(--primary-dark);
}

.settings-link {
  text-align: center;
  padding: 8px;
  color: var(--text-secondary);
  text-decoration: none;
  border-radius: 6px;
  transition: background-color 0.15s;
}

.settings-link:hover {
  background-color: var(--background);
}
</style>
