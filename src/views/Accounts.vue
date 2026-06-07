<template>
  <div class="accounts-container">
    <div class="accounts-header">
      <h1>账户设置</h1>
      <p class="subtitle">管理您的邮件账户（最多支持 5 个账户）</p>
    </div>

    <div class="accounts-content">
      <div class="accounts-list">
        <div class="list-header-row">
          <span class="count-badge">
            {{ store.accounts.length }} / 5 个账户
          </span>
          <button
            class="add-btn"
            @click="showAddForm = true"
            :disabled="store.accounts.length >= 5"
          >
            + 添加账户
          </button>
        </div>

        <div v-if="store.accounts.length === 0" class="empty-accounts">
          <div class="empty-accounts-icon">👤</div>
          <div class="empty-accounts-title">暂无邮件账户</div>
          <div class="empty-accounts-desc">添加您的第一个邮件账户开始使用</div>
          <button class="add-btn primary" @click="showAddForm = true">
            + 添加账户
          </button>
        </div>

        <div
          v-for="account in store.accounts"
          :key="account.id"
          class="account-card"
        >
          <div class="account-avatar">
            {{ account.name.charAt(0).toUpperCase() }}
          </div>
          <div class="account-info">
            <div class="account-name">{{ account.name }}</div>
            <div class="account-email">{{ account.email }}</div>
            <div class="account-server">
              {{ account.use_ssl ? '🔒' : '⚠️' }}
              {{ account.imap_server }}:{{ account.imap_port }}
            </div>
          </div>
          <div class="account-actions">
            <button class="action-btn edit" @click="editAccount(account)">✏️</button>
            <button class="action-btn sync" @click="syncAccount(account)">🔄</button>
            <button class="action-btn delete" @click="confirmDelete(account)">🗑️</button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showAddForm || editingAccount" class="modal-overlay" @click.self="closeForm">
      <div class="modal">
        <div class="modal-header">
          <h2>{{ editingAccount ? '编辑账户' : '添加账户' }}</h2>
          <button class="close-btn" @click="closeForm">✕</button>
        </div>
        <form @submit.prevent="submitForm" class="form">
          <div class="form-row">
            <label>账户名称</label>
            <input
              v-model="formData.name"
              type="text"
              required
              placeholder="例如：工作邮箱"
            />
          </div>

          <div class="form-row">
            <label>邮箱地址</label>
            <input
              v-model="formData.email"
              type="email"
              required
              placeholder="your@email.com"
            />
          </div>

          <div class="form-row">
            <label>用户名</label>
            <input
              v-model="formData.username"
              type="text"
              required
              placeholder="通常是你的邮箱地址"
            />
          </div>

          <div class="form-row">
            <label>密码</label>
            <input
              v-model="formData.password"
              type="password"
              required
              placeholder="邮箱密码或授权码"
            />
          </div>

          <div class="form-grid">
            <div class="form-row">
              <label>IMAP 服务器</label>
              <input
                v-model="formData.imap_server"
                type="text"
                required
                placeholder="imap.example.com"
              />
            </div>
            <div class="form-row">
              <label>端口</label>
              <input
                v-model.number="formData.imap_port"
                type="number"
                required
                min="1"
                max="65535"
                placeholder="993"
              />
            </div>
          </div>

          <div class="form-row checkbox-row">
            <label>
              <input
                v-model="formData.use_ssl"
                type="checkbox"
              />
              使用 SSL 加密连接（推荐）
            </label>
          </div>

          <div class="form-tip" v-if="showTip">
            <strong>💡 常见邮箱设置：</strong>
            <ul>
              <li>QQ邮箱: imap.qq.com:993 (需要使用授权码)</li>
              <li>163邮箱: imap.163.com:993 (需要使用授权码)</li>
              <li>Gmail: imap.gmail.com:993</li>
              <li>Outlook: imap-mail.outlook.com:993</li>
            </ul>
          </div>

          <div class="form-actions">
            <button type="button" class="cancel-btn" @click="closeForm">
              取消
            </button>
            <button type="submit" class="submit-btn" :disabled="submitting">
              <span v-if="submitting">保存中...</span>
              <span v-else>{{ editingAccount ? '保存修改' : '添加账户' }}</span>
            </button>
          </div>
        </form>
      </div>
    </div>

    <div v-if="deleteConfirm" class="modal-overlay" @click.self="deleteConfirm = null">
      <div class="modal small">
        <div class="modal-header">
          <h2>确认删除</h2>
        </div>
        <div class="modal-body">
          <p>确定要删除账户 <strong>{{ deleteConfirm.name }}</strong> 吗？</p>
          <p class="warning">此操作将删除该账户的所有本地邮件数据，且无法恢复。</p>
        </div>
        <div class="form-actions">
          <button class="cancel-btn" @click="deleteConfirm = null">取消</button>
          <button class="delete-btn" @click="doDelete">确认删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useMailStore } from '@/stores/mail'
import type { Account, AccountInput } from '@/types'

const store = useMailStore()
const router = useRouter()

const showAddForm = ref(false)
const editingAccount = ref<Account | null>(null)
const deleteConfirm = ref<Account | null>(null)
const submitting = ref(false)
const showTip = ref(true)

