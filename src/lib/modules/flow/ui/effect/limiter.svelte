<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { LimiterNodeData } from '$lib/modules/pipeline/types';
	import { methods as audioMethods } from '$lib/modules/audio/methods';
	import Wrapper from '../node.svelte';
	import Slider from './_slider.svelte';

	type LimiterNodeType = Node<LimiterNodeData, 'limiter'>;
	let { id, data }: NodeProps<LimiterNodeType> = $props();

	const flow = useSvelteFlow();

	function setThreshold(v: number) {
		const patch = { thresholdDb: v };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
	function setDrive(v: number) {
		const patch = { driveDb: v };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
</script>

<Wrapper label="Limiter" accent="effect" hasInput hasOutput>
	<div class="flex w-50 flex-col gap-2">
		<Slider
			label="Ceiling"
			value={data.thresholdDb}
			min={-24}
			max={0}
			step={0.1}
			unit=" dB"
			onChange={setThreshold}
		/>
		<Slider
			label="Drive"
			value={data.driveDb}
			min={0}
			max={24}
			step={0.1}
			unit=" dB"
			onChange={setDrive}
		/>
		<span class="text-[10px] text-neutral-900">Soft (tanh) — saturates, no hard clipping</span>
	</div>
</Wrapper>
