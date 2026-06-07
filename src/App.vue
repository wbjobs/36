<template>
  <div class="app-container">
    <Sidebar />
    <router-view v-if="store.hasAccounts" />
    <router-view v-else />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import Sidebar from '@/components/Sidebar.vue'
import { useMailStore } from '@/stores/mail'

const store = useMailStore()

onMounted(async () => {
  try {
    await store.loadAccounts()
    await store.loadTags()
  } catch (e) {
    console.error('Failed to load initial data:', e)
  }
})
</script>

<style scoped>
.app-container {
  display: flex;
  width: 100%;
  height: 100%;
}
</style>
