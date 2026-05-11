<script lang="ts">
	import type { NodeKind } from '$lib/modules/pipeline/types';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import { DND_MIME, kinds, registry } from '../utils/nodes';

	function onDragStart(event: DragEvent, kind: NodeKind) {
		if (!event.dataTransfer) return;
		event.dataTransfer.setData(DND_MIME, kind);
		event.dataTransfer.effectAllowed = 'move';
	}

	function onClickAdd(kind: NodeKind) {
		pipelineStore.editorActions?.addNode(kind);
	}
</script>

<aside class="flex w-64 shrink-0 flex-col gap-3 border-l border-neutral-200 bg-background p-4 dark:border-neutral-800">
	<h2 class="text-xs font-semibold tracking-wide text-neutral-800 uppercase dark:text-neutral-200">
		Nodes
	</h2>
	<ul class="flex flex-col gap-2">
		{#each kinds as kind (kind)}
			<li
				draggable="true"
				ondragstart={(e) => onDragStart(e, kind)}
				class="flex items-center justify-between rounded-xl bg-neutral-100 p-3 dark:bg-neutral-800"
			>
				<span>{registry[kind].label}</span>
				<button
					class="rounded px-2 py-0.5 text-xs text-neutral-800 hover:bg-neutral-200 dark:text-neutral-200 dark:hover:bg-neutral-700"
					onclick={() => onClickAdd(kind)}
					aria-label={`Add ${registry[kind].label}`}
				>
					+
				</button>
			</li>
		{/each}
	</ul>
	<p class="mt-3 text-xs text-neutral-500">Click + to add, or drag onto canvas</p>
</aside>
