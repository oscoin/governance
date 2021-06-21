import * as lodash from "lodash";
import * as ethers from "ethers";
import * as multihash from "multihashes";
import { OperationType } from "@gnosis.pm/safe-core-sdk-types";

import type {
  TransactionReceipt,
  TransactionResponse,
} from "@ethersproject/providers";
import type * as project from "ui/src/project";
import type { Org, MemberResponse } from "ui/src/org/theGraphApi";
import * as Safe from "./org/safe";

import * as svelteStore from "ui/src/svelteStore";
import type * as identity from "ui/src/proxy/identity";
import * as ethereum from "ui/src/ethereum";
import * as error from "ui/src/error";
import * as ipc from "ui/src/ipc";
import * as modal from "ui/src/modal";
import * as notification from "ui/src/notification";
import * as proxy from "ui/src/proxy";
import * as router from "ui/src/router";
import * as transaction from "./transaction";
import * as urn from "ui/src/urn";
import * as wallet from "ui/src/wallet";
import { sleep } from "ui/src/sleep";
import { claimsAddress, ClaimsContract } from "./attestation/contract";
import { identitySha1Urn } from "ui/src/urn";

import {
  getOrgs,
  getGnosisSafeMembers,
  getOrgProjectAnchors,
} from "ui/src/org/theGraphApi";

export type { MemberResponse, Org };

import ModalAnchorProject from "ui/Modal/Org/AnchorProject.svelte";

const orgFactoryAbi = [
  "function createOrg(address[], uint256) returns (address)",
  "event OrgCreated(address, address)",
];

const orgAbi = [
  "function owner() view returns (address)",
  "function anchor(bytes32, uint32, bytes)",
];

const orgFactoryAddress = (network: ethereum.Environment): string => {
  switch (network) {
    case ethereum.Environment.Local:
      throw new error.Error({
        code: error.Code.FeatureNotAvailableForGivenNetwork,
        message: "Orgs not available on the Local testnet",
      });
    case ethereum.Environment.Rinkeby:
      return "0xF3D04e874D07d680e8b26332eEae5b9B1c263121";
    case ethereum.Environment.Mainnet:
      return "0xa15bEb4876F20018b6b4A4116B7560c5fcC9336e";
  }
};

const ORG_POLL_INTERVAL_MS = 2000;

// Update the org data for the sidebar store every
// `ORG_POLL_INTERVAL_MS` milliseconds.
const updateOrgsForever = async (): Promise<never> => {
  let showError = true;

  for (;;) {
    const walletStore = svelteStore.get(wallet.store);

    await svelteStore.waitUntil(
      walletStore,
      w => w.status === wallet.Status.Connected
    );

    await fetchOrgs().then(
      () => {
        showError = true;
      },
      err => {
        // We only show the first error that is thrown by
        // `fetchOrgs()`. If the function keeps throwing errors we
        // don’t show them. We reset this behavior after the fetch is
        // successful.
        if (showError) {
          error.show(
            new error.Error({
              code: error.Code.OrgFetchFailed,
              message: `Failed to fetch org data`,
              source: err,
            })
          );
          showError = false;
        }
      }
    );

    await sleep(ORG_POLL_INTERVAL_MS);
  }
};

// Start a background task that continously updates the org data for
// the sidebar.
export const initialize = (): void => {
  updateOrgsForever();
};

export const openOnGnosisSafe = (
  gnosisSafeAddress: string,
  view: "transactions" | "settings"
): void => {
  ipc.openUrl(
    Safe.appUrl(
      svelteStore.get(wallet.store).environment,
      gnosisSafeAddress,
      view
    )
  );
};

export const anchorProject = async (
  orgAddress: string,
  gnosisSafeAddress: string,
  projectUrn: string,
  commitHash: string
): Promise<void> => {
  const walletStore = svelteStore.get(wallet.store);
  const checksummedGnosisSafeAddress =
    ethers.utils.getAddress(gnosisSafeAddress);
  const checksummedOrgAddress = ethers.utils.getAddress(orgAddress);
  const encodedProjectUrn = ethers.utils.zeroPad(
    urn.parseIdentitySha1(projectUrn),
    32
  );
  const encodedCommitHash = multihash.encode(
    ethers.utils.arrayify(`0x${commitHash}`),
    "sha1"
  );

  const orgContract = new ethers.Contract(checksummedGnosisSafeAddress, orgAbi);

  const orgContractInstance = await orgContract.populateTransaction.anchor(
    encodedProjectUrn,
    ethers.constants.Zero,
    encodedCommitHash
  );

  const txData = orgContractInstance.data;
  if (!txData) {
    throw new error.Error({
      code: error.Code.OrgCreateCouldNotGenerateTx,
      message: "Could not generate transaction",
    });
  }

  notification.info({
    message:
      "Waiting for you to confirm the anchor transaction in your connected wallet",
    showIcon: true,
    persist: true,
  });

  await Safe.signAndProposeTransaction(walletStore, gnosisSafeAddress, {
    to: checksummedOrgAddress,
    value: "0",
    data: txData,
    operation: OperationType.Call,
  });

  notification.info({
    message:
      "Your anchored project will appear once the quorum of members have confirmed the transaction",
    showIcon: true,
    actions: [
      {
        label: "View on Gnosis Safe",
        handler: () => {
          openOnGnosisSafe(gnosisSafeAddress, "transactions");
        },
      },
    ],
    persist: true,
  });

  router.push({ type: "org", address: orgAddress, activeTab: "projects" });
};

