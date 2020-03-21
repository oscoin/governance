import hotkeys from "hotkeys-js";
import { get } from "svelte/store";
import { push, pop, location } from "svelte-spa-router";
import * as path from "./path.js";

export const initializeHotkeys = () => {
  hotkeys("shift+d", () => {
    if (path.active(path.designSystemGuide(), get(location))) {
      pop();
    }
    push(path.designSystemGuide());
  });

  // TODO(merle): Remove temporary hotkey to open user registration
  hotkeys("shift+t", () => {
    if (path.active(path.registerUser(), get(location))) {
      pop();
    }
    push(path.registerUser());
  });

  hotkeys("shift+/", () => {
    if (path.active(path.help(), get(location))) {
      pop();
    }
    push(path.help());
  });

  // TODO(sarah): Remove temporary hotkey for identity creation
  hotkeys("shift+i", () => {
    if (path.active(path.createIdentity(), get(location))) {
      pop();
    }
    push(path.createIdentity());
  });

  hotkeys("esc", () => {
    if (
      path.active(path.help(), get(location)) ||
      path.active(path.designSystemGuide(), get(location)) ||
      path.active(path.createProject(), get(location)) ||
      path.active(path.registerProject("**"), get(location), true) ||
      path.active(path.createIdentity(), get(location)) ||
      path.active(path.registerUser(), get(location))
    ) {
      pop();
    }
  });
};
