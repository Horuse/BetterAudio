<script lang="ts">
	interface Props {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		unit?: string;
		format?: (v: number) => string;
		onChange: (v: number) => void;
	}

	let {
		label,
		value,
		min,
		max,
		step = 0.1,
		unit = '',
		format,
		onChange
	}: Props = $props();

	function display(v: number): string {
		if (format) return format(v);
		const fixed = step >= 1 ? 0 : 1;
		return `${v.toFixed(fixed)}${unit}`;
	}

	function onInput(e: Event) {
		const next = Number((e.currentTarget as HTMLInputElement).value);
		if (!Number.isNaN(next)) onChange(next);
	}
</script>

<label class="flex flex-col gap-0.5 text-[11px] text-neutral-1000">
	<span class="flex items-baseline justify-between">
		<span>{label}</span>
		<span class="font-mono tabular-nums text-neutral-900">{display(value)}</span>
	</span>
	<input
		type="range"
		class="nodrag nopan nowheel w-full accent-amber-500"
		{min}
		{max}
		{step}
		{value}
		oninput={onInput}
	/>
</label>
