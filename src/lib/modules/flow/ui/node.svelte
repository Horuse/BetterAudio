<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		label: string;
		children?: Snippet;
	}

	let { label, children }: Props = $props();
</script>

<div class="size-32 rounded-xl bg-neutral-300 border border-neutral-500 p-3">
	<div class="mb-2 text-xs font-semibold tracking-wide text-neutral-900 uppercase">
		{label}
	</div>
	{@render children?.()}
</div>
