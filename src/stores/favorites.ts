import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

export interface FavoriteEntry {
  id: string
  storage: string
  path: string
}

export const useFavoritesStore = defineStore('favorites', () => {
  const items = ref<FavoriteEntry[]>([])

  const paths = computed(() => new Set(items.value.map((item) => `${item.storage}:${item.path}`)))

  function isFavorite(storage: string, path: string) {
    return paths.value.has(`${storage}:${path}`)
  }

  function toggle(storage: string, path: string) {
    const key = `${storage}:${path}`
    if (paths.value.has(key)) {
      items.value = items.value.filter((item) => `${item.storage}:${item.path}` !== key)
      return
    }

    items.value.unshift({
      id: crypto.randomUUID(),
      storage,
      path
    })
  }

  return { items, isFavorite, toggle }
})
