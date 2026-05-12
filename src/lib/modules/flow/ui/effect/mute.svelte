<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { MuteNodeData } from '$lib/modules/pipeline/types';
	import { methods as audioMethods } from '$lib/modules/audio/methods';
	import Wrapper from '../node.svelte';

	type MuteNodeType = Node<MuteNodeData, 'mute'>;
	let { id, data }: NodeProps<MuteNodeType> = $props();

	const flow = useSvelteFlow();

	function toggle() {
		const patch = { muted: !data.muted };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
</script>

<Wrapper label="Mute" accent="effect" hasInput hasOutput>
	<button
		class={[
			'nodrag nopan w-40 rounded-lg border px-3 py-2 text-sm font-medium transition-colors',
			data.muted
				? 'border-red-500/40 bg-red-500/40 text-red-100'
				: 'border-neutral-400 bg-neutral-100 text-neutral-1100 hover:bg-neutral-200'
		]}
		onclick={toggle}
	>
		{data.muted ? 'MUTED' : 'Active'}
	</button>
</Wrapper>
