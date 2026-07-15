import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
    history: createWebHashHistory(),
    routes: [
        {
            path: "/",
            redirect: "/overlay",
        },
        {
            path: "/overlay",
            component: () => import("@/layouts/Overlay.vue"),
        },
        {
            path: "/settings",
            component: () => import("@/pages/Settings/index.vue"),
        },
    ],
});

export default router;
