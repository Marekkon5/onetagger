import { createRouter, createWebHashHistory } from 'vue-router';

import Index from '../views/Index.vue';
import AudioFeatures from '../views/AudioFeatures.vue';
import TagEditor from '../views/TagEditor.vue';
import Renamer from '../views/Renamer.vue';

// Required for hot reload, idk why it broke
const AutotaggerStatus = () => import('../views/AutotaggerStatus.vue');
const Autotagger = () => import('../views/Autotagger.vue');
const QuickTag = () => import('../views/QuickTag.vue');

const history = createWebHashHistory();

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
];

const router = createRouter({
    history,
    routes
});

export default router;
