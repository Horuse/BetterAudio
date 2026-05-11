import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AudioDevice, AudioStateEvent, StartPipelinePayload } from './types';

const AUDIO_STATE_EVENT = 'audio://state';

export const methods = {
	listInputDevices: (): Promise<AudioDevice[]> => invoke<AudioDevice[]>('list_input_devices'),
	listOutputDevices: (): Promise<AudioDevice[]> => invoke<AudioDevice[]>('list_output_devices'),
	startPipeline: (graph: StartPipelinePayload): Promise<void> =>
		invoke('start_pipeline', { graph }),
	stopPipeline: (): Promise<void> => invoke('stop_pipeline'),
	onState: (cb: (e: AudioStateEvent) => void): Promise<UnlistenFn> =>
		listen<AudioStateEvent>(AUDIO_STATE_EVENT, (evt) => cb(evt.payload))
};
