export type NodeCategory = 'input' | 'output' | 'effect';

export type NodeKind =
	| 'microphone'
	| 'systemAudio'
	| 'appAudio'
	| 'speaker'
	| 'fileRecording'
	| 'gain'
	| 'mute'
	| 'channelBalance'
	| 'limiter'
	| 'levelMeter';

export interface MicrophoneNodeData extends Record<string, unknown> {
	deviceId: string | null;
}

export interface SystemAudioNodeData extends Record<string, unknown> {
	excludeCurrentApp: boolean;
}

export interface AppAudioNodeData extends Record<string, unknown> {
	bundleId: string | null;
}

export interface SpeakerNodeData extends Record<string, unknown> {
	deviceId: string | null;
}

export interface FileRecordingNodeData extends Record<string, unknown> {
	filePath: string | null;
}

export interface GainNodeData extends Record<string, unknown> {
	gainDb: number;
}

export interface MuteNodeData extends Record<string, unknown> {
	muted: boolean;
}

export interface ChannelBalanceNodeData extends Record<string, unknown> {
	leftGainDb: number;
	rightGainDb: number;
}

export interface LimiterNodeData extends Record<string, unknown> {
	thresholdDb: number;
	driveDb: number;
}

export interface LevelMeterNodeData extends Record<string, unknown> {
	// no params yet — just visualises the live signal
}

export type NodeDataMap = {
	microphone: MicrophoneNodeData;
	systemAudio: SystemAudioNodeData;
	appAudio: AppAudioNodeData;
	speaker: SpeakerNodeData;
	fileRecording: FileRecordingNodeData;
	gain: GainNodeData;
	mute: MuteNodeData;
	channelBalance: ChannelBalanceNodeData;
	limiter: LimiterNodeData;
	levelMeter: LevelMeterNodeData;
};

export type AnyNodeData = NodeDataMap[NodeKind];

export interface PipelineNode<K extends NodeKind = NodeKind> {
	id: string;
	kind: K;
	data: NodeDataMap[K];
	position: { x: number; y: number };
}

export interface PipelineEdge {
	id: string;
	source: string;
	target: string;
}

export interface Pipeline {
	id: string;
	name: string;
	nodes: PipelineNode[];
	edges: PipelineEdge[];
	createdAt: number;
	updatedAt: number;
}
