export type FileType = 'file' | 'folder'

export interface OpenListFileItem {
  name: string
  size: number
  is_dir: boolean
  modified: string
  sign?: string
  thumb?: string
  type?: number
}

export interface ExplorerFileItem {
  id: string
  name: string
  path: string
  type: FileType
  size: number
  modifiedAt: string
  raw: OpenListFileItem
}

export interface FileListRequest {
  path: string
  password?: string
  page?: number
  per_page?: number
  refresh?: boolean
}

export interface FileListResponse {
  content: OpenListFileItem[]
  total: number
  readme?: string
  header?: string
  provider?: string
}

export interface FileActionPayload {
  storage: string
  path: string
}

export interface RenamePayload extends FileActionPayload {
  name: string
}

export interface RemovePayload {
  dir: string
  names: string[]
}

export interface MoveCopyPayload {
  src_dir: string
  dst_dir: string
  names: string[]
}

export interface SearchPayload {
  parent: string
  keywords: string
  scope?: number
  page?: number
  per_page?: number
}
