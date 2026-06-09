import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core'

export const settings = sqliteTable('settings', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  key: text('key').notNull().unique(),
  value: text('value').notNull()
})

export const favorites = sqliteTable('favorites', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  storage: text('storage').notNull(),
  path: text('path').notNull()
})

export const history = sqliteTable('history', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  type: text('type').notNull(),
  path: text('path').notNull(),
  time: integer('time').notNull()
})

export const tasks = sqliteTable('tasks', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  type: text('type').notNull(),
  status: text('status').notNull(),
  progress: integer('progress').notNull(),
  speed: integer('speed').notNull(),
  path: text('path').notNull()
})
