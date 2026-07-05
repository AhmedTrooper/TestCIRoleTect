import { createRouter, createWebHashHistory } from 'vue-router';
import HomeTab from '../components/HomeTab.vue';
import SettingsTab from '../components/SettingsTab.vue';
import JobDetailView from '../components/JobDetailView.vue';
import ResumesTab from '../components/ResumesTab.vue';
import ResumeDetailView from '../components/ResumeDetailView.vue';
import CoverLettersTab from '../components/CoverLettersTab.vue';
import CoverLetterDetailView from '../components/CoverLetterDetailView.vue';
import CompilerTab from '../components/CompilerTab.vue';
import AboutTab from '../components/AboutTab.vue';
import DiagramTab from '../components/DiagramTab.vue';

const routes = [
  { 
    path: '/', 
    name: 'Home', 
    component: HomeTab 
  },
  {
    path: '/about',
    name: 'About',
    component: AboutTab
  },
  {
    path: '/compiler',
    name: 'Compiler',
    component: CompilerTab
  },
  {
    path: '/diagrams',
    name: 'Diagrams',
    component: DiagramTab
  },
  {
    path: '/inbox',
    name: 'Inbox',
    component: () => import('../components/InboxTab.vue')
  },
  { 
    path: '/settings', 
    name: 'Settings', 
    component: SettingsTab 
  },
  {
    path: '/resumes',
    name: 'Resumes',
    component: ResumesTab
  },
  {
    path: '/cover-letters',
    name: 'CoverLetters',
    component: CoverLettersTab
  },
  {
    path: '/jobs',
    name: 'Jobs',
    component: () => import('../components/JobsTab.vue')
  },
  {
    path: '/resume/:id',
    name: 'ResumeDetail',
    component: ResumeDetailView,
    props: true
  },
  {
    path: '/cover-letter/:id',
    name: 'CoverLetterDetail',
    component: CoverLetterDetailView,
    props: true
  },
  { 
    path: '/parse', 
    name: 'JobParser', 
    component: () => import('../components/JobParserView.vue') 
  },
  {
    path: '/job/:id', 
    name: 'JobDetail', 
    component: JobDetailView, 
    props: true // Passes the :id as a prop to the component
  },
  {
    path: '/inbox/:id',
    name: 'InboxDetail',
    component: () => import('../components/InboxJobDetailView.vue'),
    props: true
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
