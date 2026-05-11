<script lang="ts">
	import { Handle, Position, useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { InputNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type InputNodeType = Node<InputNodeData, 'input'>;
	let { id, data }: NodeProps<InputNodeType> = $props();

	const flow = useSvelteFlow();

	function onChange(e: Event) {
		const value = (e.currentTarget as HTMLSelectElement).value || null;
		flow.updateNodeData(id, { deviceId: value });
	}
</script>

<Wrapper label="Input">
	<select
		class="w-full rounded border px-2 py-1 text-sm"
		value={data.deviceId ?? ''}
		onchange={onChange}
	>
		<option value="">— Select microphone —</option>
		{#each audioStore.inputDevices as device (device.id)}
			<option value={device.id}>{device.name}</option>
		{/each}
	</select>
	<Handle type="source" class="handle" position={Position.Right} />
</Wrapper>
