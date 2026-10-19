/// <reference types="vite/client" />

declare const __FITIFACT_HEIC_APPROVED__: boolean;
declare const __FITIFACT_VERSION__: string;

declare module "*fitifact_wasm.js" {
  export default function init(): Promise<void>;
  export function compile_requirements(value: string): string;
  export function compile_constraints(value: string): string;
  export function compile_profile(id: string): string;
  export function image_limits(): string;
  export function inspect_bytes(bytes: Uint8Array): string;
  export function validate_bytes(bytes: Uint8Array, constraints: string): string;
  export function plan_bytes(bytes: Uint8Array, constraints: string): string;
  export function plan_rgba(
    rgba: Uint8Array,
    width: number,
    height: number,
    constraints: string,
  ): WasmRgbaPlan;
  export interface WasmRgbaPlan {
    report_json: string;
    take_preview(): Uint8Array | undefined;
    free(): void;
  }
  export function adapt_bytes(bytes: Uint8Array, constraints: string, options: string): WasmAdapt;
  export function adapt_rgba(
    rgba: Uint8Array,
    width: number,
    height: number,
    constraints: string,
    options: string,
  ): WasmAdapt;
  export interface WasmAdapt {
    report_json: string;
    take_output(): Uint8Array | undefined;
    free(): void;
  }
  export function inspect_media_probe(probe: string): string;
  export function plan_media_probe(probe: string, constraints: string, transforms: string): string;
  export function adapt_media_probe(
    probe: string,
    constraints: string,
    transforms: string,
    output: string,
  ): string;
}

declare module "libheif-js/wasm-bundle";
