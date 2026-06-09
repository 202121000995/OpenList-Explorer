import { createRouter, createWebHistory } from 'vue-router'
import AppLayout from '@/layouts/AppLayout.vue'
import FileManagerView from '@/views/FileManagerView.vue'
import HomeView from '@/views/HomeView.vue'
import OpenListSettingsView from '@/views/OpenListSettingsView.vue'
import SettingsView from '@/views/SettingsView.vue'
import TasksView from '@/views/TasksView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: AppLayout,
      children: [
        { path: '', redirect: { name: 'files' } },
        { path: 'home', name: 'home', component: HomeView },
        { path: 'files', name: 'files', component: FileManagerView },
        { path: 'uploads', name: 'uploads', component: TasksView, props: { type: 'upload' } },
        { path: 'downloads', name: 'downloads', component: TasksView, props: { type: 'download' } },
        { path: 'openlist', name: 'openlist', component: OpenListSettingsView },
        { path: 'settings', name: 'settings', component: SettingsView }
      ]
    }
  ]
})

export default router
