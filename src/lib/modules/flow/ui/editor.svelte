<script lang="ts">
	import {
		Background,
		Controls,
		SvelteFlow,
		useSvelteFlow,
		type Edge as XyEdge,
		type Node as XyNode
	} from '@xyflow/svelte';
	import { createId } from '@paralleldrive/cuid2';
	import { onDestroy, onMount, untrack } from 'svelte';
	import type { NodeKind, Pipeline } from '$lib/modules/pipeline/types';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import {
		DND_MIME,
		defaultDataFor,
		fromXyEdges,
		fromXyNodes,
		nodeTypes,
		registry,
		toXyEdges,
		toXyNodes
	} from '../utils';
	import Sidebar from './sidebar.svelte';

	let { pipeline }: { pipeline: Pipeline } = $props();

	const flow = useSvelteFlow();

	let nodes = $state.raw<XyNode[]>(untrack(() => toXyNodes(pipeline.nodes)));
	let edges = $state.raw<XyEdge[]>(untrack(() => toXyEdges(pipeline.edges)));

	type ContextMenu =
		| { kind: 'node'; nodeId: string; x: number; y: number }
		| { kind: 'edge'; edgeId: string; x: number; y: number };

	let contextMenu = $state<ContextMenu | null>(null);

	function onDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
	}

	function onDrop(event: DragEvent) {
		event.preventDefault();
		const kind = event.dataTransfer?.getData(DND_MIME) as NodeKind | undefined;
		if (!kind || !(kind in registry)) return;
		const position = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
		addNode(kind, position);
	}

	function addNode(kind: NodeKind, position?: { x: number; y: number }) {
		const fallback = { x: 100 + nodes.length * 40, y: 100 + nodes.length * 40 };
		nodes = [
			...nodes,
			{
				id: createId(),
				type: kind,
				position: position ?? fallback,
				data: defaultDataFor(kind)
			}
		];
	}

	function deleteNode(nodeId: string) {
		nodes = nodes.filter((n) => n.id !== nodeId);
		edges = edges.filter((e) => e.source !== nodeId && e.target !== nodeId);
	}

	function deleteEdge(edgeId: string) {
		edges = edges.filter((e) => e.id !== edgeId);
	}

	function onNodeContextMenu({ node, event }: { node: XyNode; event: MouseEvent }) {
		event.preventDefault();
		contextMenu = { kind: 'node', nodeId: node.id, x: event.clientX, y: event.clientY };
	}

	function onEdgeContextMenu({ edge, event }: { edge: XyEdge; event: MouseEvent }) {
		event.preventDefault();
		contextMenu = { kind: 'edge', edgeId: edge.id, x: event.clientX, y: event.clientY };
	}

	function closeContextMenu() {
		contextMenu = null;
	}

	function onMenuDelete() {
		if (!contextMenu) return;
		if (contextMenu.kind === 'node') deleteNode(contextMenu.nodeId);
		else deleteEdge(contextMenu.edgeId);
		contextMenu = null;
	}

	function getSnapshot(): Pipeline {
		return {
			id: pipeline.id,
			name: pipeline.name,
			createdAt: pipeline.createdAt,
			nodes: fromXyNodes(nodes),
			edges: fromXyEdges(edges),
			updatedAt: Date.now()
		};
	}

	pipelineStore.editorActions = { addNode, getSnapshot };

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		nodes;
		edges;
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			untrack(() => pipelineStore.save(getSnapshot()));
		}, 500);
		return () => clearTimeout(saveTimer);
	});

	// The Tauri WebView (and historic browser behavior) treats Backspace outside
	// of editable fields as "navigate back". XYFlow also defaults `deleteKey` to
	// Backspace, so we explicitly accept Delete too and swallow the default
	// navigation in case the press lands outside the flow.
	function onWindowKeyDown(e: KeyboardEvent) {
		if (e.key !== 'Backspace' && e.key !== 'Delete') return;
		const t = e.target as HTMLElement | null;
		const tag = t?.tagName?.toLowerCase();
		if (tag === 'input' || tag === 'textarea' || tag === 'select' || t?.isContentEditable) {
			return;
		}
		e.preventDefault();
	}

	onMount(() => {
		window.addEventListener('keydown', onWindowKeyDown, { capture: true });
		return () => window.removeEventListener('keydown', onWindowKeyDown, { capture: true });
	});

	onDestroy(() => {
		clearTimeout(saveTimer);
		if (pipelineStore.editorActions?.getSnapshot === getSnapshot) {
			pipelineStore.editorActions = null;
		}
	});
</script>

<svelte:window onclick={closeContextMenu} />

<div class="flex h-full w-full">
	<div
		class="flex-1"
		role="region"
		aria-label="Flow editor"
		ondragover={onDragOver}
		ondrop={onDrop}
	>
		<SvelteFlow
			class="!bg-background"
			bind:nodes
			bind:edges
			{nodeTypes}
			defaultEdgeOptions={{ animated: true }}
			deleteKey={['Delete', 'Backspace']}
			onnodecontextmenu={onNodeContextMenu}
			onedgecontextmenu={onEdgeContextMenu}
			onpaneclick={closeContextMenu}
			fitView
		>
			<Background patternClass="fill-neutral-200"/>
			<Controls />
		</SvelteFlow>
	</div>
	<Sidebar />
</div>

{#if contextMenu}
	<div
		class="fixed z-50 min-w-40 overflow-hidden rounded-lg border border-neutral-400 bg-neutral-100 shadow-lg"
		style="top: {contextMenu.y}px; left: {contextMenu.x}px"
		role="menu"
		onclick={(e) => e.stopPropagation()}
		oncontextmenu={(e) => e.preventDefault()}
		onkeydown={(e) => e.key === 'Escape' && closeContextMenu()}
		tabindex="-1"
	>
		<div class="px-3 py-1.5 text-[10px] tracking-wider text-neutral-900 uppercase">
			{contextMenu.kind === 'node' ? 'Node' : 'Edge'}
		</div>
		<button
			class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-red-700 hover:bg-red-500/15 dark:text-red-300"
			onclick={onMenuDelete}
			role="menuitem"
		>
			Delete
			<span class="ml-auto text-[10px] text-neutral-900">⌫</span>
		</button>
	</div>
{/if}
