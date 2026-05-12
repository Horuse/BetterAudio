<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { MicrophoneNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type MicrophoneNodeType = Node<MicrophoneNodeData, 'microphone'>;
	let { id, data }: NodeProps<MicrophoneNodeType> = $props();

	const flow = useSvelteFlow();

	function onChange(e: Event) {
		const value = (e.currentTarget as HTMLSelectElement).value || null;
		flow.updateNodeData(id, { deviceId: value });
	}
</script>

<Wrapper label="Microphone" accent="input" hasOutput>
	<select
		class="nodrag nopan w-full rounded border border-neutral-400 bg-neutral-100 px-2 py-1 text-sm text-neutral-1100"
		value={data.deviceId ?? ''}
		onchange={onChange}
	>
		<option value="">— Select microphone —</option>
		{#each audioStore.inputDevices as device (device.id)}
			<option value={device.id}>{device.name}</option>
		{/each}
	</select>
</Wrapper>
