declare module "svelte-spa-router" {
  import type { SvelteComponent } from "svelte";
  import { Readable } from "svelte/store";

  export const location: Readable<string>;
  export function link(node: HTMLElement): void;
  export function pop(): void;
  export function push(path: string): void;

  export default class Router extends SvelteComponent {
    $$prop_def: {
      routes: {
        [path: string]: typeof SvelteComponent;
      };
    };
  }
}
