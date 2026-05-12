<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { SpeakerNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type SpeakerNodeType = Node<SpeakerNodeData, 'speaker'>;
	let { id, data }: NodeProps<SpeakerNodeType> = $props();

	const flow = useSvelteFlow();

	function onChange(e: Event) {
		const value = (e.currentTarget as HTMLSelectElement).value || null;
		flow.updateNodeData(id, { deviceId: value });
	}
</script>

<Wrapper label="Speaker" accent="output" hasInput>
	<select
		class="nodrag nopan w-full rounded border border-neutral-400 bg-neutral-100 px-2 py-1 text-sm text-neutral-1100"
		value={data.deviceId ?? ''}
		onchange={onChange}
	>
		<option value="">— Select output —</option>
		{#each audioStore.outputDevices as device (device.id)}
			<option value={device.id}>{device.name}</option>
		{/each}
	</select>
</Wrapper>
