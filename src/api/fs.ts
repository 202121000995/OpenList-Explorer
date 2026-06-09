import { openListHttp } from './http'
import type {
  FileListRequest,
  FileListResponse,
  MoveCopyPayload,
  RemovePayload,
  RenamePayload,
  SearchPayload
} from '@/models/file'

export interface OfflineDownloadPayload {
  path: string
  urls: string[]
  tool: string
  delete_policy: 'delete_on_upload_succeed' | 'delete_never'
}

export interface OpenListTaskItem {
  id?: string
  tid?: string
  name?: string
  state?: string
  status?: string
  progress?: number
  percentage?: number
  speed?: number
  path?: string
  dst?: string
  error?: string
  err?: string
  [key: string]: unknown
}

export const fsApi = {
  list(payload: FileListRequest) {
    return openListHttp.post<unknown, FileListResponse>('/api/fs/list', payload)
  },

  get(path: string) {
    return openListHttp.post<unknown, { raw_url?: string; name?: string; size?: number }>('/api/fs/get', { path })
  },

  search(payload: SearchPayload) {
    return openListHttp.post<unknown, FileListResponse>('/api/fs/search', payload)
  },

  remove(payload: RemovePayload) {
    return openListHttp.post('/api/fs/remove', payload)
  },

  rename(payload: RenamePayload) {
    return openListHttp.post('/api/fs/rename', {
      path: payload.path,
      name: payload.name
    })
  },

  move(payload: MoveCopyPayload) {
    return openListHttp.post('/api/fs/move', payload)
  },

  copy(payload: MoveCopyPayload) {
    return openListHttp.post('/api/fs/copy', payload)
  },

  mkdir(path: string) {
    return openListHttp.post('/api/fs/mkdir', { path })
  },

  upload(path: string, file: File, onUploadProgress?: (progress: number) => void) {
    const formData = new FormData()
    formData.append('file', file)

    return openListHttp.put('/api/fs/form', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
        'File-Path': encodeURI(path)
      },
      onUploadProgress(event) {
        if (!event.total) return
        onUploadProgress?.(Math.round((event.loaded / event.total) * 100))
      }
    })
  },

  offlineDownload(payload: OfflineDownloadPayload) {
    return openListHttp.post('/api/fs/add_offline_download', payload)
  },

  offlineDownloadTools() {
    return openListHttp.get<unknown, string[]>('/api/public/offline_download_tools')
  },

  offlineDownloadUndoneTasks() {
    return openListHttp.get<unknown, OpenListTaskItem[]>('/api/admin/task/offline_download/undone')
  },

  offlineDownloadDoneTasks() {
    return openListHttp.get<unknown, OpenListTaskItem[]>('/api/admin/task/offline_download/done')
  },

  offlineDownloadTaskAction(action: 'cancel' | 'retry' | 'delete', taskId: string) {
    return openListHttp.post(`/api/admin/task/offline_download/${action}`, null, {
      params: { tid: taskId }
    })
  }
}
