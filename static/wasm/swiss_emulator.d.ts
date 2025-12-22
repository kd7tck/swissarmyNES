/* tslint:disable */
/* eslint-disable */

export class Emulator {
  free(): void;
  [Symbol.dispose](): void;
  get_pixels(): number;
  set_button(player: number, button: number, pressed: boolean): void;
  get_pixels_len(): number;
  set_sample_rate(rate: number): void;
  get_audio_samples(): number;
  clear_audio_samples(): void;
  get_audio_samples_len(): number;
  constructor();
  step(): void;
  reset(): void;
  load_rom(rom_data: Uint8Array): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_emulator_free: (a: number, b: number) => void;
  readonly emulator_clear_audio_samples: (a: number) => void;
  readonly emulator_get_audio_samples: (a: number) => number;
  readonly emulator_get_audio_samples_len: (a: number) => number;
  readonly emulator_get_pixels: (a: number) => number;
  readonly emulator_get_pixels_len: (a: number) => number;
  readonly emulator_load_rom: (a: number, b: number, c: number) => [number, number];
  readonly emulator_new: () => number;
  readonly emulator_reset: (a: number) => void;
  readonly emulator_set_button: (a: number, b: number, c: number, d: number) => void;
  readonly emulator_set_sample_rate: (a: number, b: number) => void;
  readonly emulator_step: (a: number) => [number, number];
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
