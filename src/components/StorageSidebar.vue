<template>
  <aside class="storage-sidebar">
    <div class="instance-rail">
      <n-tooltip v-for="instance in settingsStore.instances" :key="instance.id" placement="right">
        <template #trigger>
          <button
            class="instance-button"
            :class="{ active: instance.id === settingsStore.activeInstanceId }"
            type="button"
            @click="switchOpenList(instance.id)"
            @contextmenu.prevent="openInstanceMenu($event, instance.id)"
          >
            <span>{{ instanceLabel(instance.name) }}</span>
          </button>
        </template>
        {{ instance.name }}
      </n-tooltip>
      <n-tooltip placement="right">
        <template #trigger>
          <button class="instance-button add" type="button" @click="addOpenList">
            <Plus :size="18" />
          </button>
        </template>
        添加 OpenList
      </n-tooltip>
      <n-dropdown
        trigger="manual"
        placement="right-start"
        :x="instanceMenu.x"
        :y="instanceMenu.y"
        :show="instanceMenu.show"
        :options="instanceMenuOptions"
        @clickoutside="instanceMenu.show = false"
        @select="handleInstanceMenuSelect"
      />
    </div>

    <div class="storage-sidebar-body">
      <div class="storage-sidebar-header">
        <span>存储空间</span>
        <n-tooltip>
          <template #trigger>
            <button class="sidebar-refresh-button" type="button" :disabled="storageStore.loading" @click="refreshStorages">
              <RefreshCw :size="15" />
              <span>刷新 OpenList</span>
            </button>
          </template>
          重新连接并读取最新挂载
        </n-tooltip>
      </div>

      <div v-if="storageStore.hasStorages" class="storage-list">
        <button
          v-for="storage in storageStore.storages"
          :key="storage.id"
          class="storage-card"
          :class="{ active: storage.id === storageStore.activeStorageId }"
          type="button"
          @click="selectStorage(storage.id)"
        >
          <span class="storage-logo" :style="{ backgroundColor: storage.color }">{{ storage.iconText }}</span>
          <span class="storage-meta">
            <span class="storage-name">{{ storage.name }}</span>
            <span class="storage-quota">{{ quotaLabel(storage) }}</span>
            <span v-if="hasQuota(storage)" class="quota-track">
              <span class="quota-bar" :style="{ width: `${quotaPercent(storage)}%`, backgroundColor: storage.color }" />
            </span>
          </span>
        </button>
      </div>

      <div v-else class="storage-empty">
        <CircleAlert :size="22" />
        <div class="storage-empty-title">未连接 OpenList</div>
        <div class="storage-empty-text">连接后会自动读取 OpenList 中的挂载存储。</div>
      </div>

      <div class="capacity-summary">
        <div class="summary-label">连接状态</div>
        <div class="summary-value">{{ summaryLabel }}</div>
        <div v-if="totalSize > 0" class="summary-track">
          <span :style="{ width: `${totalPercent}%` }" />
        </div>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { CircleAlert, Plus, RefreshCw } from '@lucide/vue'
import { useDialog, useMessage, type DropdownOption } from 'naive-ui'
import type { StorageEntry } from '@/models/storage'
import { useFilesStore } from '@/stores/files'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'
import { formatBytes } from '@/utils/format'

const storageStore = useStorageStore()
const filesStore = useFilesStore()
const settingsStore = useSettingsStore()
const message = useMessage()
const dialog = useDialog()
const router = useRouter()
const instanceMenu = reactive({
  show: false,
  x: 0,
  y: 0,
  instanceId: ''
})

