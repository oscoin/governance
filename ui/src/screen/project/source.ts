import { derived, get, writable } from "svelte/store";
import type { Readable, Writable } from "svelte/store";
import { push } from "svelte-spa-router";

import * as error from "../../error";
import { isExperimental } from "../../ipc";
import type { HorizontalItem } from "../../menu";
import * as path from "../../path";
import type { Project, User } from "../../project";
import * as remote from "../../remote";
import * as source from "../../source";

import IconCommit from "../../../DesignSystem/Primitive/Icon/Commit.svelte";
import IconFile from "../../../DesignSystem/Primitive/Icon/File.svelte";
import IconRevision from "../../../DesignSystem/Primitive/Icon/Revision.svelte";

export enum ViewKind {
  Aborted = "ABORTED",
  Blob = "BLOB",
  Error = "ERROR",
  Root = "ROOT",
}

export interface Aborted {
  kind: ViewKind.Aborted;
}

export interface Blob {
  kind: ViewKind.Blob;
  blob: source.Blob;
}

interface Error {
  kind: ViewKind.Error;
  error: error.Error;
}

export interface Root {
  kind: ViewKind.Root;
  readme: source.Readme | null;
}

export type View = Aborted | Blob | Error | Root;

export interface Code {
  lastCommit: source.CommitHeader;
  path: string;
  view: View;
}

interface Screen {
  code: Writable<Code>;
  history: GrouppedCommitsHistory;
  menuItems: HorizontalItem[];
  mergeRequests: source.MergeRequest[];
  peer: User;
  project: Project;
  revisions: [source.Branch | source.Tag];
  selectedPath: Readable<source.SelectedPath>;
  selectedRevision: source.SelectedRevision;
  tree: Writable<source.Tree>;
}

export interface GrouppedCommitsHistory {
  history: CommitGroup[];
  stats: source.Stats;
}

// A set of commits groupped by time.
interface CommitGroup {
  time: string;
  commits: source.CommitHeader[];
}

const pathStore = writable<source.SelectedPath>({
  request: null,
  selected: "",
});

const screenStore = remote.createStore<Screen>();
export const store = screenStore.readable;

export const fetch = async (project: Project, peer: User): Promise<void> => {
  if (!screenStore.is(remote.Status.Success)) {
    screenStore.loading();
  }

  const fetchTreeRoot = async (
    selectedRevision: source.Revision
  ): Promise<[source.Tree, Code]> => {
    const tree = await source.fetchTree(
      project.urn,
      peer.peerId,
      selectedRevision,
      ""
    );
    const root = await fetchCode(project, peer, selectedRevision, tree, "");
    return [tree, root];
  };

  try {
    const revisions = await source.fetchRevisions(project.urn, peer.peerId);
    const mergeRequests = await source.fetchMergeRequests(project.urn);
    const selectedRevision = defaultRevision(project, revisions);
    const [history, [tree, root]] = await Promise.all([
      source.fetchCommits(project.urn, peer.peerId, selectedRevision),
      fetchTreeRoot(selectedRevision),
    ]);
    const grouppedHistory = groupCommitHistory(history);

    screenStore.success({
      code: writable<Code>(root),
      history: grouppedHistory,
      menuItems: menuItems(project, grouppedHistory, mergeRequests),
      mergeRequests,
      peer,
      project,
      revisions: mapRevisions(revisions),
      selectedPath: derived(pathStore, store => store),
      selectedRevision: {
        request: null,
        selected: selectedRevision,
      },
      tree: writable<source.Tree>(tree),
    });
  } catch (err) {
    screenStore.error(error.fromException(err));
  }
};

export const selectPath = async (path: string): Promise<void> => {
  const screen = get(screenStore);
  const current = get(pathStore);

  if (current.selected !== path && screen.status === remote.Status.Success) {
    const {
      data: {
        peer,
        project,
        selectedRevision: { selected: revision },
        tree,
      },
    } = screen;

    const code = await fetchCode(project, peer, revision, get(tree), path);
    if (code.view.kind !== ViewKind.Aborted) {
      screen.data.code.set(code);
    }
  }
};

