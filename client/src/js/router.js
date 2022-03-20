import Vue from 'vue';
import VueRouter from 'vue-router';

import Index from '../views/Index.vue';
import Autotagger from '../views/Autotagger.vue';
import AutotaggerStatus from '../views/AutotaggerStatus.vue';
import QuickTag from '../views/QuickTag.vue';
import AudioFeatures from '../views/AudioFeatures.vue';
import TagEditor from '../views/TagEditor.vue';
import Renamer from '../views/Renamer.vue';

Vue.use(VueRouter);

const routes = [
  {
    path: '/',
    component: Index
  },
  {
    path: '/autotagger',
    component: Autotagger
  },
  {
    path: '/autotagger/status',
    component: AutotaggerStatus
  },
  {
    path: '/quicktag',
    component: QuickTag
  },
  {
    path: '/audiofeatures',
    component: AudioFeatures
  },
  {
    path: '/audiofeatures/status',
    component: AutotaggerStatus
  },
  {
    path: '/tageditor',
    component: TagEditor
  },
  {
    path: '/renamer',
    component: Renamer
  }
]

const router = new VueRouter({
  routes
});

export default router;
