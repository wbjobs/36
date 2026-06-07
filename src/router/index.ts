import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useMailStore } from '@/stores/mail'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/inbox'
  },
  {
    path: '/inbox',
    name: 'Inbox',
    component: () => import('@/views/Inbox.vue')
  },
  {
    path: '/accounts',
    name: 'Accounts',
    component: () => import('@/views/Accounts.vue')
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

router.beforeEach(async (to, from, next) => {
  const store = useMailStore()
  if (store.accounts.length === 0) {
    try {
      await store.loadAccounts()
      await store.loadTags()
    } catch (e) {
      console.error('Failed to load initial data:', e)
    }
  }

  if (to.name !== 'Accounts' && !store.hasAccounts) {
    next({ name: 'Accounts' })
  } else {
    next()
  }
})

export default router
