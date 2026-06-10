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
          :class="{ active: isNavActive(item.name) }"
          type="button"
          @click="handleNavClick(item.name)"
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
            <button class="chrome-button" type="button" @click="showSettingsModal = true">
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

    <n-modal v-model:show="showTaskModal" display-directive="show">
      <n-card
        class="shell-modal task-shell-modal"
        :title="taskModalType === 'upload' ? '上传列表' : '下载列表'"
        role="dialog"
        aria-modal="true"
        closable
        @close="showTaskModal = false"
      >
        <TasksView :type="taskModalType" embedded />
      </n-card>
    </n-modal>

    <n-modal v-model:show="showSettingsModal" display-directive="show">
      <n-card
        class="shell-modal settings-shell-modal"
        title="界面设置"
        role="dialog"
        aria-modal="true"
        closable
        @close="showSettingsModal = false"
      >
        <SettingsView embedded />
      </n-card>
    </n-modal>

    <n-modal v-model:show="showOnboarding" :mask-closable="false">
      <n-card class="onboarding-modal" title="连接 OpenList" role="dialog" aria-modal="true">
        <div class="onboarding-options">
          <button class="onboarding-option" type="button" @click="chooseOnboarding('builtin')">
            <Cloud :size="22" />
            <span>
              <strong>使用内置 OpenList</strong>
              <small>适合新用户，软件会启动随安装包提供的 OpenList。</small>
            </span>
          </button>
          <button class="onboarding-option" type="button" @click="chooseOnboarding('existing')">
            <Settings :size="22" />
            <span>
              <strong>连接已有 OpenList</strong>
              <small>输入服务器地址和 OpenList 账号密码。</small>
            </span>
          </button>
        </div>
      </n-card>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { Cloud, Download, Files, Moon, Settings, Sun, Upload } from '@lucide/vue'
import SettingsView from '@/views/SettingsView.vue'
import TasksView from '@/views/TasksView.vue'
import { useFilesStore } from '@/stores/files'
import { useFavoritesStore } from '@/stores/favorites'
import { useHistoryStore } from '@/stores/history'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'
import { useTasksStore } from '@/stores/tasks'
import { syncOfflineDownloadTasks } from '@/services/offlineTasks'
import type { TransferStatus } from '@/models/task'

const route = useRoute()
const router = useRouter()
const settingsStore = useSettingsStore()
const storageStore = useStorageStore()
const filesStore = useFilesStore()
const tasksStore = useTasksStore()
const favoritesStore = useFavoritesStore()
const historyStore = useHistoryStore()
const showOnboarding = ref(false)
const showTaskModal = ref(false)
const showSettingsModal = ref(false)
const taskModalType = ref<'upload' | 'download'>('upload')
let unlistenTransfer: UnlistenFn | null = null
let offlineTaskTimer: number | null = null

interface TransferProgressPayload {
  id: string
  status: TransferStatus
  progress: number
  speed: number
  local_path?: string
}

const navItems = [
  { name: 'files', label: '文件', icon: Files },
  { name: 'uploads', label: '上传列表', icon: Upload },
  { name: 'downloads', label: '下载列表', icon: Download }
]

function isNavActive(name: string) {
  if (name === 'uploads') return showTaskModal.value && taskModalType.value === 'upload'
  if (name === 'downloads') return showTaskModal.value && taskModalType.value === 'download'
  return route.name === name
}

async function handleNavClick(name: string) {
  if (name === 'uploads' || name === 'downloads') {
    taskModalType.value = name === 'uploads' ? 'upload' : 'download'
    showTaskModal.value = true
    return
  }
  await router.push({ name })
}

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

async function chooseOnboarding(mode: 'builtin' | 'existing') {
  localStorage.setItem('openlist.onboardingDone', '1')
  showOnboarding.value = false
  await router.push({ name: 'openlist', query: { mode } })
}

function startOfflineTaskSync() {
  if (offlineTaskTimer !== null) return
  syncOfflineDownloadTasks()
  offlineTaskTimer = window.setInterval(() => {
    if (settingsStore.hasToken) syncOfflineDownloadTasks()
  }, 10000)
}

function stopOfflineTaskSync() {
  if (offlineTaskTimer !== null) {
    window.clearInterval(offlineTaskTimer)
    offlineTaskTimer = null
  }
}

function restartOfflineTaskSync() {
  stopOfflineTaskSync()
  if (settingsStore.hasToken) startOfflineTaskSync()
}

onMounted(async () => {
  await Promise.all([
    settingsStore.hydrateFromDatabase(),
    favoritesStore.hydrateFromDatabase(),
    historyStore.hydrateFromDatabase(),
    tasksStore.hydrateFromDatabase()
  ])

  unlistenTransfer = await listen<TransferProgressPayload>('transfer://progress', (event) => {
    tasksStore.updateTask(event.payload.id, {
      status: event.payload.status,
      progress: event.payload.progress,
      speed: event.payload.speed,
      localPath: event.payload.local_path
    })
  })

  settingsStore.ensureInstances()
  const hasToken = await settingsStore.initializeToken()
  if (!hasToken && localStorage.getItem('openlist.onboardingDone') !== '1') {
    showOnboarding.value = true
  }
  if (!hasToken) return

  await storageStore.loadFromOpenList()
  settingsStore.markInstanceStatus(settingsStore.activeInstanceId, storageStore.loadError ? 'offline' : 'online')
  startOfflineTaskSync()
  if (route.name === 'files' && storageStore.activeStorage) {
    filesStore.resetToActiveStorage()
    await filesStore.load(storageStore.activeStorage.mountPath)
  }
})

watch(() => [settingsStore.hasToken, settingsStore.activeInstanceId] as const, restartOfflineTaskSync)

onBeforeUnmount(() => {
  unlistenTransfer?.()
  stopOfflineTaskSync()
})
</script>
