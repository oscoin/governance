context("user profile", () => {
  before(() => {
    cy.resetProxyState();
    cy.onboardUser("cloudhead");
    cy.createProjectWithFixture("platinum", "Best project ever.", "master", [
      "ele",
      "abbey",
    ]);
  });

  context("visitor view profile page", () => {
    // TODO(sos): unskip when we have a proxy testnet
    it.skip("opens from the peer selector with the correct data", () => {
      // Go to the project source page
      cy.visit("./public/index.html#/profile/projects");
      cy.contains("platinum").click();
      cy.contains("Source").click();

      // Pick a user from the peer selector
      cy.pick("peer-selector").click();
      cy.get(".peer-dropdown").pick("abbey").click();

      cy.pick("header").should("exist");

      // Check for the correct data
      cy.pick("entity-name").contains("abbey");
      cy.pick("project-list").contains("platinum").should("exist");
    });
  });
});
