import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core'

export const settings = sqliteTable('settings', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  key: text('key').notNull().unique(),
  value: text('value').notNull()
})

export const favorites = sqliteTable('favorites', {
  id: text('id').primaryKey(),
  storage: text('storage').notNull(),
  path: text('path').notNull()
})

export const history = sqliteTable('history', {
  id: text('id').primaryKey(),
  type: text('type').notNull(),
  path: text('path').notNull(),
  time: integer('time').notNull()
})

export const tasks = sqliteTable('tasks', {
  id: text('id').primaryKey(),
  type: text('type').notNull(),
  status: text('status').notNull(),
  progress: integer('progress').notNull(),
  speed: integer('speed').notNull(),
  path: text('path').notNull(),
  localPath: text('local_path'),
  remoteId: text('remote_id'),
  remoteUrl: text('remote_url'),
  source: text('source'),
  message: text('message'),
  name: text('name').notNull(),
  createdAt: integer('created_at').notNull(),
  updatedAt: integer('updated_at')
})
