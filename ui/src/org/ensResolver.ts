// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import type { TransactionResponse } from "./contract";

import { ethers } from "ethers";
import { ENS__factory as EnsFactory } from "radicle-contracts/build/contract-bindings/ethers";

import * as error from "ui/src/error";
import * as svelteStore from "ui/src/svelteStore";
import * as Wallet from "ui/src/wallet";
import * as ethereum from "ui/src/ethereum";

const resolverAbi = [
  "function multicall(bytes[] calldata data) returns(bytes[] memory results)",
  "function setAddr(bytes32 node, address addr)",
  "function setText(bytes32 node, string calldata key, string calldata value)",
];

export const DOMAIN = "radicle.eth";
export type EnsRecord = { name: string; value: string };

export interface Registration {
  // The fully qualified domain name for the registration.
  domain: string;
  // Address that owns this registration
  owner: string;
  // Address record
  address: string | null;
  url: string | null;
  avatar: string | null;
  twitter: string | null;
  github: string | null;
}

export async function setRecords(
  domain: string,
  records: EnsRecord[]
): Promise<TransactionResponse> {
  const wallet = svelteStore.get(Wallet.store);

  const resolver = await wallet.provider.getResolver(domain);

  // The type definitions of `ethers` are not correct. `getResolver()`
  // can return `null`.
  //
  // See https://github.com/ethers-io/ethers.js/issues/1850
  if (!resolver) {
    throw new error.Error({
      message: "Domain is not registered",
      details: { domain },
    });
  }

  const resolverContract = new ethers.Contract(
    resolver.address,
    resolverAbi,
    wallet.signer
  );
  const node = ethers.utils.namehash(domain);

  const calls = [];
  const iface = new ethers.utils.Interface(resolverAbi);

  for (const record of records) {
    switch (record.name) {
      case "address":
        calls.push(iface.encodeFunctionData("setAddr", [node, record.value]));
        break;
      case "url":
      case "avatar":
        calls.push(
          iface.encodeFunctionData("setText", [node, record.name, record.value])
        );
        break;
      case "github":
      case "twitter":
        calls.push(
          iface.encodeFunctionData("setText", [
            node,
            `com.${record.name}`,
            record.value,
          ])
        );
        break;
      default:
        throw new error.Error({
          message: `Unknown field ${record.name}`,
          details: { record },
        });
    }
  }
  return resolverContract.multicall(calls);
}

export async function getRegistration(
  domain: string
): Promise<Registration | null> {
  const wallet = svelteStore.get(Wallet.store);
  const resolver = await wallet.provider.getResolver(domain);

  // The type definitions of `ethers` are not correct. `getResolver()`
  // can return `null`.
  //
  // See https://github.com/ethers-io/ethers.js/issues/1850
  if (!resolver) {
    return null;
  }

  const owner = await getOwner(domain);

  const meta = await Promise.allSettled([
    resolver.getAddress(),
    resolver.getText("avatar"),
    resolver.getText("url"),
    resolver.getText("com.twitter"),
    resolver.getText("com.github"),
  ]);

  const [address, avatar, url, twitter, github] = meta.map(
    (value: PromiseSettledResult<string>) =>
      value.status === "fulfilled" ? value.value : null
  );

  return {
    domain,
    url,
    avatar,
    owner,
    address,
    twitter,
    github,
  };
}

async function getOwner(name: string): Promise<string> {
  const wallet = svelteStore.get(Wallet.store);
  const ensAddr = ethereum.ensAddress(wallet.environment);

  const registry = EnsFactory.connect(ensAddr, wallet.signer);
  const owner = await registry.owner(ethers.utils.namehash(name));

  return owner;
}