const totalUsed = computed(() => storageStore.storages.reduce((sum, storage) => sum + (storage.usedBytes ?? 0), 0))
const totalSize = computed(() => storageStore.storages.reduce((sum, storage) => sum + (storage.totalBytes ?? 0), 0))
const totalPercent = computed(() => (totalSize.value ? Math.min(100, Math.round((totalUsed.value / totalSize.value) * 100)) : 0))
const summaryLabel = computed(() => {
  if (storageStore.hasStorages && !totalSize.value) return `${storageStore.storages.length} 个挂载点`
  if (totalSize.value) return `${formatBytes(totalUsed.value)} / ${formatBytes(totalSize.value)}`
  return storageStore.loadError || '等待连接'
})
const instanceMenuOptions = computed<DropdownOption[]>(() => [
  { label: '编辑连接', key: 'edit' },
  { label: '删除 OpenList', key: 'delete', disabled: settingsStore.instances.length <= 1 }
])

function quotaPercent(storage: StorageEntry) {
  if (!storage.usedBytes || !storage.totalBytes) return 0
  return Math.max(3, Math.min(100, Math.round((storage.usedBytes / storage.totalBytes) * 100)))
}

function hasQuota(storage: StorageEntry) {
  return Boolean(storage.usedBytes && storage.totalBytes)
}

function quotaLabel(storage: StorageEntry) {
  if (!storage.usedBytes || !storage.totalBytes) return storage.driver
  return `${formatBytes(storage.usedBytes)} / ${formatBytes(storage.totalBytes)}`
}

function instanceLabel(name: string) {
  return name.trim().slice(0, 1).toUpperCase() || 'O'
}

async function selectStorage(id: string) {
  storageStore.selectStorage(id)
  filesStore.resetToActiveStorage()
  await filesStore.load()
}

async function refreshStorages() {
  try {
    await settingsStore.initializeToken()
    await storageStore.loadFromOpenList()
    if (storageStore.loadError) {
      settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
      message.error(storageStore.loadError)
      return
    }

    filesStore.resetToActiveStorage()
    if (storageStore.activeStorage) await filesStore.load()
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, storageStore.hasStorages ? 'online' : 'unknown')
    message.success(storageStore.hasStorages ? 'OpenList 已刷新' : 'OpenList 已连接，但未读取到挂载点')
  } catch (error) {
    settingsStore.markInstanceStatus(settingsStore.activeInstanceId, 'offline')
    message.error(error instanceof Error ? error.message : 'OpenList 刷新失败')
  }
}

async function switchOpenList(id: string) {
  await settingsStore.switchInstance(id)
  storageStore.clearStorages()
  filesStore.resetToActiveStorage()
  await refreshStorages()
}

function addOpenList() {
  settingsStore.addInstance({
    name: `OpenList ${settingsStore.instances.length + 1}`,
    serverUrl: 'http://127.0.0.1:5244'
  })
  storageStore.clearStorages()
  filesStore.resetToActiveStorage()
  router.push({ name: 'openlist' })
}

function openInstanceMenu(event: MouseEvent, instanceId: string) {
  instanceMenu.instanceId = instanceId
  instanceMenu.show = false
  requestAnimationFrame(() => {
    instanceMenu.x = event.clientX
    instanceMenu.y = event.clientY
    instanceMenu.show = true
  })
}

async function handleInstanceMenuSelect(key: string) {
  const instanceId = instanceMenu.instanceId
  instanceMenu.show = false
  if (!instanceId) return

  if (key === 'edit') {
    await switchOpenList(instanceId)
    await router.push({ name: 'openlist' })
  }

  if (key === 'delete') {
    const instance = settingsStore.instances.find((item) => item.id === instanceId)
    if (!instance) return
    dialog.warning({
      title: '删除 OpenList',
      content: `确认删除“${instance.name}”？保存的访问凭据也会一起删除。`,
      positiveText: '删除',
      negativeText: '取消',
      onPositiveClick: async () => {
        const removed = await settingsStore.removeInstance(instanceId)
        if (!removed) {
          message.warning('至少需要保留一个 OpenList')
          return
        }
        storageStore.clearStorages()
        filesStore.resetToActiveStorage()
        await refreshStorages()
        message.success('OpenList 已删除')
      }
    })
  }
}

</script>