const parseOrgCreatedReceipt = (receipt: TransactionReceipt): string => {
  const iface = new ethers.utils.Interface(orgFactoryAbi);

  let orgAddress: string | undefined;

  receipt.logs.forEach(log => {
    try {
      const parsed = iface.parseLog(log);

      if (parsed.name === "OrgCreated") {
        orgAddress = parsed.args[0].toLowerCase();
      }
    } catch {
      // Ignore parsing errors.
    }
  });

  if (!orgAddress) {
    throw new error.Error({
      code: error.Code.OrgCreateNotFoundInInterfaceLogs,
      message: "Org not found in interface logs",
    });
  }

  return orgAddress;
};

const submitCreateOrgTx = (
  wallet: wallet.Wallet,
  owner: string
): Promise<TransactionResponse> => {
  const orgFactory = new ethers.Contract(
    orgFactoryAddress(wallet.environment),
    orgFactoryAbi,
    wallet.signer
  );
  return orgFactory.createOrg([owner], 1);
};

// Holds the number of pending org creation transactions
export const pendingOrgs = svelteStore.writable<number>(0);

export const createOrg = async (owner: string): Promise<void> => {
  const walletStore = svelteStore.get(wallet.store);
  notification.info({
    message:
      "Waiting for you to confirm the org creation transaction in your connected wallet",
    showIcon: true,
    persist: true,
  });
  const response = await submitCreateOrgTx(walletStore, owner);
  pendingOrgs.update(x => x + 1);

  transaction.add(transaction.createOrg(response));
  notification.info({
    message: "Org creation transaction confirmed, your org will appear shortly",
    showIcon: true,
  });

  const receipt: TransactionReceipt =
    await walletStore.provider.waitForTransaction(response.hash);

  const orgAddress = parseOrgCreatedReceipt(receipt);

  await svelteStore.waitUntil(orgSidebarStore, orgs => {
    return orgs.some(org => org.id === orgAddress);
  });
  pendingOrgs.update(x => x - 1);

  notification.info({
    message: `Org ${orgAddress} has been created`,
    showIcon: true,
    actions: [
      {
        label: "Go to org",
        handler: () => {
          router.push({
            type: "org",
            address: orgAddress,
            activeTab: "projects",
          });
        },
      },
    ],
  });
  await fetchOrgs();
};

const fetchGnosisSafeAddr = async (
  orgAddress: string,
  provider: ethers.providers.Provider
): Promise<string> => {
  const org = new ethers.Contract(orgAddress, orgAbi, provider);
  const safeAddr: string = await org.owner();

  return safeAddr.toLowerCase();
};

export const orgSidebarStore = svelteStore.writable<Org[]>([]);

const fetchOrgs = async (): Promise<void> => {
  const walletStore = svelteStore.get(wallet.store);
  const w = svelteStore.get(walletStore);

  if (w.status !== wallet.Status.Connected) {
    throw new error.Error({
      code: error.Code.OrgFetchOrgsCalledWithNoWallet,
      message: "Tried to call fetchOrgs while the wallet wasn't connected",
    });
  }

  const orgs = await getOrgs(w.connected.account.address);
  const sortedOrgs = lodash.sortBy(orgs, org => org.timestamp);
  orgSidebarStore.set(sortedOrgs);
};

// Information about an org and the safe that controls it.
interface OrgWithSafe {
  orgAddress: string;
  gnosisSafeAddress: string;
  members: Member[];
  threshold: number;
}

export const fetchOrg = async (orgAddress: string): Promise<OrgWithSafe> => {
  const walletStore = svelteStore.get(wallet.store);
  const gnosisSafeAddress = await fetchGnosisSafeAddr(
    orgAddress,
    walletStore.provider
  );
  const { members, threshold } = await fetchMembers(
    walletStore,
    gnosisSafeAddress
  );
  return { orgAddress, gnosisSafeAddress, members, threshold };
};

interface OrgMembers {
  threshold: number;
  members: Member[];
}

export interface Member {
  ethereumAddress: string;
  identity: identity.RemoteIdentity | undefined;
}

export const fetchMembers = async (
  wallet: wallet.Wallet,
  gnosisSafeAddress: string
): Promise<OrgMembers> => {
  const response: MemberResponse = await getGnosisSafeMembers(
    gnosisSafeAddress
  );

  const contract = new ClaimsContract(
    wallet.signer,
    claimsAddress(wallet.environment)
  );

  const members = await Promise.all(
    response.members.map(async ethereumAddress => {
      const identity = await getClaimedIdentity(contract, ethereumAddress);
      return { ethereumAddress, identity };
    })
  );

  return {
    threshold: response.threshold,
    members,
  };
};

