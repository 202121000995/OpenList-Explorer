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

export interface LocalAria2Status {
  available: boolean
  running: boolean
  rpc_url: string
  rpc_port: number
  download_dir?: string
  binary_path?: string
  message: string
}

export async function getBuiltinOpenListStatus() {
  return invoke<BuiltinOpenListStatus>('builtin_openlist_status')
}

export async function getLocalAria2Status(rpcPort = 6800, rpcSecret = '') {
  return invoke<LocalAria2Status>('local_aria2_status', { rpcPort, rpcSecret })
}

export interface StartAria2Payload {
  rpcPort: number
  rpcSecret?: string
  downloadDir?: string
  maxConcurrent: number
  split: number
}

export async function startLocalAria2(payload: StartAria2Payload) {
  return invoke<LocalAria2Status>('start_local_aria2', { ...payload })
}

export async function startBuiltinOpenList() {
  return invoke<BuiltinOpenListSession>('start_builtin_openlist')
}

export async function resetBuiltinOpenListAdminPassword() {
  return invoke<string>('reset_builtin_openlist_admin_password')
}

export async function openExternalUrl(url: string) {
  return invoke<void>('open_external_url', { url })
}
