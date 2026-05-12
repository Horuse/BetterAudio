<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { GainNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';
	import Slider from './_slider.svelte';

	type GainNodeType = Node<GainNodeData, 'gain'>;
	let { id, data }: NodeProps<GainNodeType> = $props();

	const flow = useSvelteFlow();

	function set(v: number) {
		flow.updateNodeData(id, { gainDb: v });
	}
</script>

<Wrapper label="Gain" accent="effect" hasInput hasOutput>
	<div class="w-50">
		<Slider
			label="Level"
			value={data.gainDb}
			min={-60}
			max={24}
			step={0.1}
			unit=" dB"
			onChange={set}
		/>
	</div>
</Wrapper>
