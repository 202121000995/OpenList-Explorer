import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { usePreferredDark, useStorage } from '@vueuse/core'
import { tokenVault } from '@/services/tokenVault'

export type ThemeMode = 'light' | 'dark' | 'auto'
export type LanguageMode = 'zh-CN' | 'en-US'

export interface OpenListInstance {
  id: string
  name: string
  serverUrl: string
  username: string
  publicBaseUrl: string
  isBuiltin: boolean
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
  const theme = useStorage<ThemeMode>('openlist.theme', 'auto')
  const language = useStorage<LanguageMode>('openlist.language', 'zh-CN')
  const downloadDir = useStorage('openlist.downloadDir', '')
  const uploadThreads = useStorage('openlist.uploadThreads', 3)
  const downloadThreads = useStorage('openlist.downloadThreads', 3)
  const cacheSize = ref('0 MB')
  const hasToken = ref(false)

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
        isBuiltin
      }
      instances.value = [instance]
      activeInstanceId.value = instance.id
    }

    if (!activeInstanceId.value || !instances.value.some((instance) => instance.id === activeInstanceId.value)) {
      activeInstanceId.value = instances.value[0]?.id ?? ''
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
      isBuiltin: Boolean(partial.isBuiltin)
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
    if (activeInstanceId.value === id) {
      activeInstanceId.value = instances.value[0]?.id ?? ''
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

  return {
    serverUrl,
    username,
    publicBaseUrl,
    instances,
    activeInstanceId,
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
    updateToken,
    clearToken,
    initializeToken
  }
})
