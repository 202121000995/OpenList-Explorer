import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { fsApi } from '@/api/fs'
import type { ExplorerFileItem } from '@/models/file'
import { useHistoryStore } from '@/stores/history'
import { useSettingsStore } from '@/stores/settings'
import { useStorageStore } from '@/stores/storage'
import { dirname, joinPath } from '@/utils/path'

function mapFile(parentPath: string, item: ExplorerFileItem['raw']): ExplorerFileItem {
  const path = joinPath(parentPath, item.name)
  return {
    id: path,
    name: item.name,
    path,
    type: item.is_dir ? 'folder' : 'file',
    size: item.size,
    modifiedAt: item.modified,
    raw: item
  }
}

export const useFilesStore = defineStore('files', () => {
  const storageStore = useStorageStore()
  const settingsStore = useSettingsStore()
  const historyStore = useHistoryStore()

  const currentPath = ref(storageStore.activeStorage?.mountPath ?? '')
  const files = ref<ExplorerFileItem[]>([])
  const selectedPaths = ref<string[]>([])
  const loading = ref(false)
  const lastError = ref('')
  const keyword = ref('')
  const sortKey = computed({
    get: () => settingsStore.fileSortKey,
    set: (value) => {
      settingsStore.fileSortKey = value
    }
  })
  const sortOrder = computed({
    get: () => settingsStore.fileSortOrder,
    set: (value) => {
      settingsStore.fileSortOrder = value
    }
  })

  const selectedFiles = computed(() => files.value.filter((file) => selectedPaths.value.includes(file.path)))

  const sortedFiles = computed(() => {
    return [...files.value].sort((a, b) => {
      if (a.type !== b.type) return a.type === 'folder' ? -1 : 1
      const direction = sortOrder.value === 'asc' ? 1 : -1
      if (sortKey.value === 'size') return (a.size - b.size) * direction
      return String(a[sortKey.value]).localeCompare(String(b[sortKey.value]), 'zh-CN') * direction
    })
  })

  async function load(path = currentPath.value, refresh = false) {
    if (!path) {
      files.value = []
      selectedPaths.value = []
      lastError.value = '请先设置 OpenList 连接'
      return
    }

    loading.value = true
    lastError.value = ''
    selectedPaths.value = []

    try {
      const response = await fsApi.list({ path, refresh, page: 1, per_page: 1000 })
      currentPath.value = path
      files.value = (response.content ?? []).map((item) => mapFile(path, item))
      if (path === '/') {
        storageStore.applyRootList(response)
      }
      historyStore.add('browse', path)
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : '目录加载失败'
    } finally {
      loading.value = false
    }
  }

  async function enter(file: ExplorerFileItem) {
    if (file.type !== 'folder') return
    await load(file.path)
  }

  async function goUp() {
    if (!currentPath.value) return
    await load(dirname(currentPath.value))
  }

  async function refresh() {
    if (!currentPath.value) return
    await load(currentPath.value, true)
  }

  async function search() {
    const words = keyword.value.trim()
    if (!words) {
      await load(currentPath.value)
      return
    }

    if (!currentPath.value) {
      lastError.value = '请先设置 OpenList 连接'
      return
    }

    loading.value = true
    lastError.value = ''
    selectedPaths.value = []

    try {
      const response = await fsApi.search({
        parent: currentPath.value,
        keywords: words,
        page: 1,
        per_page: 1000
      })
      files.value = (response.content ?? []).map((item) => mapFile(currentPath.value, item))
      historyStore.add('search', `${currentPath.value}?q=${words}`)
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : '搜索失败'
    } finally {
      loading.value = false
    }
  }

  async function removeSelected() {
    if (!selectedFiles.value.length) return
    await fsApi.remove({
      dir: currentPath.value,
      names: selectedFiles.value.map((file) => file.name)
    })
    await refresh()
  }

  async function moveSelected(destinationPath: string) {
    if (!selectedFiles.value.length) return
    await fsApi.move({
      src_dir: currentPath.value,
      dst_dir: destinationPath,
      names: selectedFiles.value.map((file) => file.name)
    })
    await refresh()
  }

  async function copySelected(destinationPath: string) {
    if (!selectedFiles.value.length) return
    await fsApi.copy({
      src_dir: currentPath.value,
      dst_dir: destinationPath,
      names: selectedFiles.value.map((file) => file.name)
    })
    await refresh()
  }

  async function rename(file: ExplorerFileItem, name: string) {
    await fsApi.rename({ storage: storageStore.activeStorageId, path: file.path, name })
    await refresh()
  }

  async function mkdir(name: string) {
    if (!currentPath.value) return
    await fsApi.mkdir(joinPath(currentPath.value, name))
    await refresh()
  }

  async function getRawUrl(file: ExplorerFileItem) {
    const detail = await fsApi.get(file.path)
    return rewriteRawUrl(detail.raw_url ?? '')
  }

  function rewriteRawUrl(rawUrl: string) {
    const publicBaseUrl = settingsStore.publicBaseUrl.trim().replace(/\/+$/, '')
    if (!rawUrl || !publicBaseUrl) return rawUrl

    try {
      const raw = new URL(rawUrl)
      if (!isPrivateHost(raw.hostname)) return rawUrl
      const publicBase = new URL(publicBaseUrl)
      raw.protocol = publicBase.protocol
      raw.host = publicBase.host
      return raw.toString()
    } catch {
      return rawUrl
    }
  }

  function isPrivateHost(hostname: string) {
    const host = hostname.replace(/^\[|\]$/g, '').toLowerCase()
    if (host === 'localhost' || host.endsWith('.local')) return true
    if (!host.includes('.') && !host.includes(':')) return true
    if (host === '::1' || host.startsWith('fe80:') || host.startsWith('fc') || host.startsWith('fd')) return true

    const parts = host.split('.').map((part) => Number(part))
    if (parts.length !== 4 || parts.some((part) => Number.isNaN(part))) return false
    const [a, b] = parts
    return (
      a === 10 ||
      a === 127 ||
      (a === 172 && b >= 16 && b <= 31) ||
      (a === 192 && b === 168) ||
      (a === 169 && b === 254)
    )
  }

  function resetToActiveStorage() {
    currentPath.value = storageStore.activeStorage?.mountPath ?? ''
    files.value = []
    selectedPaths.value = []
    lastError.value = ''
  }

  return {
    currentPath,
    files,
    sortedFiles,
    selectedPaths,
    selectedFiles,
    loading,
    lastError,
    keyword,
    sortKey,
    sortOrder,
    load,
    enter,
    goUp,
    refresh,
    search,
    removeSelected,
    moveSelected,
    copySelected,
    rename,
    mkdir,
    getRawUrl,
    resetToActiveStorage
  }
})
