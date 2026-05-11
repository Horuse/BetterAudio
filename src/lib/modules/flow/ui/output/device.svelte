<script lang="ts">
	import { Handle, Position, useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { OutputNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type OutputNodeType = Node<OutputNodeData, 'output'>;
	let { id, data }: NodeProps<OutputNodeType> = $props();

	const flow = useSvelteFlow();

	function onChange(e: Event) {
		const value = (e.currentTarget as HTMLSelectElement).value || null;
		flow.updateNodeData(id, { deviceId: value });
	}
</script>

<Wrapper label="Output">
	<select
		class="w-full rounded border px-2 py-1 text-sm"
		value={data.deviceId ?? ''}
		onchange={onChange}
	>
		<option value="">— Select speaker —</option>
		{#each audioStore.outputDevices as device (device.id)}
			<option value={device.id}>{device.name}</option>
		{/each}
	</select>
	<Handle type="target" class="handle" position={Position.Left} />
</Wrapper>
