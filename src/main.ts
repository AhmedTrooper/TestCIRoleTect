import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { router } from './router';
import App from './App.vue';
import VueVirtualScroller from 'vue-virtual-scroller';
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';
import "./style.css";

(async () => {
	const app = createApp(App);
	const pinia = createPinia();

	app.use(pinia);
	app.use(router);
	app.use(VueVirtualScroller);

	app.mount('#app');
})();

// Prevent global pinch-to-zoom (trackpad and touchscreen) so the app feels native
document.addEventListener('wheel', (e) => {
	if (e.ctrlKey) {
		e.preventDefault();
	}
}, { passive: false });

document.addEventListener('touchmove', (e) => {
	if (e.touches.length > 1) {
		e.preventDefault();
	}
}, { passive: false });