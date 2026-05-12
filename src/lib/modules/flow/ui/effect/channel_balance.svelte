<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { ChannelBalanceNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';
	import Slider from './_slider.svelte';

	type ChannelBalanceNodeType = Node<ChannelBalanceNodeData, 'channelBalance'>;
	let { id, data }: NodeProps<ChannelBalanceNodeType> = $props();

	const flow = useSvelteFlow();

	function setLeft(v: number) {
		flow.updateNodeData(id, { leftGainDb: v });
	}
	function setRight(v: number) {
		flow.updateNodeData(id, { rightGainDb: v });
	}
</script>

<Wrapper label="Channel Balance" accent="effect" hasInput hasOutput>
	<div class="flex w-50 flex-col gap-2">
		<Slider label="Left" value={data.leftGainDb} min={-60} max={12} step={0.1} unit=" dB" onChange={setLeft} />
		<Slider label="Right" value={data.rightGainDb} min={-60} max={12} step={0.1} unit=" dB" onChange={setRight} />
	</div>
</Wrapper>
