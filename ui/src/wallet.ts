import WalletConnect from "@walletconnect/client";
import * as svelteStore from "svelte/store";
import * as ethers from "ethers";
import * as ethersBytes from "@ethersproject/bytes";
import {
  Deferrable,
  defineReadOnly,
  resolveProperties,
} from "@ethersproject/properties";
import type {
  Provider,
  TransactionRequest,
} from "@ethersproject/abstract-provider";

import * as contract from "../src/funding/contract";
import * as error from "../src/error";
import * as modal from "../src/modal";
import * as path from "../src/path";

export enum Status {
  Connected = "CONNECTED",
  Connecting = "CONNECTING",
  NotConnected = "NOT_CONNECTED",
}

export type State =
  | { status: Status.NotConnected; error?: globalThis.Error }
  | { status: Status.Connecting }
  | { status: Status.Connected; connected: Connected };

export interface Connected {
  account: Account;
}

export interface Account {
  address: string;
  balance: string;
}

export interface Wallet extends svelteStore.Readable<State> {
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  signer: ethers.Signer;
  account(): Account | undefined;
}

export const provider = new ethers.providers.JsonRpcProvider(
  "http://localhost:8545"
);

export function build(): Wallet {
  const stateStore = svelteStore.writable<State>({
    status: Status.NotConnected,
  });

  const qrCodeModal = {
    open: (uri: string, _cb: unknown, _opts?: unknown) => {
      uriStore.set(uri);
      modal.toggle(path.walletQRCode());
    },
    close: async () => {
      // N.B: this is actually called when the connection is established,
      // not when the modal is closed per se.
      modal.hide();
    },
  };

  let walletConnect = new WalletConnect({
    bridge: "https://bridge.walletconnect.org",
    qrcodeModal: qrCodeModal,
  });

  const signer = new WalletConnectSigner(walletConnect, provider);
  const daiTokenContract = contract.daiToken(signer);

  window.ethereumDebug = new EthereumDebug(provider);

  // Connect to a wallet using walletconnect
  async function connect() {
    if (svelteStore.get(stateStore).status !== Status.NotConnected) {
      throw new Error("A wallet is already connected");
    }

    try {
      await walletConnect.connect();
    } catch (e) {
      stateStore.set({ status: Status.NotConnected, error: e });
      error.show({
        code: error.Code.WalletConnectionFailure,
        message: `Failed to connect wallet: ${e
          .toString()
          .replace("Error: ", "")}`,
      });
    }
    await initialize();
  }

  async function disconnect() {
    await walletConnect.killSession();
    // We need to reinitialize `WalletConnect` until this issue is fixed:
    // https://github.com/WalletConnect/walletconnect-monorepo/pull/370
    walletConnect = new WalletConnect({
      bridge: "https://bridge.walletconnect.org",
      qrcodeModal: qrCodeModal,
    });
    signer.walletConnect = walletConnect;
    stateStore.set({ status: Status.NotConnected });
  }

  async function initialize() {
    stateStore.set({ status: Status.Connecting });
    loadAccountData();
  }

  // Load the connected account's data.
  async function loadAccountData() {
    if (!walletConnect.connected) {
      return;
    }

    try {
      const accountAddress = await signer.getAddress();
      const balance = await daiTokenContract.balanceOf(accountAddress);
      const connected = {
        account: {
          address: accountAddress,
          balance: balance.toString(),
        },
      };
      stateStore.set({ status: Status.Connected, connected });
    } catch (error) {
      stateStore.set({ status: Status.NotConnected, error });
    }
  }

  if (walletConnect.connected) {
    initialize();
  }

  // Periodically refresh the wallet data
  const REFRESH_INTERVAL_MILLIS = 3000;
  setInterval(() => {
    loadAccountData();
  }, REFRESH_INTERVAL_MILLIS);

  function account(): Account | undefined {
    const state = svelteStore.get(stateStore);
    if (state.status === Status.Connected) {
      return state.connected.account;
    }

    return undefined;
  }

  return {
    subscribe: stateStore.subscribe,
    connect,
    disconnect,
    signer,
    account,
  };
}

declare global {
  interface Window {
    ethereumDebug: EthereumDebug;
  }
}

class WalletConnectSigner extends ethers.Signer {
  public walletConnect: WalletConnect;
  private _provider: ethers.providers.Provider;

  private sessionUpdateListener = () => {
    return undefined;
  };

  constructor(walletConnect: WalletConnect, provider: Provider) {
    super();
    defineReadOnly(this, "provider", provider);
    this._provider = provider;
    this.walletConnect = walletConnect;
    this.walletConnect.on("session_update", this.sessionUpdateListener);
  }

  async getAddress(): Promise<string> {
    const accountAddress = this.walletConnect.accounts[0];
    if (!accountAddress) {
      throw new Error(
        "The connected wallet has no accounts or there is a connection problem"
      );
    }
    return accountAddress;
  }

  async signMessage(_message: ethers.Bytes | string): Promise<string> {
    throw new Error("not implemented");
  }

  async signTransaction(
    transaction: Deferrable<TransactionRequest>
  ): Promise<string> {
    const tx = await resolveProperties(transaction);
    const from = tx.from || (await this.getAddress());
    const nonce = await this._provider.getTransactionCount(from);
    const signedTx = await this.walletConnect.signTransaction({
      from,
      to: tx.to,
      value: BigNumberToPrimitive(tx.value || 0),
      gasLimit: BigNumberToPrimitive(tx.gasLimit || 200 * 1000),
      gasPrice: BigNumberToPrimitive(tx.gasPrice || 0),
      nonce,
      data: bytesLikeToString(tx.data),
    });
    return signedTx;
  }

  connect(provider: Provider): ethers.Signer {
    return new WalletConnectSigner(this.walletConnect, provider);
  }
}

function BigNumberToPrimitive(
  bn: ethers.BigNumberish | undefined
): string | undefined {
  if (bn === undefined) {
    return undefined;
  } else {
    return ethers.BigNumber.from(bn).toString();
  }
}

function bytesLikeToString(
  bytes: ethersBytes.BytesLike | undefined
): string | undefined {
  if (bytes === undefined) {
    return undefined;
  } else {
    return ethersBytes.hexlify(bytes);
  }
}

class EthereumDebug {
  private provider: ethers.providers.JsonRpcProvider;

  constructor(provider: ethers.providers.JsonRpcProvider) {
    this.provider = provider;
  }
  async mineBlocks(blocks = 1) {
    while (blocks) {
      blocks -= 1;
      await this.provider.send("evm_mine", []);
    }
  }

  async setBlockTime(seconds = 5) {
    await this.provider.send("evm_setTime", [seconds]);
  }

  async increaseTime(seconds = 5) {
    await this.provider.send("evm_increaseTime", [seconds]);
  }
}

// URI store for the URI used to build the connecting QRCode.
export const uriStore = svelteStore.writable<string | undefined>(undefined);

export function formattedBalance(balance: number): string {
  return balance.toLocaleString("us-US");
}

// The wallet singleton
export const wallet = build();
