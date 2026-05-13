<script lang="ts">
	import { save } from '@tauri-apps/plugin-dialog';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { FileRecordingNodeData } from '$lib/modules/pipeline/types';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import Wrapper from '../node.svelte';

	type FileRecordingNodeType = Node<FileRecordingNodeData, 'fileRecording'>;
	let { id, data }: NodeProps<FileRecordingNodeType> = $props();

	const flow = useSvelteFlow();

	interface ProgressEvent {
		nodeId: string;
		frames: number;
		sampleRate: number;
		stopped?: boolean;
	}

	let durationSec = $state(0);
	let recording = $state(false);

	let unlisten: UnlistenFn | undefined;
	onMount(async () => {
		unlisten = await listen<ProgressEvent>('audio://recorder_progress', (e) => {
			const p = e.payload;
			if (p.nodeId !== id) return;
			durationSec = p.frames / p.sampleRate;
			recording = !p.stopped;
		});
	});

	// Reset display when the pipeline stops.
	$effect(() => {
		if (!audioStore.isRunning) {
			recording = false;
		}
	});

	onDestroy(() => unlisten?.());

	async function chooseFile() {
		const path = await save({
			title: 'Save recording',
			filters: [{ name: 'WAV (32-bit float)', extensions: ['wav'] }]
		});
		if (path) flow.updateNodeData(id, { filePath: path });
	}

	function basename(p: string | null): string {
		if (!p) return 'No file selected';
		const idx = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
		return idx >= 0 ? p.slice(idx + 1) : p;
	}

	function formatDuration(sec: number): string {
		const minutes = Math.floor(sec / 60);
		const remainder = sec - minutes * 60;
		return `${minutes}:${remainder.toFixed(1).padStart(4, '0')}`;
	}
</script>

<Wrapper label="File Recording" accent="output" hasInput>
	<div class="flex w-50 flex-col gap-1.5">
		<div
			class="truncate rounded bg-neutral-100 px-2 py-1 text-xs text-neutral-1000"
			title={data.filePath ?? undefined}
		>
			{basename(data.filePath)}
		</div>
		<button class="button-main primary nodrag nopan py-1 text-xs" onclick={chooseFile}>
			Choose file…
		</button>
		<div class="flex items-baseline justify-between font-mono text-[11px]">
			<span class={recording ? 'text-red-500' : 'text-neutral-900'}>
				{recording ? '● REC' : '○'}
			</span>
			<span class="text-neutral-1000 tabular-nums">{formatDuration(durationSec)}</span>
		</div>
		<span class="text-[10px] text-neutral-900">WAV PCM 32-bit float · stereo</span>
	</div>
</Wrapper>
