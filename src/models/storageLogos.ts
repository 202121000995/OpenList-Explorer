export interface StorageLogo {
  key: string
  match: RegExp
  label: string
  color: string
}

export const storageLogos: StorageLogo[] = [
  { key: '123pan', match: /123|123pan/i, label: '123', color: '#3b82f6' },
  { key: 'baidu', match: /baidu|百度/i, label: '百', color: '#2f80ed' },
  { key: 'aliyun', match: /aliyun|阿里/i, label: '阿', color: '#6366f1' },
  { key: 'onedrive', match: /onedrive|one drive|1drv/i, label: '1D', color: '#0284c7' },
  { key: 'google-drive', match: /google|drive/i, label: 'G', color: '#22c55e' },
  { key: 'webdav', match: /webdav/i, label: 'W', color: '#64748b' },
  { key: 's3', match: /s3|aws|cos|oss|kodo|对象|bucket/i, label: 'S3', color: '#f59e0b' },
  { key: 'local', match: /local|本地|root/i, label: 'L', color: '#0ea5e9' },
  { key: 'pikpak', match: /pikpak/i, label: 'P', color: '#7c3aed' },
  { key: 'quark', match: /quark|夸克/i, label: '夸', color: '#14b8a6' },
  { key: 'xunlei', match: /xunlei|thunder|迅雷/i, label: '迅', color: '#2563eb' },
  { key: 'dropbox', match: /dropbox/i, label: 'D', color: '#0061ff' },
  { key: 'mega', match: /mega/i, label: 'M', color: '#dc2626' },
  { key: 'ftp', match: /ftp|sftp/i, label: 'FTP', color: '#475569' }
]

export function resolveStorageLogo(name: string) {
  return storageLogos.find((logo) => logo.match.test(name)) ?? {
    key: 'openlist',
    label: name.slice(0, /^\d/.test(name) ? 3 : 2).toUpperCase(),
    color: '#3b82f6'
  }
}
