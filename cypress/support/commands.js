Cypress.Commands.add("nukeCache", async () => {
  console.log("Nuking Cache");
  await fetch("http://localhost:8080/v1/session/cache", { method: "DELETE" });
});

Cypress.Commands.add("nukeCocoState", async () => {
  console.log("Nuking CoCo state");
  await fetch("http://localhost:8080/v1/control/nuke/coco");
});

Cypress.Commands.add("nukeRegistryState", async () => {
  console.log("Nuking Registry state");
  await fetch("http://localhost:8080/v1/control/nuke/registry");
});

Cypress.Commands.add("nukeSessionState", async () => {
  console.log("Nuking Session state");
  await fetch("http://localhost:8080/v1/session", { method: "DELETE" });
});

Cypress.Commands.add("nukeAllState", async () => {
  console.log("Nuking CoCo, Registry and session state");
  try {
    await fetch("http://localhost:8080/v1/session/cache", { method: "DELETE" });
    await fetch("http://localhost:8080/v1/session", { method: "DELETE" });
    await fetch("http://localhost:8080/v1/control/nuke/registry");
    await fetch("http://localhost:8080/v1/control/nuke/coco");
  } catch (error) {
    console.error(error);
  }
});

Cypress.Commands.add("pick", (...ids) => {
  const selectorString = ids.map(id => `[data-cy="${id}"]`).join(" ");
  cy.get(selectorString);
});

Cypress.Commands.add(
  "createProjectWithFixture",
  async (
    name = "platinum",
    description = "Best project ever.",
    defaultBranch = "master",
    fakePeers = []
  ) =>
    await fetch("http://localhost:8080/v1/control/create-project", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        name,
        description,
        defaultBranch,
        fakePeers,
      }),
    })
);

Cypress.Commands.add(
  "registerOrg",
  async (id = "monadic", transactionFee = 111) =>
    await fetch("http://localhost:8080/v1/orgs", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        id,
        transactionFee,
      }),
    })
);

Cypress.Commands.add(
  "registerUser",
  async (handle = "nope", id = "123abcd.git", transactionFee = 222) =>
    await fetch("http://localhost:8080/v1/users", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        handle,
        id,
        transactionFee,
      }),
    })
);

Cypress.Commands.add(
  "registerAlternativeUser",
  async (handle = "anotherUser", transactionFee = 333) =>
    await fetch("http://localhost:8080/v1/control/register-user", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        handle,
        transactionFee,
      }),
    })
);

Cypress.Commands.add(
  "createIdentity",
  async (handle = "secretariat") =>
    await fetch("http://localhost:8080/v1/identities", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        handle,
      }),
    })
);
