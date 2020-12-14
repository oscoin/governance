import * as svelteStore from "svelte/store";

import * as contract from "./contract";
import * as transaction from "../transaction";
import * as validation from "../validation";

import { Wallet, Account, Status } from "../wallet";
import * as remote from "../remote";
import { BigNumber, BigNumberish, ContractTransaction } from "ethers";

export const store = svelteStore.writable<Pool | null>(null);

export interface Pool {
  data: remote.Store<PoolData>;

  // Get the account that owns this pool. Should be the connected wallet account.
  getAccount: () => Account | undefined;

  // Onboard the user's pool with the intial values
  onboard(
    amountPerBlock: BigNumberish,
    receivers: Receivers,
    initialBalance: BigNumberish
  ): Promise<void>;

  // Update the contribution per block and the list of receivers.
  updateSettings(amountPerBlock: string, receivers: Receivers): Promise<void>;

  // Adds funds to the pool. Returns once the transaction has been
  // included in the chain.
  topUp(value: number): Promise<void>;

  // Withdraw outgoing balance funds to the connected wallet.
  // Returns once the transaction has been included in the chain.
  withdraw(value: number): Promise<void>;

  // Withdraw all the outgoing balance funds to the connected wallet.
  withdrawAll(): Promise<void>;

  // Collect funds the user has received up to now from givers and
  // transfer them to the users account.
  collect(): Promise<void>;

  // Approve the ERC-20 token allowance, which permits
  // the pool to access said type of token from the user.
  approveErc20(): Promise<void>;
}

// The pool settings the user can change and save.
export interface PoolSettings {
  // The total amount to be disbursed to all receivers with each block.
  amountPerBlock: string;
  // The list of addresses that receive funds from the pool.
  receivers: string[];
}

// All the data representing a pool.
export interface PoolData {
  // The remaining balance of this pool.
  balance: number;
  // The total amount to be disbursed to all receivers with each block.
  amountPerBlock: string;
  // The list of addresses that receive funds from the pool.
  receivers: Receivers;
  // Funds that the user can collect from their givers.
  collectableFunds: number;
  // The ERC-20 token allowance. 0 means that the allowance was not
  // granted or that it was fully spent.
  erc20Allowance: BigNumberish;
}

/* Receivers */
export type Receivers = Map<Address, ReceiverStatus>;

export type Address = string;
export type Weight = BigNumberish;

export enum ReceiverStatus {
  // The receiver is being added
  Added = "Added",
  // The receiver is being removed
  Removed = "Removed",
  // The receiver is present already
  Present = "Present",
}

