import * as api from "./api";
import * as validation from "./validation";

// The possible availability statuses of an Id
export enum Status {
  // Available
  Available = "available",
  // Currently taken by an Org or a User
  Taken = "taken",
  // The id was unregistered by an Org or a User and is no longer claimable
  Retired = "retired",
}

// Check if the given id is available
const isAvailable = (id: string): Promise<boolean> =>
  api.get<string>(`ids/${id}/status`).then(status => status === "available");

// ID validation
const VALID_ID_MATCH_STR = "^[a-z0-9][a-z0-9]+$";
const VALID_ID_MATCH = new RegExp(VALID_ID_MATCH_STR);
const idConstraints = {
  presence: {
    message: `This field is required`,
    allowEmpty: false,
  },
  format: {
    pattern: VALID_ID_MATCH,
    message: `It should match ${VALID_ID_MATCH_STR}`,
  },
};

// Id validation store.
export const idValidationStore = (): validation.ValidationStore =>
  validation.createValidationStore(idConstraints, [
    {
      promise: isAvailable,
      validationMessage: "Sorry, this one is no longer available",
    },
  ]);
