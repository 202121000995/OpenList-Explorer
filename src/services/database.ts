import { invoke } from '@tauri-apps/api/core'
import type { FavoriteEntry } from '@/stores/favorites'
import type { HistoryEntry } from '@/stores/history'
import type { TransferTask } from '@/models/task'

export async function dbGetJson<T>(key: string): Promise<T | null> {
  try {
    const value = await invoke<string | null>('db_get_json', { key })
    if (!value) return null
    return JSON.parse(value) as T
  } catch {
    return null
  }
}

export async function dbSetJson<T>(key: string, value: T) {
  try {
    await invoke('db_set_json', {
      key,
      value: JSON.stringify(value)
    })
  } catch {
    // Browser preview fallback keeps using VueUse localStorage.
  }
}

export async function dbGetFavorites(): Promise<FavoriteEntry[] | null> {
  try {
    return await invoke<FavoriteEntry[]>('db_get_favorites')
  } catch {
    return null
  }
}

export async function dbReplaceFavorites(items: FavoriteEntry[]) {
  try {
    await invoke('db_replace_favorites', { items })
  } catch {
    // Browser preview fallback keeps using VueUse localStorage.
  }
}

export async function dbGetHistory(): Promise<HistoryEntry[] | null> {
  try {
    return await invoke<HistoryEntry[]>('db_get_history')
  } catch {
    return null
  }
}

export async function dbReplaceHistory(items: HistoryEntry[]) {
  try {
    await invoke('db_replace_history', { items })
  } catch {
    // Browser preview fallback keeps using VueUse localStorage.
  }
}

export async function dbGetTasks(): Promise<TransferTask[] | null> {
  try {
    return await invoke<TransferTask[]>('db_get_tasks')
  } catch {
    return null
  }
}

export async function dbReplaceTasks(items: TransferTask[]) {
  try {
    await invoke('db_replace_tasks', { items })
  } catch {
    // Browser preview fallback keeps using VueUse localStorage.
  }
}
