<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { ChannelBalanceNodeData } from '$lib/modules/pipeline/types';
	import { methods as audioMethods } from '$lib/modules/audio/methods';
	import Wrapper from '../node.svelte';
	import Slider from './_slider.svelte';

	type ChannelBalanceNodeType = Node<ChannelBalanceNodeData, 'channelBalance'>;
	let { id, data }: NodeProps<ChannelBalanceNodeType> = $props();

	const flow = useSvelteFlow();

	function setLeft(v: number) {
		const patch = { leftGainDb: v };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
	function setRight(v: number) {
		const patch = { rightGainDb: v };
		flow.updateNodeData(id, patch);
		audioMethods.updateEffect(id, patch).catch(() => {});
	}
</script>

<Wrapper label="Channel Balance" accent="effect" hasInput hasOutput>
	<div class="flex w-50 flex-col gap-2">
		<Slider label="Left" value={data.leftGainDb} min={-24} max={24} step={0.1} unit=" dB" onChange={setLeft} />
		<Slider label="Right" value={data.rightGainDb} min={-24} max={24} step={0.1} unit=" dB" onChange={setRight} />
	</div>
</Wrapper>
