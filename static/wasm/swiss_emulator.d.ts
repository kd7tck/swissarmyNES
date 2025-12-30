/* tslint:disable */
/* eslint-disable */

export class CpuState {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  pc: number;
  sp: number;
  acc: number;
  x: number;
  y: number;
  status: number;
  cycles: number;
}

export class Emulator {
  free(): void;
  [Symbol.dispose](): void;
  get_pixels(): number;
  set_button(player: number, button: number, pressed: boolean): void;
  get_oam_data(): number;
  get_palettes(): number;
  get_wram_len(): number;
  get_cpu_state(): CpuState;
  add_breakpoint(addr: number): void;
  get_nametables(): number;
  get_pixels_len(): number;
  set_sample_rate(rate: number): void;
  update_palettes(): void;
  get_oam_data_len(): number;
  get_palettes_len(): number;
  clear_breakpoints(): void;
  get_audio_samples(): number;
  remove_breakpoint(addr: number): void;
  update_nametables(): void;
  get_nametables_len(): number;
  get_pattern_tables(): number;
  clear_audio_samples(): void;
  get_audio_samples_len(): number;
  update_pattern_tables(): void;
  get_pattern_tables_len(): number;
  constructor();
  step(): boolean;
  reset(): void;
  get_wram(): number;
  load_rom(rom_data: Uint8Array): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_cpustate_free: (a: number, b: number) => void;
  readonly __wbg_emulator_free: (a: number, b: number) => void;
  readonly __wbg_get_cpustate_acc: (a: number) => number;
  readonly __wbg_get_cpustate_cycles: (a: number) => number;
  readonly __wbg_get_cpustate_pc: (a: number) => number;
  readonly __wbg_get_cpustate_sp: (a: number) => number;
  readonly __wbg_get_cpustate_status: (a: number) => number;
  readonly __wbg_get_cpustate_x: (a: number) => number;
  readonly __wbg_get_cpustate_y: (a: number) => number;
  readonly __wbg_set_cpustate_acc: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_cycles: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_pc: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_sp: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_status: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_x: (a: number, b: number) => void;
  readonly __wbg_set_cpustate_y: (a: number, b: number) => void;
  readonly emulator_add_breakpoint: (a: number, b: number) => void;
  readonly emulator_clear_audio_samples: (a: number) => void;
  readonly emulator_clear_breakpoints: (a: number) => void;
  readonly emulator_get_audio_samples: (a: number) => number;
  readonly emulator_get_audio_samples_len: (a: number) => number;
  readonly emulator_get_cpu_state: (a: number) => number;
  readonly emulator_get_nametables: (a: number) => number;
  readonly emulator_get_nametables_len: (a: number) => number;
  readonly emulator_get_oam_data: (a: number) => number;
  readonly emulator_get_oam_data_len: (a: number) => number;
  readonly emulator_get_palettes: (a: number) => number;
  readonly emulator_get_palettes_len: (a: number) => number;
  readonly emulator_get_pattern_tables: (a: number) => number;
  readonly emulator_get_pattern_tables_len: (a: number) => number;
  readonly emulator_get_pixels: (a: number) => number;
  readonly emulator_get_pixels_len: (a: number) => number;
  readonly emulator_get_wram: (a: number) => number;
  readonly emulator_get_wram_len: (a: number) => number;
  readonly emulator_load_rom: (a: number, b: number, c: number) => [number, number];
  readonly emulator_new: () => number;
  readonly emulator_remove_breakpoint: (a: number, b: number) => void;
  readonly emulator_reset: (a: number) => void;
  readonly emulator_set_button: (a: number, b: number, c: number, d: number) => void;
  readonly emulator_set_sample_rate: (a: number, b: number) => void;
  readonly emulator_step: (a: number) => [number, number, number];
  readonly emulator_update_nametables: (a: number) => void;
  readonly emulator_update_palettes: (a: number) => void;
  readonly emulator_update_pattern_tables: (a: number) => void;
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
