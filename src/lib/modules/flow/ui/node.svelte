<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Handle, Position } from '@xyflow/svelte';

	interface Props {
		label: string;
		accent?: 'input' | 'output' | 'effect';
		hasInput?: boolean;
		hasOutput?: boolean;
		children?: Snippet;
	}

	let {
		label,
		accent = 'effect',
		hasInput = false,
		hasOutput = false,
		children
	}: Props = $props();

	const ACCENT_CLASS: Record<'input' | 'output' | 'effect', string> = {
		input: 'border-emerald-500/40',
		output: 'border-sky-500/40',
		effect: 'border-amber-500/40'
	};
</script>

<div
	class={[
		'min-w-44 rounded-xl border-2 bg-neutral-200 p-3 shadow-sm',
		ACCENT_CLASS[accent]
	]}
>
	<div class="mb-2 text-[10px] font-semibold tracking-wider text-neutral-900 uppercase">
		{label}
	</div>
	{@render children?.()}

	{#if hasInput}
		<Handle type="target" class="handle" position={Position.Left} />
	{/if}
	{#if hasOutput}
		<Handle type="source" class="handle" position={Position.Right} />
	{/if}
</div>
