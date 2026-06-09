import { invoke } from '@tauri-apps/api/core'

export interface LocalDownloadResult {
  path: string
}

export async function defaultDownloadPath() {
  return invoke<string>('default_download_path')
}

export async function downloadToLocal(url: string, filename: string, downloadDir?: string) {
  return invoke<LocalDownloadResult>('download_to_local', {
    url,
    filename,
    downloadDir: downloadDir || null
  })
}

export async function downloadToLocalRelative(url: string, relativePath: string, downloadDir?: string) {
  return invoke<LocalDownloadResult>('download_to_local_relative', {
    url,
    relativePath,
    downloadDir: downloadDir || null
  })
}

export async function revealInFolder(path: string) {
  return invoke('reveal_in_folder', { path })
}
