export default [
    {
        path: '/',
        name: 'Welcome',
        component: () => import('../views/WelcomeView.vue'),
    },
    {
        path: '/demo-videos',
        name: 'Demo Videos',
        component: () => import('../views/VideoView.vue'),
    },
    {
        path: '/camera',
        name: 'Camera Feed',
        component: () => import('../views/CameraView.vue')
    }
]