import type { UnlistenFn } from '@tauri-apps/api/event';
import { methods } from './methods';
import type { AudioApplication, AudioDevice } from './types';

class AudioStore {
	inputDevices = $state<AudioDevice[]>([]);
	outputDevices = $state<AudioDevice[]>([]);
	audioApplications = $state<AudioApplication[]>([]);
	isRunning = $state(false);
	lastError = $state<string | null>(null);

	private unlisten: UnlistenFn | undefined;

	async refreshDevices(): Promise<void> {
		// list_audio_applications can fail on hosts without ScreenCaptureKit access yet —
		// don't make device enumeration fail because of it.
		const [ins, outs, apps] = await Promise.all([
			methods.listInputDevices(),
			methods.listOutputDevices(),
			methods.listAudioApplications().catch(() => [] as AudioApplication[])
		]);
		this.inputDevices = ins;
		this.outputDevices = outs;
		this.audioApplications = apps;
	}

	async init(): Promise<void> {
		await this.refreshDevices();
		this.unlisten = await methods.onState((e) => {
			if (e.kind === 'started') {
				this.isRunning = true;
				this.lastError = null;
			} else if (e.kind === 'stopped') {
				this.isRunning = false;
			} else if (e.kind === 'error') {
				this.isRunning = false;
				this.lastError = e.message;
			}
		});
	}

	destroy(): void {
		this.unlisten?.();
		this.unlisten = undefined;
	}
}

export const audioStore = new AudioStore();
