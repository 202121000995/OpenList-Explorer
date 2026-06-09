import { invoke } from '@tauri-apps/api/core'

export interface BuiltinOpenListStatus {
  available: boolean
  running: boolean
  server_url: string
  binary_path?: string
  data_dir?: string
  message: string
}

export interface BuiltinOpenListSession {
  server_url: string
  token: string
  data_dir: string
  admin_username: string
  admin_password: string
}

export async function getBuiltinOpenListStatus() {
  return invoke<BuiltinOpenListStatus>('builtin_openlist_status')
}

export async function startBuiltinOpenList() {
  return invoke<BuiltinOpenListSession>('start_builtin_openlist')
}
