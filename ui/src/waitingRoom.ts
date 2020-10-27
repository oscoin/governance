// Client representation of proxy waiting room requests, subscriptions, etc.

export enum Status {
  Created = "created",
  Requested = "requested",
  Found = "found",
  Cloning = "cloning",
  Cloned = "cloned",
  Cancelled = "cancelled",
  Failed = "failed",
  TimedOut = "timedOut",
}

export interface ProjectRequest {
  type: Status;
  urn: string;
}
