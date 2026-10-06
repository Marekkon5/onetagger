import { createRouter, createWebHistory } from 'vue-router';

const CatalogIndexView = () => import(/* webpackChunkName: "catalog-index" */ '../views/CatalogIndexView.vue');
const CatalogDetailView = () => import(/* webpackChunkName: "catalog-detail" */ '../views/CatalogDetailView.vue');
const CuratorLoginView = () => import(/* webpackChunkName: "login" */ '../views/CuratorLoginView.vue');
const ImportView = () => import(/* webpackChunkName: "import" */ '../views/ImportView.vue');
const LocalTaggerView = () => import(/* webpackChunkName: "local-tags" */ '../views/LocalTaggerView.vue');
const ProvidersView = () => import(/* webpackChunkName: "providers" */ '../views/ProvidersView.vue');
const ReviewView = () => import(/* webpackChunkName: "review" */ '../views/ReviewView.vue');

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'library', component: CatalogIndexView },
    { path: '/catalog/:kind', name: 'catalog-index', component: CatalogIndexView },
    // Each index also has a short public path so a shared link reads naturally and
    // does not depend on the internal /catalog/:kind shape.
    { path: '/artists', name: 'artists', component: CatalogIndexView, props: { kind: 'artist' } },
    { path: '/crews', name: 'crews', component: CatalogIndexView, props: { kind: 'crew' } },
    { path: '/labels', name: 'labels', component: CatalogIndexView, props: { kind: 'label' } },
    { path: '/events', name: 'events', component: CatalogIndexView, props: { kind: 'event' } },
    { path: '/catalog/records/:id', name: 'catalog-record', component: CatalogDetailView },
    { path: '/import', name: 'import', component: ImportView },
    { path: '/local-tags', name: 'local-tags', component: LocalTaggerView },
    { path: '/mix/:id', name: 'mix', component: CatalogDetailView },
    { path: '/entity/:id', name: 'entity', component: CatalogDetailView },
    { path: '/event/:id', name: 'event', component: CatalogDetailView },
    { path: '/review', name: 'review', component: ReviewView },
    { path: '/providers', name: 'providers', component: ProvidersView },
    { path: '/login', name: 'login', component: CuratorLoginView },
  ],
});

export default router;