export const selectRevision = async (
  revision: source.Branch | source.Tag
): Promise<void> => {
  const screen = get(screenStore);

  if (screen.status === remote.Status.Success) {
    const {
      data: { code, peer, project, selectedRevision: current, tree },
    } = screen;

    if (
      current.selected.type === revision.type &&
      current.selected.name === revision.name
    ) {
      return;
    }

    if (current.request) {
      current.request.abort();
    }

    const request = new AbortController();
    const fetchTreeCode = async (): Promise<[source.Tree, Code]> => {
      const tree = await source.fetchTree(
        project.urn,
        peer.peerId,
        revision,
        "",
        request.signal
      );

      const newCode = await fetchCode(
        project,
        peer,
        revision,
        tree,
        get(pathStore).selected
      );

      return [tree, newCode];
    };

    screenStore.success({
      ...screen.data,
      selectedRevision: {
        request,
        selected: revision,
      },
    });

    try {
      const [history, [newTree, newCode]] = await Promise.all([
        source.fetchCommits(project.urn, peer.peerId, revision),
        fetchTreeCode(),
      ]);
      const grouppedHistory = groupCommitHistory(history);
      code.set(newCode);
      tree.set(newTree);

      screenStore.success({
        ...screen.data,
        history: grouppedHistory,
        menuItems: menuItems(
          project,
          grouppedHistory,
          screen.data.mergeRequests
        ),
        selectedRevision: {
          request: null,
          selected: revision,
        },
      });
    } catch (err) {
      screenStore.error(error.fromException(err));
    }
  }
};

const commitStore = remote.createStore<source.Commit>();
export const commit = commitStore.readable;

export const fetchCommit = async (sha1: string): Promise<void> => {
  const screen = get(screenStore);

  if (screen.status === remote.Status.Success) {
    const {
      data: { project },
    } = screen;

    try {
      commitStore.success(await source.fetchCommit(project.urn, sha1));
    } catch (err) {
      commitStore.error(error.fromException(err));
      error.show({
        code: error.Code.CommitFetchFailure,
        message: "Could not fetch commit",
        source: err,
      });
    }
  }
};

export const selectCommit = (commit: source.CommitHeader): void => {
  const screen = get(screenStore);

  if (screen.status === remote.Status.Success) {
    const {
      data: { project },
    } = screen;

    push(path.projectSourceCommit(project.urn, commit.sha1));
  }
};

export const selectMergeRequest = (mergeRequest: source.MergeRequest): void => {
  const screen = get(screenStore);

  if (screen.status === remote.Status.Success) {
    const {
      data: { project },
    } = screen;

    push(
      path.projectSourceMergeRequest(
        project.urn,
        mergeRequest,
        project.metadata.defaultBranch
      )
    );
  }
};

const defaultRevision = (
  project: Project,
  revisions: source.Revisions
): source.Branch => {
  const projectDefault = revisions.branches.find(
    (branch: source.Branch) => branch.name === project.metadata.defaultBranch
  );
  return projectDefault ? projectDefault : revisions.branches[0];
};

const fetchBlob = async (
  project: Project,
  peer: User,
  revision: source.Revision,
  path: string,
  signal: AbortSignal
): Promise<Code> => {
  const blob = await source.fetchBlob(
    project.urn,
    peer.peerId,
    path,
    revision,
    true,
    signal
  );
  return {
    lastCommit: blob.info.lastCommit,
    path,
    view: {
      kind: ViewKind.Blob,
      blob,
    },
  };
};

const fetchCode = async (
  project: Project,
  peer: User,
  revision: source.Revision,
  tree: source.Tree,
  path: string
): Promise<Code> => {
  const currentPath = get(pathStore);
  if (currentPath.request) {
    currentPath.request.abort();
  }

  const request = new AbortController();
  pathStore.set({ request, selected: path });

  let code: Code;
  try {
    if (path === "") {
      code = await fetchRoot(project, peer, revision, tree, request.signal);
    } else {
      code = await fetchBlob(project, peer, revision, path, request.signal);
    }
  } catch (err) {
    // An in-flight request was aborted, we wait for the next one to arrive.
    if (err.name === "AbortError") {
      code = {
        lastCommit: tree.info.lastCommit,
        path,
        view: {
          kind: ViewKind.Aborted,
        },
      };
    } else {
      code = {
        lastCommit: tree.info.lastCommit,
        path,
        view: {
          kind: ViewKind.Error,
          error: err,
        },
      };
    }
  }

  if (code.view.kind !== ViewKind.Aborted) {
    pathStore.set({ request: null, selected: path });
  }

  return code;
};

