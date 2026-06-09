export function joinPath(base: string, name: string) {
  const cleanBase = base === '/' ? '' : base.replace(/\/+$/, '')
  return `${cleanBase}/${name}`.replace(/\/+/g, '/')
}

export function dirname(path: string) {
  if (!path || path === '/') return '/'
  const parts = path.split('/').filter(Boolean)
  parts.pop()
  return parts.length ? `/${parts.join('/')}` : '/'
}

export function filename(path: string) {
  return path.split('/').filter(Boolean).at(-1) || '/'
}