// Return all anchors for the org where the anchoring transactions are
// still pending
const fetchPendingAnchors = async (
  org: OrgWithSafe
): Promise<project.PendingAnchor[]> => {
  const walletStore = svelteStore.get(wallet.store);
  const txs = await Safe.getPendingTransactions(
    walletStore.environment,
    org.gnosisSafeAddress
  );
  const isAnchor = (
    anchor: project.PendingAnchor | undefined
  ): anchor is project.PendingAnchor => !!anchor;

  const pendingAnchors = txs
    .map(tx => {
      if (!tx.data) {
        return;
      }
      const iface = new ethers.utils.Interface(orgAbi);
      const parsedTx = iface.parseTransaction({ data: tx.data });

      if (parsedTx.name === "anchor") {
        const encodedProjectUrn = parsedTx.args[0];
        const encodedCommitHash = parsedTx.args[2];

        const projectId = urn.identitySha1Urn(
          ethers.utils.arrayify(`0x${encodedProjectUrn.slice(26)}`)
        );
        const byteArray = ethers.utils.arrayify(encodedCommitHash);
        const decodedMultihash = multihash.decode(byteArray);
        const decodedCommitHash = ethers.utils
          .hexlify(decodedMultihash.digest)
          .replace(/^0x/, "");
        const anchor: project.Anchor = {
          type: "pending",
          projectId,
          commitHash: decodedCommitHash,
          threshold: org.threshold,
          orgAddress: org.orgAddress,
          confirmations: tx.confirmations ? tx.confirmations.length : 0,
        };
        return anchor;
      }
    })
    .filter<project.PendingAnchor>(isAnchor);

  return pendingAnchors;
};

// Return project information for all anchors of an org. If the project
// of an anchor is not replicated by radicle link we include it in
// `unresolvedAnchors`.
//
// Includes anchors from transactions that have not been confirmed yet.
export const resolveProjectAnchors = async (
  org: OrgWithSafe
): Promise<{
  anchoredProjects: project.Project[];
  unresolvedAnchors: project.Anchor[];
}> => {
  const pendingAnchors = await fetchPendingAnchors(org);
  const confirmedAnchors = await getOrgProjectAnchors(org.orgAddress);
  const anchors: project.Anchor[] = [...pendingAnchors, ...confirmedAnchors];

  const anchoredProjects: project.Project[] = [];
  const unresolvedAnchors: project.Anchor[] = [];

  await Promise.all(
    anchors.map(async anchor => {
      try {
        const project = await proxy.client.project.get(anchor.projectId);
        anchoredProjects.push({ ...project, anchor });
      } catch (error) {
        // TODO: only catch when backend can't find project, reraise other errors
        unresolvedAnchors.push(anchor);
      }
    })
  );

  // Show pending projects first.
  anchoredProjects.sort((a, b) => {
    if (!a.anchor || !b.anchor) {
      return 0;
    }

    if (a.anchor.type === "pending" && b.anchor.type === "pending") {
      return 0;
    } else if (a.anchor.type === "pending" && b.anchor.type === "confirmed") {
      return -1;
    } else {
      return 1;
    }
  });

  return { anchoredProjects, unresolvedAnchors };
};

export interface ProjectOption {
  title: string;
  value: urn.Urn;
}

export const openAnchorProjectModal = async (
  orgAddress: string,
  gnosisSafeAddress: string
): Promise<void> => {
  const [tracked, contributed] = await Promise.all([
    proxy.client.project.listTracked(),
    proxy.client.project.listContributed(),
  ]);
  const allProjects = [...tracked, ...contributed];

  const projects: ProjectOption[] = allProjects.map(project => {
    return { title: project.metadata.name, value: project.urn };
  });

  modal.toggle(ModalAnchorProject, () => {}, {
    projects,
    orgAddress,
    gnosisSafeAddress,
  });
};

export const getProjectCount = async (): Promise<number> => {
  const [tracked, contributed] = await Promise.all([
    proxy.client.project.listTracked(),
    proxy.client.project.listContributed(),
  ]);

  return tracked.length + contributed.length;
};

async function getClaimedIdentity(
  contract: ClaimsContract,
  address: string
): Promise<identity.RemoteIdentity | undefined> {
  const radicleIdBytes = await contract.getClaimed(address);
  if (!radicleIdBytes) {
    return undefined;
  }
  const urn = identitySha1Urn(radicleIdBytes);
  let identity;
  try {
    identity = await proxy.client.remoteIdentityGet(urn);
  } catch (error) {
    if (error instanceof proxy.ResponseError && error.response.status === 404) {
      return undefined;
    }
    throw error;
  }
  // Assert that the identity claims the ethereum address
  const claimed = identity.metadata.ethereum?.address.toLowerCase();
  if (claimed !== address.toLowerCase()) {
    return undefined;
  }
  return identity;
}
