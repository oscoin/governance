import * as apolloCore from "@apollo/client/core";
import * as ethers from "ethers";
import * as multihash from "multihashes";
import * as svelteStore from "svelte/store";

import type * as project from "ui/src/project";

import * as error from "ui/src/error";
import * as ethereum from "ui/src/ethereum";
import * as urn from "ui/src/urn";
import * as wallet from "ui/src/wallet";

const createApolloClient = (uri: string): apolloCore.ApolloClient<unknown> => {
  return new apolloCore.ApolloClient({
    uri,
    cache: new apolloCore.InMemoryCache(),
    defaultOptions: {
      query: {
        fetchPolicy: "no-cache",
      },
    },
  });
};

const gnosisSubgraphClient = (): apolloCore.ApolloClient<unknown> => {
  const walletStore = svelteStore.get(wallet.store);
  let uri;
  switch (walletStore.environment) {
    case ethereum.Environment.Local:
      throw new error.Error({
        code: error.Code.FeatureNotAvailableForGivenNetwork,
        message: "Orgs is not available on the Local testnet.",
      });
    case ethereum.Environment.Rinkeby:
      uri =
        "https://api.thegraph.com/subgraphs/name/radicle-dev/gnosis-safe-rinkeby";
      break;
    case ethereum.Environment.Mainnet:
      uri = "https://api.thegraph.com/subgraphs/name/radicle-dev/gnosis-safe";
      break;
  }

  return createApolloClient(uri);
};

const orgsSubgraphClient = () => {
  const walletStore = svelteStore.get(wallet.store);
  let uri;
  switch (walletStore.environment) {
    case ethereum.Environment.Local:
      throw new error.Error({
        code: error.Code.FeatureNotAvailableForGivenNetwork,
        message: "Orgs is not available on the Local testnet.",
      });
    case ethereum.Environment.Rinkeby:
      uri =
        "https://api.thegraph.com/subgraphs/name/radicle-dev/radicle-orgs-rinkeby";
      break;
    case ethereum.Environment.Mainnet:
      uri = "https://api.thegraph.com/subgraphs/name/radicle-dev/radicle-orgs";
      break;
  }
  return createApolloClient(uri);
};

interface GnosisSafeWallet {
  id: string;
  owners: string[];
}

export interface Org {
  id: string;
  owner: string;
  creator: string;
  timestamp: number;
}

const getGnosisSafeWallets = async (walletOwnerAddress: string) => {
  return await gnosisSubgraphClient().query({
    query: apolloCore.gql`
      query GetGnosisSafeWallets($owners: [String!]!) {
        wallets(where: { owners_contains: $owners }) {
          id
          owners
        }
      }
    `,
    variables: { owners: [walletOwnerAddress] },
  });
};

export const getOrgs = async (walletOwnerAddress: string): Promise<Org[]> => {
  const gnosisSafeWallets: [GnosisSafeWallet] = (
    await getGnosisSafeWallets(walletOwnerAddress)
  ).data.wallets;

  const orgsResponse = await orgsSubgraphClient().query<{
    orgs: Array<{
      id: string;
      owner: string;
      creator: string;
      // This is a UNIX seconds timestamp formatted as a string
      timestamp: string;
    }>;
  }>({
    query: apolloCore.gql`
        query GetOrgs($owners: [String!]!) {
          orgs(where: { owner_in: $owners }) {
            id
            owner
            creator
            timestamp
          }
        }
      `,
    variables: { owners: gnosisSafeWallets.map(owner => owner.id) },
  });

  return orgsResponse.data.orgs.map(org => ({
    ...org,
    timestamp: Number.parseInt(org.timestamp),
  }));
};

export interface MemberResponse {
  threshold: number;
  members: string[];
}

export const getGnosisSafeMembers = async (
  walletAddress: string
): Promise<MemberResponse> => {
  const response = (
    await gnosisSubgraphClient().query({
      query: apolloCore.gql`
        query GetGnosisSafeWallets($id: String!) {
          wallets(where: { id: $id }) {
            owners
            threshold
          }
        }
      `,
      variables: { id: walletAddress },
    })
  ).data.wallets[0];

  return { members: response.owners, threshold: parseInt(response.threshold) };
};

export const getOrgProjectAnchors = async (
  orgAddress: string
): Promise<project.Anchor[]> => {
  const response = (
    await orgsSubgraphClient().query({
      query: apolloCore.gql`
        query GetOrgAnchoredProjects($orgAddress: String!) {
          projects(where: {org: $orgAddress}) {
            id
            anchor {
              id
              objectId
              multihash
            }
          }
        }
      `,
      variables: { orgAddress },
    })
  ).data.projects;

  return response.map(
    (project: {
      id: string;
      anchor: {
        id: string;
        objectId: string;
        multihash: string;
      };
    }) => {
      const decodedProjectId = urn.identitySha1Urn(
        ethers.utils.arrayify(`0x${project.id.slice(26)}`)
      );

      const byteArray = ethers.utils.arrayify(project.anchor.multihash);
      const decodedMultihash = multihash.decode(byteArray);
      const decodedCommitHash = ethers.utils
        .hexlify(decodedMultihash.digest)
        .replace(/^0x/, "");
      const anchor: project.Anchor = {
        type: "confirmed",
        orgAddress,
        transactionId: project.anchor.id,
        projectId: decodedProjectId,
        commitHash: decodedCommitHash,
      };

      return anchor;
    }
  );
};