const defaultForm: AccountInput = {
  name: '',
  email: '',
  imap_server: '',
  imap_port: 993,
  username: '',
  password: '',
  use_ssl: true
}

const formData = reactive<AccountInput>({ ...defaultForm })

function resetForm() {
  Object.assign(formData, defaultForm)
}

function editAccount(account: Account) {
  editingAccount.value = account
  Object.assign(formData, {
    name: account.name,
    email: account.email,
    imap_server: account.imap_server,
    imap_port: account.imap_port,
    username: account.username,
    password: account.password,
    use_ssl: account.use_ssl
  })
}

function closeForm() {
  showAddForm.value = false
  editingAccount.value = null
  resetForm()
}

async function submitForm() {
  submitting.value = true
  try {
    if (editingAccount.value) {
      await store.updateAccount(editingAccount.value.id, formData)
    } else {
      const account = await store.addAccount(formData)
      if (store.accounts.length === 1) {
        router.push('/inbox')
      }
    }
    closeForm()
  } catch (e) {
    console.error('Save failed:', e)
    alert('保存失败: ' + e)
  } finally {
    submitting.value = false
  }
}

function confirmDelete(account: Account) {
  deleteConfirm.value = account
}

async function doDelete() {
  if (!deleteConfirm.value) return
  try {
    await store.deleteAccount(deleteConfirm.value.id)
    deleteConfirm.value = null
  } catch (e) {
    console.error('Delete failed:', e)
    alert('删除失败: ' + e)
  }
}

async function syncAccount(account: Account) {
  try {
    await store.syncEmails(account.id)
    alert('同步完成！')
  } catch (e) {
    console.error('Sync failed:', e)
    alert('同步失败: ' + e)
  }
}
</script>

<style scoped>
.accounts-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  background: var(--background);
}

.accounts-header {
  padding: 32px 40px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}

.accounts-header h1 {
  font-size: 24px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.subtitle {
  color: var(--text-secondary);
  font-size: 14px;
}

.accounts-content {
  padding: 32px 40px;
  flex: 1;
}

.accounts-list {
  max-width: 800px;
  margin: 0 auto;
}

.list-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.count-badge {
  font-size: 14px;
  color: var(--text-secondary);
}

.add-btn {
  padding: 10px 20px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-weight: 500;
  color: var(--text-primary);
  transition: all 0.15s;
}

.add-btn:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}

.add-btn.primary {
  background: var(--primary);
  color: white;
  border: none;
}

.add-btn.primary:hover {
  background: var(--primary-dark);
}

.empty-accounts {
  text-align: center;
  padding: 60px 40px;
  background: var(--surface);
  border-radius: 12px;
  border: 1px dashed var(--border);
}

.empty-accounts-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.empty-accounts-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.empty-accounts-desc {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 24px;
}

.account-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: var(--surface);
  border-radius: 8px;
  border: 1px solid var(--border);
  margin-bottom: 12px;
  transition: box-shadow 0.15s;
}

.account-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.account-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--primary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  font-weight: 600;
  flex-shrink: 0;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.account-email {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.account-server {
  font-size: 12px;
  color: var(--text-light);
}

.account-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.action-btn {
  width: 36px;
  height: 36px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  font-size: 16px;
  transition: all 0.15s;
}

.action-btn:hover {
  background: var(--background);
}

.action-btn.delete:hover {
  background: var(--error);
  color: white;
  border-color: var(--error);
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: var(--surface);
  border-radius: 12px;
  width: 90%;
  max-width: 500px;
  max-height: 90vh;
  overflow-y: auto;
}

.modal.small {
  max-width: 400px;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border);
}

.modal-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.close-btn {
  background: none;
  border: none;
  font-size: 20px;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
}

.close-btn:hover {
  background: var(--background);
}

.modal-body {
  padding: 20px 24px;
}

.modal-body p {
  margin-bottom: 12px;
  line-height: 1.6;
}

.warning {
  color: var(--error);
  font-size: 13px;
}

.form {
  padding: 24px;
}

.form-row {
  margin-bottom: 16px;
}

.form-row label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.form-row input[type="text"],
.form-row input[type="email"],
.form-row input[type="password"],
.form-row input[type="number"] {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  outline: none;
  transition: border-color 0.15s;
}

.form-row input:focus {
  border-color: var(--primary);
}

.form-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 16px;
}

.checkbox-row label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  margin: 0;
}

.checkbox-row input[type="checkbox"] {
  width: 18px;
  height: 18px;
}

.form-tip {
  padding: 12px 16px;
  background: var(--background);
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 16px;
}

.form-tip ul {
  margin-top: 8px;
  padding-left: 20px;
}

.form-tip li {
  margin-bottom: 4px;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 0 24px 24px;
}

.cancel-btn {
  padding: 10px 20px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-secondary);
  font-weight: 500;
  transition: all 0.15s;
}

.cancel-btn:hover {
  background: var(--background);
}

.submit-btn {
  padding: 10px 24px;
  background: var(--primary);
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 500;
  transition: background-color 0.15s;
}

.submit-btn:hover:not(:disabled) {
  background: var(--primary-dark);
}

.delete-btn {
  padding: 10px 24px;
  background: var(--error);
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 500;
  transition: background-color 0.15s;
}

.delete-btn:hover {
  background: #d32f2f;
}
</style>
