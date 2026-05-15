<script lang="ts">
	import '../app.css';
	import { onDestroy, onMount } from 'svelte';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import { installErrorHandlers, ui as errorUi } from '$lib/modules/error';
	import { checkForUpdates, ui as updaterUi } from '$lib/modules/updater';
	import { DebugPanel } from '$lib/modules/debug';
	import { loadAppInfo } from '$lib/modules/app_info';

	const isDev = import.meta.env.DEV;

	let { children } = $props();

	onMount(async () => {
		await installErrorHandlers();
		loadAppInfo().catch(() => {});
		await Promise.all([audioStore.init(), pipelineStore.refresh()]);
		checkForUpdates().catch(() => {});
	});

	onDestroy(() => audioStore.destroy());
</script>

<updaterUi.UpdateBanner />

<main>
	{@render children()}
</main>

<errorUi.ErrorModal />

{#if isDev}
	<DebugPanel />
{/if}

