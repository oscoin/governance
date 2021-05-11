import * as svelteStore from "svelte/store";

export { default as Router } from "ui/src/router/Router.svelte";

export type Route =
  | { type: "empty" }
  | { type: "designSystemGuide" }
  | { type: "lock" }
  | { type: "onboarding" }
  | { type: "profile"; activeTab: "projects" | "following" }
  | {
      type: "project";
      activeTab: "files" | "commits" | "commit";
      urn: string;
      commitHash: string;
    }
  | { type: "settings" };

const writableHistory: svelteStore.Writable<Route[]> = svelteStore.writable([]);

export const push = (newRoute: Route): void => {
  writableHistory.update(history => [...history, newRoute]);
};

export const pop = (): void => {
  writableHistory.update(history => history.slice(0, -1));
};

export const routeStore: svelteStore.Readable<Route> = svelteStore.derived(
  writableHistory,
  state => {
    if (state.length === 0) {
      return <Route>{ type: "empty" };
    } else {
      return state.slice(-1)[0];
    }
  }
);
