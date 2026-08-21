/// <reference types="vite/client" />

declare const __FITIFACT_HEIC_APPROVED__: boolean;

declare module "*fitifact_wasm.js" {
  export default function init(): Promise<void>;
  export function compile_requirements(value: string): string;
  export function compile_constraints(value: string): string;
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
}

declare module "libheif-js/wasm-bundle";
