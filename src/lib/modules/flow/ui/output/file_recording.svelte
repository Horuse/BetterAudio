<script lang="ts">
	import { save } from '@tauri-apps/plugin-dialog';
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import type { FileRecordingNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';

	type FileRecordingNodeType = Node<FileRecordingNodeData, 'fileRecording'>;
	let { id, data }: NodeProps<FileRecordingNodeType> = $props();

	const flow = useSvelteFlow();

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
</script>

<Wrapper label="File Recording" accent="output" hasInput>
	<div class="flex w-50 flex-col gap-1.5">
		<div class="truncate rounded bg-neutral-100 px-2 py-1 text-xs text-neutral-1000" title={data.filePath ?? undefined}>
			{basename(data.filePath)}
		</div>
		<button
			class="button-main primary nodrag nopan py-1 text-xs"
			onclick={chooseFile}
		>
			Choose file…
		</button>
		<span class="text-[10px] text-neutral-900">WAV PCM 32-bit float · stereo</span>
	</div>
</Wrapper>
