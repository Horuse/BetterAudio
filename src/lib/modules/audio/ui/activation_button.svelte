<script lang="ts">
	import { methods } from '../methods';
	import { audioStore } from '../stores.svelte';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';

	let busy = $state(false);

	async function toggle() {
		if (busy) return;
		busy = true;
		audioStore.lastError = null;
		try {
			if (audioStore.isRunning) {
				await methods.stopPipeline();
			} else {
				const snapshot = pipelineStore.editorActions?.getSnapshot();
				if (!snapshot) {
					audioStore.lastError = 'No pipeline loaded';
					return;
				}
				await methods.startPipeline({ nodes: snapshot.nodes, edges: snapshot.edges });
			}
		} catch (e) {
			audioStore.lastError = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<button
	class="button-header px-4"
	class:bg-green-600={!audioStore.isRunning}
	class:text-white={!audioStore.isRunning}
	class:bg-red-600={audioStore.isRunning}
	class:hover:bg-green-700={!audioStore.isRunning}
	class:hover:bg-red-700={audioStore.isRunning}
	disabled={busy}
	onclick={toggle}
>
	{#if busy}
		…
	{:else if audioStore.isRunning}
		Stop
	{:else}
		Activate
	{/if}
</button>
