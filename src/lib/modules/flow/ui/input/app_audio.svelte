<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { AppAudioNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type AppAudioNodeType = Node<AppAudioNodeData, 'appAudio'>;
	let { id, data }: NodeProps<AppAudioNodeType> = $props();

	const flow = useSvelteFlow();

	function onChange(e: Event) {
		const value = (e.currentTarget as HTMLSelectElement).value || null;
		flow.updateNodeData(id, { bundleId: value });
	}
</script>

<Wrapper label="App Audio" accent="input" hasOutput>
	<p class="mb-2 max-w-50 text-[11px] text-neutral-900">
		Capture audio from a specific running app (ScreenCaptureKit, macOS 13+).
	</p>
	<select
		class="nodrag nopan w-full rounded border border-neutral-400 bg-neutral-100 px-2 py-1 text-sm text-neutral-1100"
		value={data.bundleId ?? ''}
		onchange={onChange}
	>
		<option value="">— Select application —</option>
		{#each audioStore.audioApplications as app (app.bundleId)}
			<option value={app.bundleId}>{app.name}</option>
		{/each}
	</select>
</Wrapper>
