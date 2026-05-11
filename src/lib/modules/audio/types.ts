import type { PipelineEdge, PipelineNode } from '$lib/modules/pipeline/types';

export type DeviceKind = 'input' | 'output';

export interface AudioDevice {
	id: string;
	name: string;
	kind: DeviceKind;
}

export type AudioStateEvent =
	| { kind: 'started' }
	| { kind: 'stopped' }
	| { kind: 'error'; message: string };

export interface StartPipelinePayload {
	nodes: PipelineNode[];
	edges: PipelineEdge[];
}
