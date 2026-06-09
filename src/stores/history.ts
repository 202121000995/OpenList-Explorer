import { ref } from 'vue'
import { defineStore } from 'pinia'

export interface HistoryEntry {
  id: string
  type: 'browse' | 'search' | 'download' | 'upload'
  path: string
  time: number
}

export const useHistoryStore = defineStore('history', () => {
  const items = ref<HistoryEntry[]>([])

  function add(type: HistoryEntry['type'], path: string) {
    items.value.unshift({
      id: crypto.randomUUID(),
      type,
      path,
      time: Date.now()
    })
    items.value = items.value.slice(0, 100)
  }

  return { items, add }
})
