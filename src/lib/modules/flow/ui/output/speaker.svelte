<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { SpeakerNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';
	import { Combobox } from '$lib/modules/form/ui';

	type SpeakerNodeType = Node<SpeakerNodeData, 'speaker'>;
	let { id, data }: NodeProps<SpeakerNodeType> = $props();

	const flow = useSvelteFlow();

	let refreshing = $state(false);

	function setDevice(value: string | null) {
		flow.updateNodeData(id, { deviceId: value });
	}

	async function refresh() {
		refreshing = true;
		try {
			await audioStore.refreshDevices();
		} finally {
			refreshing = false;
		}
	}

	let options = $derived(audioStore.outputDevices.map((d) => ({ value: d.id, label: d.name })));
	let missing = $derived(
		!!data.deviceId && !audioStore.outputDevices.some((d) => d.id === data.deviceId)
	);
</script>

<Wrapper label="Speaker" accent="output" hasInput>
	<div class="flex w-50 flex-col gap-1">
		<div class="flex items-center gap-1">
			<Combobox {options} value={data.deviceId ?? null} placeholder="— Select output —" onChange={setDevice} />
			<button
				type="button"
				class="nodrag nopan flex h-7 w-7 shrink-0 items-center justify-center rounded border border-neutral-400 bg-neutral-100 text-neutral-900 hover:bg-neutral-200 disabled:opacity-50"
				title="Refresh devices"
				disabled={refreshing}
				onclick={refresh}
			>
				<svg viewBox="0 0 16 16" class={['h-3.5 w-3.5', refreshing ? 'animate-spin' : '']} aria-hidden="true">
					<path
						d="M13 8a5 5 0 1 1-1.5-3.5M13 2v3h-3"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					/>
				</svg>
			</button>
		</div>
		{#if missing}
			<span class="text-[10px] text-red-500">Selected device not available</span>
		{/if}
	</div>
</Wrapper>
