before(() => {
  cy.nukeAllState();
  cy.createIdentity();
  cy.createProjectWithFixture();
  cy.visit("./public/index.html");
});

context("project creation", () => {
  context("project creation modal", () => {
    // TODO(rudolfs): test empty project listing has wording and button

    it("can be opened via the profile context menu and closed by pressing cancel", () => {
      cy.get('[data-cy="profile-context-menu"]').click();
      cy.get('[data-cy="dropdown-menu"] [data-cy="new-project"]').click({
        // TODO(rudolfs): remove this once #246 is fixed
        force: true
      });
      cy.get('[data-cy="page"] [data-cy="create-project"]').should("exist");
      cy.get('[data-cy="create-project"] [data-cy="cancel-button"]').click();
      cy.get('[data-cy="profile-screen"]').should("exist");
    });

    it("can be closed by pressing escape key", () => {
      cy.get('[data-cy="profile-context-menu"]').click();
      cy.get('[data-cy="dropdown-menu"] [data-cy="new-project"]').click({
        // TODO(rudolfs): remove this once #246 is fixed
        force: true
      });
      cy.get('[data-cy="page"] [data-cy="create-project"]').should("exist");
      cy.get("body").type("{esc}");
      cy.get('[data-cy="profile-screen"]').should("exist");
    });
  });

  context("validations", () => {
    beforeEach(() => {
      cy.get('[data-cy="profile-context-menu"]').click();
      cy.get('[data-cy="dropdown-menu"] [data-cy="new-project"]').click({
        // TODO(rudolfs): remove this once #246 is fixed
        force: true
      });

      // Set up minimal form input to show validations
      cy.get('[data-cy="page"] [data-cy="name"]').type("this-name-is-valid");
      cy.get('[data-cy="page"] [data-cy="new-project"]').click();
      cy.get('[data-cy="page"] [data-cy="create-project-button"]').click();
    });

    afterEach(() => {
      cy.get("body").type("{esc}", { force: true });
    });

    context("name", () => {
      it("prevents the user from creating a project with an invalid name", () => {
        // shows a validation message when name is not present
        cy.get('[data-cy="page"] [data-cy="name"]').clear();
        cy.get('[data-cy="page"]').contains("Project name is required");

        // shows a validation message when name contains invalid characters
        // spaces are not allowed
        cy.get('[data-cy="page"] [data-cy="name"]').type("no spaces");
        cy.get('[data-cy="page"]').contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // special characters are disallowed
        cy.get('[data-cy="page"] [data-cy="name"]').clear();
        cy.get('[data-cy="page"] [data-cy="name"]').type("$bad");
        cy.get('[data-cy="page"]').contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // can't start with an underscore
        cy.get('[data-cy="page"] [data-cy="name"]').clear();
        cy.get('[data-cy="page"] [data-cy="name"]').type("_nein");
        cy.get('[data-cy="page"]').contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // can't start with a dash
        cy.get('[data-cy="page"] [data-cy="name"]').clear();
        cy.get('[data-cy="page"] [data-cy="name"]').type("-nope");
        cy.get('[data-cy="page"]').contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // has to be at least two characters long
        cy.get('[data-cy="page"] [data-cy="name"]').clear();
        cy.get('[data-cy="page"] [data-cy="name"]').type("x");
        cy.get('[data-cy="page"]').contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );
      });
    });

    context("new repository", () => {
      it("prevents the user from picking an invalid directory", () => {
        // shows a validation message when new project path is empty
        cy.get('[data-cy="page"] [data-cy="new-project"]')
          .contains("Pick a directory for the new project")
          .should("exist");

        // TODO(rudolfs): test non-empty directory validation
      });
    });

    context("existing repository", () => {
      it("prevents the user from picking an invalid directory", () => {
        cy.get('[data-cy="page"] [data-cy="existing-project"]').click();

        // shows a validation message when existing project path is empty
        cy.get('[data-cy="page"] [data-cy="existing-project"]')
          .contains("Pick a directory with an existing repository")
          .should("exist");

        // TODO(rudolfs): test empty directory validation
        // TODO(rudolfs): test existing Radicle directory validation
      });
    });

    context("form", () => {
      it("prevents the user from submitting invalid data", () => {
        // shows a validation message when new project path is empty
        cy.get('[data-cy="page"] [data-cy="new-project"]')
          .contains("Pick a directory for the new project")
          .should("exist");

        cy.get('[data-cy="page"] [data-cy="existing-project"]').click();
        // shows a validation message when existing project path is empty
        cy.get('[data-cy="page"] [data-cy="existing-project"]')
          .contains("Pick a directory with an existing repository")
          .should("exist");
      });
    });
  });
});
