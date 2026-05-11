<script lang="ts">
	import {
		Background,
		Controls,
		SvelteFlow,
		useSvelteFlow,
		type Connection,
		type Edge as XyEdge,
		type Node as XyNode
	} from '@xyflow/svelte';
	import { createId } from '@paralleldrive/cuid2';
	import { onDestroy, untrack } from 'svelte';
	import type { NodeKind, Pipeline } from '$lib/modules/pipeline/types';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import {
		DND_MIME,
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

	function onConnect(connection: Connection) {
		if (!connection.source || !connection.target) return;
		edges = [...edges, { id: createId(), source: connection.source, target: connection.target }];
	}

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
				data: { ...registry[kind].defaultData }
			}
		];
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

	onDestroy(() => {
		clearTimeout(saveTimer);
		if (pipelineStore.editorActions?.getSnapshot === getSnapshot) {
			pipelineStore.editorActions = null;
		}
	});
</script>

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
			onconnect={onConnect}
			fitView
		>
			<Background />
			<Controls />
		</SvelteFlow>
	</div>
	<Sidebar />
</div>