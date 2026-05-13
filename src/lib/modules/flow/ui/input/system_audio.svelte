<script lang="ts">
	import { useSvelteFlow, type Node, type NodeProps } from '@xyflow/svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import type { SystemAudioNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';

	type SystemAudioNodeType = Node<SystemAudioNodeData, 'systemAudio'>;
	let { id, data }: NodeProps<SystemAudioNodeType> = $props();

	const flow = useSvelteFlow();

	function onToggle(e: Event) {
		const checked = (e.currentTarget as HTMLInputElement).checked;
		flow.updateNodeData(id, { excludeCurrentApp: checked });
	}

	async function openPrivacySettings() {
		try {
			await openUrl('x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture');
		} catch {
			// fall through silently — not all hosts support deep links
		}
	}
</script>

<Wrapper label="System Audio" accent="input" hasOutput>
	<div class="flex w-50 flex-col gap-1.5">
		<p class="text-[11px] text-neutral-900">
			Captures all system output via ScreenCaptureKit (macOS 13+).
		</p>
		<div class="flex items-center justify-between gap-2 rounded border border-amber-300 bg-amber-50 px-2 py-1 text-[10px] text-neutral-1000">
			<span>Needs Screen Recording permission</span>
			<button
				type="button"
				class="nodrag nopan shrink-0 rounded border border-amber-400 bg-amber-100 px-1.5 py-0.5 hover:bg-amber-200"
				onclick={openPrivacySettings}
			>
				Open Settings
			</button>
		</div>
		<label class="nodrag nopan flex items-center gap-2 text-xs text-neutral-1000">
			<input
				type="checkbox"
				class="nodrag nopan rounded"
				checked={data.excludeCurrentApp ?? true}
				onchange={onToggle}
			/>
			Exclude this app (avoid feedback)
		</label>
	</div>
</Wrapper>
