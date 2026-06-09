import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { fsApi } from '@/api/fs'
import { defaultStorages, type StorageEntry } from '@/models/storage'
import { resolveStorageLogo } from '@/models/storageLogos'

export const useStorageStore = defineStore('storage', () => {
  const storages = ref<StorageEntry[]>(defaultStorages)
  const activeStorageId = ref('')
  const loading = ref(false)
  const loadError = ref('')
  const lastLoadedAt = ref<number | null>(null)

  const activeStorage = computed(
    () => storages.value.find((storage) => storage.id === activeStorageId.value) ?? storages.value[0] ?? null
  )
  const hasStorages = computed(() => storages.value.length > 0)

  function selectStorage(id: string) {
    activeStorageId.value = id
  }

  async function loadFromOpenList() {
    loading.value = true
    loadError.value = ''

    try {
      const response = await fsApi.list({ path: '/', page: 1, per_page: 200, refresh: true })
      const mountDirs = (response.content ?? []).filter((item) => item.is_dir)

      storages.value = mountDirs.map((item) => {
        const mountPath = `/${item.name}`
        const logo = resolveStorageLogo(item.name)

        return {
          id: mountPath,
          name: item.name,
          driver: response.provider && response.provider !== 'unknown' ? response.provider : 'OpenList',
          mountPath,
          color: logo.color,
          iconText: logo.label,
          logoKey: logo.key,
          usedBytes: item.size > 0 ? item.size : undefined,
          totalBytes: undefined
        }
      })

      if (!storages.value.some((storage) => storage.id === activeStorageId.value)) {
        activeStorageId.value = storages.value[0]?.id ?? ''
      }
      lastLoadedAt.value = Date.now()
    } catch (error) {
      storages.value = []
      activeStorageId.value = ''
      loadError.value = error instanceof Error ? error.message : '存储列表加载失败'
    } finally {
      loading.value = false
    }
  }

  function clearStorages() {
    storages.value = []
    activeStorageId.value = ''
    loadError.value = ''
    lastLoadedAt.value = null
  }

  return {
    storages,
    activeStorageId,
    activeStorage,
    hasStorages,
    loading,
    loadError,
    lastLoadedAt,
    selectStorage,
    loadFromOpenList,
    clearStorages
  }
})