const fetchRoot = async (
  project: Project,
  peer: User,
  revision: source.Revision,
  tree: source.Tree,
  signal: AbortSignal
): Promise<Code> => {
  return {
    lastCommit: tree.info.lastCommit,
    path: "",
    view: {
      kind: ViewKind.Root,
      readme: await source.fetchReadme(
        project.urn,
        peer.peerId,
        revision,
        tree,
        signal
      ),
    },
  };
};

const mapRevisions = (
  revisions: source.Revisions
): [source.Branch | source.Tag] => {
  const branches = revisions.branches as [source.Branch | source.Tag];
  const tags = revisions.tags as [source.Branch | source.Tag];
  if (isExperimental()) {
    return branches.concat(tags) as [source.Branch | source.Tag];
  }
  return branches;
};

const mergeRequestCommitsStore = remote.createStore<GrouppedCommitsHistory>();
export const mergeRequestCommits = mergeRequestCommitsStore.readable;

export const fetchMergeRequestCommits = async (
  mergeRequest: source.MergeRequest
): Promise<void> => {
  const screen = get(screenStore);

  if (screen.status === remote.Status.Success) {
    const {
      data: { peer, project },
    } = screen;

    try {
      const baseProjectLatestCommit = (
        await source.fetchCommits(project.urn, peer.peerId, {
          type: source.RevisionType.Branch,
          name: project.metadata.defaultBranch,
        })
      ).history[0];

      const mrCommits = await source.fetchCommits(
        project.urn,
        mergeRequest.peer_id,
        {
          type: source.RevisionType.Sha,
          sha: mergeRequest.commit,
        }
      );

      const nothingNewIdx = baseProjectLatestCommit
        ? mrCommits.history.findIndex(
            ch => ch.sha1 === baseProjectLatestCommit.sha1
          )
        : 0;
      const newCommits = mrCommits.history.slice(
        0,
        nothingNewIdx === -1 ? 0 : nothingNewIdx
      );

      mergeRequestCommitsStore.success({
        history: groupCommits(newCommits),
        stats: { ...mrCommits.stats, commits: newCommits.length },
      });
    } catch (err) {
      mergeRequestCommitsStore.error(error.fromException(err));
      error.show({
        code: error.Code.CommitFetchFailure,
        message: "Could not fetch merge request commits",
        source: err,
      });
    }
  }
};

const menuItems = (
  project: Project,
  history: GrouppedCommitsHistory,
  mergeRequests: source.MergeRequest[]
): HorizontalItem[] => {
  return [
    {
      icon: IconFile,
      title: "Files",
      href: path.projectSourceFiles(project.urn),
      looseActiveStateMatching: true,
    },
    {
      icon: IconCommit,
      title: "Commits",
      counter: history.stats.commits,
      href: path.projectSourceCommits(project.urn),
      looseActiveStateMatching: true,
    },
    {
      icon: IconRevision,
      title: "Merge Requests",
      counter: mergeRequests.length,
      href: path.projectSourceMergeRequests(project.urn),
      looseActiveStateMatching: true,
    },
  ];
};

// Convert a source.CommitsHistory to the UI-friendly `GrouppedCommitsHistory` form.
const groupCommitHistory = (
  history: source.CommitsHistory
): GrouppedCommitsHistory => {
  return { ...history, history: groupCommits(history.history) };
};

export const groupCommits = (commits: source.CommitHeader[]): CommitGroup[] => {
  const grouppedCommits: CommitGroup[] = [];
  let groupDate: Date | undefined = undefined;

  commits = commits.sort((a, b) => {
    if (a.committerTime > b.committerTime) {
      return -1;
    } else if (a.committerTime < b.committerTime) {
      return 1;
    }

    return 0;
  });

  for (const commit of commits) {
    const time = commit.committerTime * 1000;
    const date = new Date(time);
    const isNewDay =
      !grouppedCommits.length ||
      !groupDate ||
      date.getDate() < groupDate.getDate() ||
      date.getMonth() < groupDate.getMonth() ||
      date.getFullYear() < groupDate.getFullYear();

    if (isNewDay) {
      grouppedCommits.push({
        time: formatGroupTime(time),
        commits: [],
      });
      groupDate = date;
    }
    grouppedCommits[grouppedCommits.length - 1].commits.push(commit);
  }
  return grouppedCommits;
};

const formatGroupTime = (t: number): string => {
  return new Date(t).toLocaleDateString("en-US", {
    month: "long",
    weekday: "long",
    day: "numeric",
    year: "numeric",
  });
};
