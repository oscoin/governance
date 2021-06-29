// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import * as zod from "zod";
import type { Fetcher, RequestOptions } from "./fetcher";

export interface Person {
  email: string;
  name: string;
}
const personSchema: zod.Schema<Person> = zod.object({
  email: zod.string(),
  name: zod.string(),
});

export interface CommitHeader {
  author: Person;
  committer: Person;
  committerTime: number;
  description: string;
  sha1: string;
  summary: string;
}

export enum ObjectType {
  Blob = "BLOB",
  Tree = "TREE",
}

interface Info {
  name: string;
  objectType: ObjectType;
  lastCommit: CommitHeader;
}

export interface SourceObject {
  path: string;
  info: Info;
}

const sourceObjectSchema = zod.object({
  path: zod.string(),
  info: zod.object({
    name: zod.string(),
    objectType: zod.enum([ObjectType.Blob, ObjectType.Tree]),
    lastCommit: zod.object({
      author: personSchema,
      committer: personSchema,
      committerTime: zod.number(),
      description: zod.string(),
      sha1: zod.string(),
      summary: zod.string(),
    }),
  }),
});

// See
// https://github.com/radicle-dev/radicle-surf/blob/605e6f40840310c14bfe21d7d8a97ac4204f0ec0/source/src/object/blob.rs#L67-L80
// for the serialization.
export type Blob = SourceObject & BlobContent;

type BlobContent =
  | { binary: false; html: boolean; content: string }
  | { binary: true };

const blobContentSchema: zod.Schema<BlobContent> = zod.union([
  zod.object({
    binary: zod.literal(false),
    html: zod.boolean(),
    content: zod.string(),
  }),
  zod.object({ binary: zod.literal(true) }),
]);

const blobSchema: zod.Schema<Blob> = zod.intersection(
  sourceObjectSchema,
  blobContentSchema
);

export enum RevisionType {
  Branch = "branch",
  Tag = "tag",
  Sha = "sha",
}

export interface Branch {
  type: RevisionType.Branch;
  name: string;
}

export interface Tag {
  type: RevisionType.Tag;
  name: string;
}

export interface Sha {
  type: RevisionType.Sha;
  sha: string;
}

export type RevisionSelector = (Branch | Tag | Sha) & { peerId?: string };

interface BlobGetParams {
  projectUrn: string;
  peerId?: string;
  path: string;
  revision: RevisionSelector;
  highlight?: boolean;
}

export class Client {
  private fetcher: Fetcher;

  constructor(fetcher: Fetcher) {
    this.fetcher = fetcher;
  }

  async blobGet(params: BlobGetParams, options: RequestOptions): Promise<Blob> {
    return this.fetcher.fetchOk(
      {
        method: "GET",
        path: `source/blob/${params.projectUrn}`,
        query: {
          path: params.path,
          peerId: params.peerId,
          revision: { peerId: params.peerId, ...params.revision },
          highlight: params.highlight,
        },
        options,
      },
      blobSchema
    );
  }
}
