// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import * as ethers from "ethers";
import * as ethereum from "ui/src/ethereum";
import * as error from "ui/src/error";
import * as svelteStore from "ui/src/svelteStore";
import * as Wallet from "ui/src/wallet";

import {
  Registrar__factory as RegistrarFactory,
  RadicleToken__factory as RadicleTokenFactory,
} from "radicle-contracts/build/contract-bindings/ethers";

function registrarAddress(network: ethereum.Environment): string {
  switch (network) {
    case ethereum.Environment.Local:
      throw new error.Error({
        code: error.Code.FeatureNotAvailableForGivenNetwork,
        message: "ENS Registrar not available on the Local testnet",
      });
    case ethereum.Environment.Rinkeby:
      return "0x80b68878442b6510D768Be1bd88712710B86eAcD";
    case ethereum.Environment.Mainnet:
      return "0x37723287Ae6F34866d82EE623401f92Ec9013154";
  }
}

function registrar() {
  const wallet = svelteStore.get(Wallet.store);
  return RegistrarFactory.connect(
    registrarAddress(wallet.environment),
    wallet.signer
  );
}

export async function isAvailable(name: string): Promise<boolean> {
  const r = registrar();
  return r.available(name);
}

export async function getFee(): Promise<ethers.BigNumber> {
  const r = registrar();
  return await r.registrationFeeRad();
}

export function formatFee(fee: ethers.BigNumber): string {
  return ethers.utils.commify(
    parseFloat(ethers.utils.formatUnits(fee)).toFixed(2)
  );
}

export function deadline(): ethers.BigNumber {
  // Expire one hour from now.
  return ethers.BigNumber.from(Math.floor(Date.now() / 1000)).add(3600);
}

export interface CommitResult {
  tx: ethers.ContractTransaction;
  // The minimum number of blocks that must have passed between a commitment
  // and name registration.
  minimumCommitmentAge: number;
}

export async function commit(
  name: string,
  salt: Uint8Array,
  fee: ethers.BigNumber,
  signature: ethers.Signature,
  deadline: ethers.BigNumber
): Promise<CommitResult> {
  const wallet = svelteStore.get(Wallet.store);
  const ownerAddr = wallet.getAddress();
  if (!ownerAddr) {
    throw new error.Error({
      message: "Wallet not connected",
    });
  }

  const minimumCommitmentAge = (
    await registrar().minCommitmentAge()
  ).toNumber();

  const commitment = createCommitment(name, ownerAddr, salt);

  const tx = await registrar().commitWithPermit(
    commitment,
    ownerAddr.toLowerCase(),
    fee,
    deadline,
    signature.v,
    signature.r,
    signature.s
  );

  return {
    tx,
    minimumCommitmentAge,
  };
}

export async function register(
  name: string,
  salt: Uint8Array
): Promise<ethers.ContractTransaction> {
  const wallet = svelteStore.get(Wallet.store);

  const address = wallet.getAddress();

  if (!address) {
    throw new error.Error({
      message: "Wallet not initialized",
    });
  }

  return await registrar().register(name, address, ethers.BigNumber.from(salt));
}

export async function permitSignature(
  value: ethers.BigNumberish,
  deadline: ethers.BigNumberish
): Promise<ethers.Signature> {
  const wallet = svelteStore.get(Wallet.store);
  const spenderAddr = registrarAddress(wallet.environment);
  const owner = wallet.signer;

  const token = RadicleTokenFactory.connect(
    Wallet.radToken.radTokenAddress(wallet.environment),
    wallet.signer
  );

  const ownerAddr = (await owner.getAddress()).toLowerCase();
  const nonce = await token.nonces(ownerAddr);
  const chainId = (await wallet.provider.getNetwork()).chainId;

  const data = {
    domain: {
      name: await token.name(),
      chainId,
      verifyingContract: token.address,
    },
    primaryType: "Permit",
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "chainId", type: "uint256" },
        { name: "verifyingContract", type: "address" },
      ],
      Permit: [
        { name: "owner", type: "address" },
        { name: "spender", type: "address" },
        { name: "value", type: "uint256" },
        { name: "nonce", type: "uint256" },
        { name: "deadline", type: "uint256" },
      ],
    },
    message: {
      owner: ownerAddr.toLowerCase(),
      spender: spenderAddr.toLowerCase(),
      value: ethers.BigNumber.from(value).toString(),
      nonce: ethers.BigNumber.from(nonce).toString(),
      deadline: ethers.BigNumber.from(deadline).toString(),
    },
  };

  const sig = await owner.signTypedData(ownerAddr, data);

  return ethers.utils.splitSignature(sig);
}

function createCommitment(
  name: string,
  ownerAddress: string,
  salt: Uint8Array
): string {
  const bytes = ethers.utils.concat([
    ethers.utils.toUtf8Bytes(name),
    ownerAddress,
    ethers.BigNumber.from(salt).toHexString(),
  ]);

  return ethers.utils.keccak256(bytes);
}
