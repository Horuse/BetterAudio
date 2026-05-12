<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { GainNodeData } from '$lib/modules/pipeline/types';
	import { methods as audioMethods } from '$lib/modules/audio/methods';
	import Wrapper from '../node.svelte';
	import Slider from './_slider.svelte';

	type GainNodeType = Node<GainNodeData, 'gain'>;
	let { id, data }: NodeProps<GainNodeType> = $props();

	const flow = useSvelteFlow();

	function set(v: number) {
		const patch = { gainDb: v };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
</script>

<Wrapper label="Gain" accent="effect" hasInput hasOutput>
	<div class="w-50">
		<Slider
			label="Level"
			value={data.gainDb}
			min={-24}
			max={24}
			step={0.1}
			unit=" dB"
			onChange={set}
		/>
		<p class="mt-1 text-[10px] text-neutral-900">Symmetric ±24 dB · use Mute for full silence</p>
	</div>
</Wrapper>
