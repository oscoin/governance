import * as api from "./api";
import * as event from "./event";
import * as remote from "./remote";

// TYPES
export interface Avatar {
  background: {
    r: number;
    g: number;
    b: number;
  };
  emoji: string;
}

export interface Identity {
  id: string;
  metadata: {
    handle: string;
    displayName?: string;
    avatarUrl?: string;
  };
  registered?: string;
  avatarFallback: Avatar;
}

// STATE
const creationStore = remote.createStore<Identity>();
export const store = creationStore.readable;

// EVENTS
enum Kind {
  Create = "CREATE",
  Fetch = "FETCH",
}

interface Create extends event.Event<Kind> {
  kind: Kind.Create;
  handle: string;
  displayName?: string;
  avatarUrl?: string;
}

type Msg = Create;

interface CreateInput {
  handle: string;
  displayName?: string;
  avatarUrl?: string;
}

function update(msg: Msg): void {
  switch (msg.kind) {
    case Kind.Create:
      creationStore.loading();
      api
        .post<CreateInput, Identity>("identities", {
          handle: msg.handle,
          displayName: msg.displayName,
          avatarUrl: msg.avatarUrl,
        })
        .then((id) => {
          creationStore.success(id);
        })
        .catch(creationStore.error);

      break;
  }
}

export const create = event.create<Kind, Msg>(Kind.Create, update);

// MOCK
export const fallback = {
  id: "cloudhead@123abcd.git",
  metadata: {
    handle: "cloudhead",
    displayName: "Alexis Sellier",
    avatarUrl: "https://avatars1.githubusercontent.com/u/40774",
  },
  avatarFallback: {
    background: {
      r: 122,
      g: 112,
      b: 90,
    },
    emoji: "💡",
  },
};
