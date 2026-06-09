import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { usePreferredDark, useStorage } from '@vueuse/core'
import { dbGetJson, dbSetJson } from '@/services/database'
import { tokenVault } from '@/services/tokenVault'

export type ThemeMode = 'light' | 'dark' | 'auto'
export type LanguageMode = 'zh-CN' | 'en-US'
export type OpenListInstanceStatus = 'unknown' | 'online' | 'offline'

export interface OpenListInstance {
  id: string
  name: string
  serverUrl: string
  username: string
  publicBaseUrl: string
  isBuiltin: boolean
  lastConnectedAt?: number
  lastStatus?: OpenListInstanceStatus
}

interface SettingsSnapshot {
  instances: OpenListInstance[]
  activeInstanceId: string
  defaultInstanceId: string
  theme: ThemeMode
  language: LanguageMode
  downloadDir: string
  uploadThreads: number
  downloadThreads: number
}

function createId() {
  return crypto.randomUUID?.() ?? `instance-${Date.now()}`
}

export const useSettingsStore = defineStore('settings', () => {
  const prefersDark = usePreferredDark()
  const legacyServerUrl = useStorage('openlist.serverUrl', 'http://127.0.0.1:5244')
  const legacyUsername = useStorage('openlist.username', '')
  const legacyPublicBaseUrl = useStorage('openlist.publicBaseUrl', '')
  const instances = useStorage<OpenListInstance[]>('openlist.instances', [])
  const activeInstanceId = useStorage('openlist.activeInstanceId', '')
  const defaultInstanceId = useStorage('openlist.defaultInstanceId', '')
  const theme = useStorage<ThemeMode>('openlist.theme', 'auto')
  const language = useStorage<LanguageMode>('openlist.language', 'zh-CN')
  const downloadDir = useStorage('openlist.downloadDir', '')
  const uploadThreads = useStorage('openlist.uploadThreads', 3)
  const downloadThreads = useStorage('openlist.downloadThreads', 3)
  const cacheSize = ref('0 MB')
  const hasToken = ref(false)
  let hydrated = false

  const effectiveTheme = computed(() => (theme.value === 'auto' ? (prefersDark.value ? 'dark' : 'light') : theme.value))
  const activeInstance = computed(() => {
    ensureInstances()
    return instances.value.find((instance) => instance.id === activeInstanceId.value) ?? instances.value[0]
  })

  const serverUrl = computed({
    get: () => activeInstance.value?.serverUrl ?? 'http://127.0.0.1:5244',
    set: (value: string) => updateActiveInstance({ serverUrl: value })
  })

  const username = computed({
    get: () => activeInstance.value?.username ?? '',
    set: (value: string) => updateActiveInstance({ username: value })
  })

  const publicBaseUrl = computed({
    get: () => activeInstance.value?.publicBaseUrl ?? '',
    set: (value: string) => updateActiveInstance({ publicBaseUrl: value })
  })

  function ensureInstances() {
    if (!instances.value.length) {
      const isBuiltin = legacyServerUrl.value.includes('127.0.0.1') || legacyServerUrl.value.includes('localhost')
      const instance: OpenListInstance = {
        id: isBuiltin ? 'builtin-local' : createId(),
        name: isBuiltin ? '本机 OpenList' : 'OpenList',
        serverUrl: legacyServerUrl.value || 'http://127.0.0.1:5244',
        username: legacyUsername.value,
        publicBaseUrl: legacyPublicBaseUrl.value,
        isBuiltin,
        lastStatus: 'unknown'
      }
      instances.value = [instance]
      activeInstanceId.value = instance.id
      defaultInstanceId.value = instance.id
    }

    instances.value = instances.value.map((instance) => ({
      ...instance,
      lastStatus: instance.lastStatus ?? 'unknown'
    }))

    if (!defaultInstanceId.value || !instances.value.some((instance) => instance.id === defaultInstanceId.value)) {
      defaultInstanceId.value = instances.value[0]?.id ?? ''
    }

    if (!activeInstanceId.value || !instances.value.some((instance) => instance.id === activeInstanceId.value)) {
      activeInstanceId.value = defaultInstanceId.value || instances.value[0]?.id || ''
    }
  }

  function updateInstance(id: string, patch: Partial<Omit<OpenListInstance, 'id'>>) {
    instances.value = instances.value.map((instance) => (instance.id === id ? { ...instance, ...patch } : instance))
  }

  function updateActiveInstance(patch: Partial<Omit<OpenListInstance, 'id'>>) {
    ensureInstances()
    if (!activeInstanceId.value) return
    updateInstance(activeInstanceId.value, patch)
  }

  function addInstance(partial: Partial<Omit<OpenListInstance, 'id'>>) {
    const instance: OpenListInstance = {
      id: createId(),
      name: partial.name || '远程 OpenList',
      serverUrl: partial.serverUrl || 'http://127.0.0.1:5244',
      username: partial.username || '',
      publicBaseUrl: partial.publicBaseUrl || '',
      isBuiltin: Boolean(partial.isBuiltin),
      lastConnectedAt: partial.lastConnectedAt,
      lastStatus: partial.lastStatus || 'unknown'
    }
    instances.value = [...instances.value, instance]
    activeInstanceId.value = instance.id
    hasToken.value = false
    return instance
  }

  async function removeInstance(id: string) {
    ensureInstances()
    if (instances.value.length <= 1) return false
    await tokenVault.clearToken(id)
    instances.value = instances.value.filter((instance) => instance.id !== id)
    if (defaultInstanceId.value === id) {
      defaultInstanceId.value = instances.value[0]?.id ?? ''
    }
    if (activeInstanceId.value === id) {
      activeInstanceId.value = defaultInstanceId.value || instances.value[0]?.id || ''
      await initializeToken()
    }
    return true
  }

  async function switchInstance(id: string) {
    ensureInstances()
    if (!instances.value.some((instance) => instance.id === id)) return false
    activeInstanceId.value = id
    return initializeToken()
  }

  function setDefaultInstance(id: string) {
    ensureInstances()
    if (!instances.value.some((instance) => instance.id === id)) return false
    defaultInstanceId.value = id
    return true
  }

  function markInstanceStatus(id: string, status: OpenListInstanceStatus) {
    const patch: Partial<Omit<OpenListInstance, 'id'>> = { lastStatus: status }
    if (status === 'online') patch.lastConnectedAt = Date.now()
    updateInstance(id, patch)
  }

  async function updateToken(token: string) {
    ensureInstances()
    await tokenVault.setToken(activeInstanceId.value, token)
    hasToken.value = Boolean(token.trim())
  }

  async function clearToken() {
    ensureInstances()
    await tokenVault.clearToken(activeInstanceId.value)
    hasToken.value = false
  }

  async function initializeToken() {
    ensureInstances()
    hasToken.value = await tokenVault.hasStoredToken(activeInstanceId.value)
    return hasToken.value
  }

  async function hydrateFromDatabase() {
    const saved = await dbGetJson<SettingsSnapshot>('settings')
    if (saved) {
      if (saved.instances?.length) instances.value = saved.instances
      activeInstanceId.value = saved.activeInstanceId || activeInstanceId.value
      defaultInstanceId.value = saved.defaultInstanceId || defaultInstanceId.value
      theme.value = saved.theme || theme.value
      language.value = saved.language || language.value
      downloadDir.value = saved.downloadDir || downloadDir.value
      uploadThreads.value = saved.uploadThreads || uploadThreads.value
      downloadThreads.value = saved.downloadThreads || downloadThreads.value
      ensureInstances()
    }
    hydrated = true
  }

  watch(
    [instances, activeInstanceId, defaultInstanceId, theme, language, downloadDir, uploadThreads, downloadThreads],
    () => {
      if (!hydrated) return
      dbSetJson<SettingsSnapshot>('settings', {
        instances: instances.value,
        activeInstanceId: activeInstanceId.value,
        defaultInstanceId: defaultInstanceId.value,
        theme: theme.value,
        language: language.value,
        downloadDir: downloadDir.value,
        uploadThreads: uploadThreads.value,
        downloadThreads: downloadThreads.value
      })
    },
    { deep: true }
  )

  return {
    serverUrl,
    username,
    publicBaseUrl,
    instances,
    activeInstanceId,
    defaultInstanceId,
    activeInstance,
    theme,
    language,
    downloadDir,
    uploadThreads,
    downloadThreads,
    cacheSize,
    hasToken,
    effectiveTheme,
    ensureInstances,
    addInstance,
    updateInstance,
    removeInstance,
    switchInstance,
    setDefaultInstance,
    markInstanceStatus,
    updateToken,
    clearToken,
    initializeToken,
    hydrateFromDatabase
  }
})
