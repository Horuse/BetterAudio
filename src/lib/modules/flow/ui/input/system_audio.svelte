<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { SystemAudioNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';

	type SystemAudioNodeType = Node<SystemAudioNodeData, 'systemAudio'>;
	let { id, data }: NodeProps<SystemAudioNodeType> = $props();

	const flow = useSvelteFlow();

	function onToggle(e: Event) {
		const checked = (e.currentTarget as HTMLInputElement).checked;
		flow.updateNodeData(id, { excludeCurrentApp: checked });
	}
</script>

<Wrapper label="System Audio" accent="input" hasOutput>
	<p class="mb-2 max-w-50 text-[11px] text-neutral-900">
		Captures all system output via ScreenCaptureKit (macOS 13+). Requires Screen Recording
		permission.
	</p>
	<label class="nodrag nopan flex items-center gap-2 text-xs text-neutral-1000">
		<input
			type="checkbox"
			class="nodrag nopan rounded"
			checked={data.excludeCurrentApp ?? true}
			onchange={onToggle}
		/>
		Exclude this app (avoid feedback)
	</label>
</Wrapper>
