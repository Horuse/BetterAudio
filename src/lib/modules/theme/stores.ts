import { browser } from '$app/environment';
import { writable } from 'svelte/store';
import { withoutTransition } from '$lib/utils/transition';

const stored = browser ? window.localStorage.getItem('theme') : 'dark';

export const themeStore = writable<string>(stored || 'dark');

themeStore.subscribe((value) => {
	if (!browser) return;
	window.localStorage.setItem('theme', value);
	withoutTransition(() =>
		value === 'dark'
			? document.documentElement.classList.add('dark')
			: document.documentElement.classList.remove('dark')
	);
});
