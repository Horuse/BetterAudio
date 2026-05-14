<script lang="ts">
	import type { SVGAttributes } from 'svelte/elements';
	let props: SVGAttributes<SVGSVGElement> = $props();
</script>

<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" {...props}>
	<path
		d="M10.73 19.8a.75.75 0 0 0 1.04-1.1l-6.25-5.95h14.73a.75.75 0 0 0 0-1.5H5.52l6.25-5.95a.75.75 0 0 0-1.04-1.1l-7.42 7.08a1 1 0 0 0 0 1.44l7.42 7.07Z"
		fill="currentColor"
	/>
</svg>