// The type used by the Radicle-Contracts library to express a
// Receiver, i.e., a Address <> Weight Pair.
interface ReceiverWeight {
  receiver: string;
  weight: Weight;
}
export function make(wallet: Wallet): Pool {
  const data = remote.createStore<PoolData>();
  const poolContract = contract.pool(wallet.signer);
  const erc20TokenContract = contract.erc20Token(wallet.signer);
  loadPoolData();

  async function loadPoolData() {
    try {
      const balance = await poolContract.withdrawable();
      const collectableFunds = await poolContract.collectable();
      const amountPerBlock = await poolContract.getAmountPerBlock();
      const contract_receivers = await poolContract.getAllReceivers();
      const receivers = new Map<Address, ReceiverStatus>(
        contract_receivers.map((e: ReceiverWeight) => [
          e.receiver,
          ReceiverStatus.Present,
        ])
      );
      const erc20Allowance = await getErc20Allowance();

      data.success({
        // Handle potential overflow using BN.js
        balance: Number(balance),
        amountPerBlock: amountPerBlock.toString(),
        receivers,
        // Handle potential overflow using BN.js
        collectableFunds: Number(collectableFunds),
        erc20Allowance,
      });
    } catch (error) {
      data.error(error);
    }
  }

  function getAccount(): Account | undefined {
    const w = svelteStore.get(wallet);
    return w.status === Status.Connected ? w.connected.account : undefined;
  }

  async function updateAmountPerBlock(amount: BigNumberish): Promise<void> {
    return poolContract
      .setAmountPerBlock(amount)
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.monthlyContribution(tx, amount));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  async function onboard(
    amountPerBlock: string,
    receivers: Receivers,
    initialBalance: number
  ): Promise<void> {
    return updateAmountPerBlock(amountPerBlock)
      .then(_ => updateReceiverAddresses(receivers))
      .then(_ => topUp(initialBalance))
      .finally(loadPoolData);
  }

  async function updateSettings(
    amountPerBlock: string,
    receivers: Receivers
  ): Promise<void> {
    return updateAmountPerBlock(amountPerBlock)
      .then(_ => updateReceiverAddresses(receivers))
      .finally(loadPoolData);
  }

  async function updateReceiverAddresses(receivers: Receivers): Promise<void> {
    if (receivers.size === 0) return;

    const receiverWeights: ReceiverWeight[] = [...receivers.entries()].map(
      ([address, status]) => {
        return { receiver: address, weight: weightForStatus(status) };
      }
    );

    return poolContract
      .setReceivers(receiverWeights, [])
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.receivers(tx, receivers));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  async function topUp(value: BigNumberish): Promise<void> {
    return poolContract
      .topUp(value, { gasLimit: 200 * 1000 })
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.topUp(tx));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  async function withdraw(amount: BigNumberish): Promise<void> {
    return poolContract
      .withdraw(amount)
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.withdraw(tx, amount));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  async function withdrawAll(): Promise<void> {
    const all = await poolContract.WITHDRAW_ALL();
    return withdraw(all);
  }

  async function collect(): Promise<void> {
    return poolContract
      .collect()
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.collect(tx));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  async function getErc20Allowance(): Promise<BigNumberish> {
    const account = getAccount();
    if (account) {
      return erc20TokenContract.allowance(
        account.address,
        contract.POOL_ADDRESS
      );
    } else {
      return Promise.resolve(0);
    }
  }

  async function approveErc20(): Promise<void> {
    const unlimited = BigNumber.from(1).shl(256).sub(1);
    return erc20TokenContract
      .approve(contract.POOL_ADDRESS, unlimited)
      .then((tx: ContractTransaction) => {
        transaction.add(transaction.erc20Allowance(tx));
        tx.wait();
      })
      .finally(loadPoolData);
  }

  return {
    data,
    getAccount,
    onboard,
    updateSettings,
    topUp,
    withdraw,
    withdrawAll,
    collect,
    approveErc20,
  };
}

/**
 * Stores
 */
export const membersStore = svelteStore.writable("");
export const amountStore = svelteStore.writable("");
export const budgetStore = svelteStore.writable("");

/**
 *
 * Validations
 *
 */

// Patterns
const COMMA_SEPARATED_LIST = /(^[-\w\s]+(?:,[-\w\s]*)*$)?/;

const contraints = {
  // The contraints for a valid input list of members.
  members: {
    format: {
      pattern: COMMA_SEPARATED_LIST,
      message: `Should be a comma-separated list of addresses`,
    },
  },

  // The contraints for a valid monthly contribution.
  monthlyContribution: {
    presence: {
      message: "The amount is required",
      allowEmpty: false,
    },
    numericality: {
      strict: true,
      greaterThan: -1,
    },
  },

  // The contraints for a valid pool top up amount input.
  topUpAmount: {
    presence: {
      message: "The amount is required",
      allowEmpty: false,
    },
    numericality: {
      strict: true,
      greaterThan: 0,
    },
  },
};

export const membersValidationStore: validation.ValidationStore = validation.createValidationStore(
  contraints.members
);

export const monthlyContributionValidationStore = (): validation.ValidationStore => {
  return validation.createValidationStore(contraints.monthlyContribution);
};

// Validate a balance operation, either a 'Top Up' or a 'Cash out'.
// The provided `balance` represents the account balance upon which
// the value being validated will be compared for sufficiency.
export const balanceValidationStore = (
  balance: BigNumberish
): validation.ValidationStore => {
  return validation.createValidationStore(contraints.topUpAmount, [
    {
      promise: amount => Promise.resolve(balance >= amount),
      validationMessage: "Insufficient balance",
    },
  ]);
};

// Check whether this pool is onboarded.
export function isOnboarded(data: PoolData): boolean {
  return (
    data.receivers.size > 0 || data.amountPerBlock !== "0" || data.balance > 0
  );
}

function weightForStatus(status: ReceiverStatus): number {
  return status === ReceiverStatus.Removed ? 0 : 1;
}
