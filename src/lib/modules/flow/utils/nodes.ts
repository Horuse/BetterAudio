import type { Component } from 'svelte';
import type { NodeTypes } from '@xyflow/svelte';
import type { AnyNodeData, NodeKind } from '$lib/modules/pipeline/types';
import InputDevice from '../ui/input/device.svelte';
import OutputDevice from '../ui/output/device.svelte';

// MIME type used during drag-and-drop from the sidebar.
export const DND_MIME = 'application/x-betteraudio-nodekind';

export interface NodeRegistryEntry {
	label: string;
	component: Component<any>;
	defaultData: AnyNodeData;
}

export const registry: Record<NodeKind, NodeRegistryEntry> = {
	input: {
		label: 'Input',
		component: InputDevice,
		defaultData: { deviceId: null }
	},
	output: {
		label: 'Output',
		component: OutputDevice,
		defaultData: { deviceId: null }
	}
};

export const nodeTypes: NodeTypes = Object.fromEntries(
	Object.entries(registry).map(([kind, entry]) => [kind, entry.component])
);

export const kinds: NodeKind[] = Object.keys(registry) as NodeKind[];
