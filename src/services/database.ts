import { invoke } from '@tauri-apps/api/core'

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
