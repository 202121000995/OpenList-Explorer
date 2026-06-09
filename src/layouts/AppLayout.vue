<template>
  <div class="desktop-window">
    <header class="window-titlebar">
      <button class="window-brand" type="button" @click="router.push({ name: 'files' })">
        <Cloud :size="17" />
        <span>OpenList Explorer</span>
      </button>

      <nav class="titlebar-nav">
        <button
          v-for="item in navItems"
          :key="item.name"
          class="titlebar-nav-item"
          :class="{ active: route.name === item.name }"
          type="button"
          @click="router.push({ name: item.name })"
        >
          <component :is="item.icon" :size="15" />
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <div class="window-actions">
        <div class="connection-pill" aria-live="polite">
          <span class="status-dot" :class="connectionClass" />
          <span>{{ connectionLabel }}</span>
        </div>
        <n-tooltip>
          <template #trigger>
            <button class="openlist-button" type="button" @click="router.push({ name: 'openlist' })">
              设置 OpenList
            </button>
          </template>
          连接内置或已有 OpenList
        </n-tooltip>
        <n-tooltip>
          <template #trigger>
            <button class="chrome-button" type="button" @click="router.push({ name: 'settings' })">
              <Settings :size="16" />
            </button>
          </template>
          界面设置
        </n-tooltip>
        <n-tooltip>
          <template #trigger>
            <button class="chrome-button" type="button" @click="toggleTheme">
              <Moon v-if="settingsStore.effectiveTheme === 'light'" :size="16" />
              <Sun v-else :size="16" />
            </button>
          </template>
          切换主题
        </n-tooltip>
      </div>
    </header>

    <main class="window-content">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { Cloud, Download, Files, Moon, Settings, Sun, Upload } from '@lucide/vue'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'

const route = useRoute()
const router = useRouter()
const settingsStore = useSettingsStore()
const storageStore = useStorageStore()

const navItems = [
  { name: 'files', label: '文件', icon: Files },
  { name: 'uploads', label: '上传列表', icon: Upload },
  { name: 'downloads', label: '下载列表', icon: Download }
]

const connectionClass = computed(() => {
  if (settingsStore.hasToken && storageStore.hasStorages) return 'online'
  if (settingsStore.hasToken) return 'warning'
  return 'offline'
})

const connectionLabel = computed(() => {
  if (settingsStore.hasToken && storageStore.hasStorages) return 'OpenList 已连接'
  if (settingsStore.hasToken) return 'OpenList 待读取'
  return 'OpenList 未连接'
})

function toggleTheme() {
  settingsStore.theme = settingsStore.effectiveTheme === 'dark' ? 'light' : 'dark'
}

onMounted(() => {
  settingsStore.initializeToken()
})
</script>
