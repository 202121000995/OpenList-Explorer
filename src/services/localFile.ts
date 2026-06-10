import { invoke } from '@tauri-apps/api/core'

export interface LocalDownloadResult {
  path: string
}

export interface LocalUploadSelection {
  path: string
  relativePath: string
  size: number
}

export interface UrlProbeResult {
  ok: boolean
  status: number
  contentType?: string
  contentLength?: number
}

export async function defaultDownloadPath() {
  return invoke<string>('default_download_path')
}

export async function selectUploadFiles(pickDirectory = false) {
  return invoke<LocalUploadSelection[]>('select_upload_files', { pickDirectory })
}

export async function expandUploadPaths(paths: string[]) {
  return invoke<LocalUploadSelection[]>('expand_upload_paths', { paths })
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

export async function downloadWithEngine(id: string, url: string, filename: string, downloadDir?: string, relativePath?: string) {
  return invoke<LocalDownloadResult>('download_with_engine', {
    id,
    url,
    filename,
    relativePath: relativePath || null,
    downloadDir: downloadDir || null
  })
}

export async function uploadWithEngine(
  id: string,
  serverUrl: string,
  token: string,
  localPath: string,
  remotePath: string
) {
  return invoke('upload_with_engine', {
    id,
    serverUrl,
    token,
    localPath,
    remotePath
  })
}

export async function pauseTransferTask(id: string) {
  return invoke('pause_transfer_task', { id })
}

export async function resumeTransferTask(id: string) {
  return invoke('resume_transfer_task', { id })
}

export async function cancelTransferTask(id: string) {
  return invoke('cancel_transfer_task', { id })
}

export async function revealInFolder(path: string) {
  return invoke('reveal_in_folder', { path })
}

export async function probeUrl(url: string) {
  return invoke<UrlProbeResult>('probe_url', { url })
}
