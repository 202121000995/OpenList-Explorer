import { computed, watch } from 'vue'
import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'
import { dbGetFavorites, dbGetJson, dbReplaceFavorites } from '@/services/database'

export interface FavoriteEntry {
  id: string
  storage: string
  path: string
}

export const useFavoritesStore = defineStore('favorites', () => {
  const items = useStorage<FavoriteEntry[]>('openlist.favorites', [])
  let hydrated = false

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

  async function hydrateFromDatabase() {
    const saved = (await dbGetFavorites()) ?? (await dbGetJson<FavoriteEntry[]>('favorites'))
    if (saved) items.value = saved
    hydrated = true
  }

  watch(items, (value) => {
    if (hydrated) dbReplaceFavorites(value)
  }, { deep: true })

  return { items, isFavorite, toggle, hydrateFromDatabase }
})
