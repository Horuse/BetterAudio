<script lang="ts">
	import { page } from '$app/state';
	import type { Pipeline } from '$lib/modules/pipeline/types';
	import { methods as pipelineMethods } from '$lib/modules/pipeline/methods';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import { ActivationButton } from '$lib/modules/audio/ui';
	import Header from '$lib/components/layout/header.svelte';
	import Flow from '$lib/modules/flow';

	let pipeline = $state<Pipeline | null>(null);
	let notFound = $state(false);

	$effect(() => {
		const id = page.params.id;
		if (!id) {
			notFound = true;
			return;
		}
		(async () => {
			const p = await pipelineMethods.get(id);
			if (!p) {
				notFound = true;
			} else {
				pipeline = p;
			}
		})();
	});

	let nameSaveTimer: ReturnType<typeof setTimeout> | undefined;
	function onNameInput() {
		clearTimeout(nameSaveTimer);
		nameSaveTimer = setTimeout(() => {
			if (!pipeline) return;
			// Merge with the latest editor snapshot when one exists so we don't
			// clobber unsaved node/edge changes; otherwise persist the current
			// pipeline object with the new name.
			const snapshot = pipelineStore.editorActions?.getSnapshot();
			const next = snapshot
				? { ...snapshot, name: pipeline.name }
				: { ...pipeline, updatedAt: Date.now() };
			pipelineStore.save(next);
		}, 500);
	}
</script>

<Header>
	{#snippet left()}
		<div class="flex items-center gap-3">
			<a href="/" class="button-header px-4 text-sm">← Back</a>
			{#if pipeline}
				<input bind:value={pipeline.name} oninput={onNameInput} class="input-base" />
			{/if}
		</div>
	{/snippet}

	{#snippet right()}
		<div class="flex items-center gap-3">
			{#if audioStore.lastError}
				<span class="text-xs text-red-600">{audioStore.lastError}</span>
			{/if}
			{#if pipeline}
				<ActivationButton />
			{/if}
		</div>
	{/snippet}
</Header>

<div class="flex h-[calc(100vh-40px)] w-full">
	{#if notFound}
		<div class="p-8 text-sm text-gray-500">Pipeline not found.</div>
	{:else if pipeline}
		<Flow.ui.Flow {pipeline} />
	{:else}
		<div class="p-8 text-sm text-gray-500">Loading…</div>
	{/if}
</div>
