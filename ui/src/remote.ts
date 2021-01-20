import { derived, get, writable, Readable } from "svelte/store";

import * as error from "./error";

export enum Status {
  NotAsked = "NOT_ASKED",
  Loading = "LOADING",
  Error = "ERROR",
  Success = "SUCCESS",
}

export type Data<T> =
  | { status: Status.NotAsked }
  | { status: Status.Loading }
  | { status: Status.Success; data: T }
  | { status: Status.Error; error: error.Error };

// Shorthand for casting these states
export type ErrorState = { status: Status.Error; error: Error };
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SuccessState = { status: Status.Success; data: any };

export const is = <T>(data: Data<T>, status: Status): boolean => {
  return data.status === status;
};

// A Store is a typesafe svelte readable store that exposes `updateStatus`
// and `update`. It's like a Writable but it can't be externally `set`, and
// it only accepts data that conforms to the `RemoteData` interface
//
// a Readable store of Remote Data based on type T
export interface Store<T> extends Readable<Data<T>> {
  is: (status: Status) => boolean;
  loading: () => void;
  success: (response: T) => void;
  error: (error: error.Error) => void;
  readable: Readable<Data<T>>;
  // Try and unwrap the underlying store's data value.
  // Returns the data of type T if the store's data
  // is in `SuccessState`, undefined otherwise.
  unwrap: () => T | undefined;
  start: (start: StartStopNotifier<Data<T>>) => void;
  reset: () => void;
}

// We should only be updating in this direction: NotAsked => Loading, Loading -> Success | Error
type UpdateableStatus = Status.Loading | Status.Success | Status.Error;

interface Update<T> {
  (status: Status.Loading): void;
  (status: Status.Success, payload: T): void;
  (status: Status.Error, payload: error.Error): void;
}

declare type Subscriber<T> = (value: T) => void;
declare type Unsubscriber = () => void;
declare type StartStopNotifier<T> = (set: Subscriber<T>) => Unsubscriber | void;

// TODO(sos): add @param docs here, consider making generic type T required
export const createStore = <T>(): Store<T> => {
  let starter: StartStopNotifier<Data<T>> | null;
  const initialState = { status: Status.NotAsked } as Data<T>;
  const internalStore = writable(initialState, set => {
    if (starter) {
      return starter(set);
    } else {
      return () => {
        set({ status: Status.NotAsked });
      };
    }
  });
  // eslint-disable-next-line @typescript-eslint/unbound-method
  const { subscribe, update } = internalStore;

  const updateInternalStore: Update<T> = (
    status: UpdateableStatus,
    payload?: T | error.Error
  ) => {
    let val: Data<T>;
    switch (status) {
      case Status.Loading:
        val = { status: Status.Loading };
        break;
      case Status.Success:
        val = { status: Status.Success, data: payload as T };
        break;
      case Status.Error: {
        val = { status: Status.Error, error: payload as error.Error };
        break;
      }
    }

    update(() => {
      return val;
    });
  };

  const resetInternalStore = () => update(() => ({ status: Status.NotAsked }));

  const is = (status: Status): boolean => {
    return get(internalStore).status === status;
  };

  const unwrap = (): T | undefined => {
    return is(Status.Success)
      ? (get(internalStore) as SuccessState).data
      : undefined;
  };

  return {
    is,
    subscribe,
    success: (response: T): void =>
      updateInternalStore(Status.Success, response),
    loading: (): void => updateInternalStore(Status.Loading),
    error: (err: error.Error): void => {
      if (err.code !== error.Code.RequestAbortError) {
        updateInternalStore(Status.Error, err);
      }
    },
    readable: derived(internalStore, $store => $store),
    unwrap,
    start: (start: StartStopNotifier<Data<T>>): void => {
      starter = start;
    },
    reset: resetInternalStore,
  };
};

export const chain = <I, O>(
  input: Readable<Data<I>>,
  output: Store<O>
): Promise<I> => {
  const promise = new Promise<I>((resolve, reject) => {
    input.subscribe(state => {
      if (state.status === Status.Loading) {
        output.loading();
      }

      if (state.status === Status.Error) {
        output.error(state.error);
        reject(state.error);
      }

      if (state.status === Status.Success) {
        resolve(state.data);
      }
    });
  });

  return promise;
};

export const fetch = <T>(
  store: Store<T>,
  req: Promise<T>,
  filter?: (val: T) => T
): void => {
  if (get(store).status === Status.NotAsked) {
    store.loading();
  }

  req
    .then(val => {
      return filter ? filter(val) : val;
    })
    .then(store.success)
    .catch((err: globalThis.Error) => store.error(error.fromException(err)));
};
