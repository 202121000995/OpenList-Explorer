import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'
import { watch } from 'vue'
import { dbGetHistory, dbGetJson, dbReplaceHistory } from '@/services/database'

export interface HistoryEntry {
  id: string
  type: 'browse' | 'search' | 'download' | 'upload'
  path: string
  time: number
}

export const useHistoryStore = defineStore('history', () => {
  const items = useStorage<HistoryEntry[]>('openlist.history', [])
  let hydrated = false

  function add(type: HistoryEntry['type'], path: string) {
    items.value.unshift({
      id: crypto.randomUUID(),
      type,
      path,
      time: Date.now()
    })
    items.value = items.value.slice(0, 100)
  }

  async function hydrateFromDatabase() {
    const saved = (await dbGetHistory()) ?? (await dbGetJson<HistoryEntry[]>('history'))
    if (saved) items.value = saved
    hydrated = true
  }

  watch(items, (value) => {
    if (hydrated) dbReplaceHistory(value)
  }, { deep: true })

  return { items, add, hydrateFromDatabase }
})
