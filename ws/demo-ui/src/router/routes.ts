export default [
    {
        path: '/',
        name: 'Welcome',
        component: () => import('../views/WelcomeView.vue'),
    },
    {
        path: '/demo-videos',
        name: 'PrintNanny Vision',
        component: () => import('../views/VideoView.vue'),
    }
]