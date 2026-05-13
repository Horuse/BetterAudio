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

</script>

<div
	class={[
		'min-w-32 max-w-80 rounded-2xl border border-neutral-400 bg-neutral-200 p-4 shadow-sm',
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
