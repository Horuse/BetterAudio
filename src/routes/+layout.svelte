<script lang="ts">
	import '../app.css';
	import { onDestroy, onMount } from 'svelte';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';

	let { children } = $props();

	onMount(async () => {
		await Promise.all([audioStore.init(), pipelineStore.refresh()]);
	});

	onDestroy(() => audioStore.destroy());
</script>

<main>
	{@render children()}
</main>
