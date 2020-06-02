import { DIALOG_SHOWOPENDIALOG } from "../../native/ipc.js";

const withEmptyRepositoryStub = async (callback) => {
  const pwd = await cy.exec("pwd").stdout;
  const emptyDirectoryPath = `${pwd}/fixtures/empty-repo`;

  await cy.exec(`rm -rf ${emptyDirectoryPath}`);
  await cy.exec(`mkdir ${emptyDirectoryPath}`);

  // stub native call and return the directory path to the UI
  const appWindow = await cy.window();
  appWindow.electron = {
    ipcRenderer: {
      invoke: (msg) => {
        if (msg === DIALOG_SHOWOPENDIALOG) {
          return emptyDirectoryPath;
        }
      },
    },
  };

  await callback();

  // clean up the fixture
  await cy.exec(`rm -rf ${emptyDirectoryPath}`);
};

const withPlatinumStub = async (callback) => {
  const pwd = await cy.exec("pwd").result;
  const platinumPath = `${pwd}/fixtures/git-platinum-copy`;

  await cy.exec(`rm -rf ${platinumPath}`);
  await cy.exec(
    `git clone ${pwd}/.git/modules/fixtures/git-platinum ${platinumPath}`
  );

  // stub native call and return the directory path to the UI
  const appWindow = await cy.window();
  appWindow.electron = {
    ipcRenderer: {
      invoke: (msg) => {
        if (msg === DIALOG_SHOWOPENDIALOG) {
          return platinumPath;
        }
      },
    },
  };

  await callback();

  await cy.exec(`rm -rf ${platinumPath}`);
};

beforeEach(() => {
  cy.nukeAllState();
  cy.createIdentity();
  cy.createProjectWithFixture();
  cy.visit("./public/index.html");
});

context("project creation", () => {
  context("project creation modal", () => {
    // TODO(rudolfs): test empty project listing has wording and button

    it("can be opened via the profile context menu and closed by pressing cancel", () => {
      cy.pick("profile-context-menu").click();
      cy.pick("dropdown-menu", "new-project").click();
      cy.pick("page", "create-project").should("exist");
      cy.pick("create-project", "cancel-button").click();
      cy.pick("profile-screen").should("exist");
    });

    it("can be closed by pressing escape key", () => {
      cy.pick("profile-context-menu").click();
      cy.pick("dropdown-menu", "new-project").click();
      cy.pick("page", "create-project").should("exist");
      cy.get("body").type("{esc}");
      cy.pick("profile-screen").should("exist");
    });
  });

  context("validations", () => {
    beforeEach(() => {
      cy.pick("profile-context-menu").click();
      cy.pick("dropdown-menu", "new-project").click();

      // Set up minimal form input to show validations
      cy.pick("page", "name").type("this-name-is-valid");
      cy.pick("page", "new-project").click();
      cy.pick("page", "create-project-button").click();
    });

    afterEach(() => {
      cy.get("body").type("{esc}", { force: true });
    });

    context("name", () => {
      it("prevents the user from creating a project with an invalid name", () => {
        // shows a validation message when name is not present
        cy.pick("page", "name").clear();
        cy.pick("page").contains("Project name is required");

        // shows a validation message when name contains invalid characters
        // spaces are not allowed
        cy.pick("page", "name").type("no spaces");
        cy.pick("page").contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // special characters are disallowed
        cy.pick("page", "name").clear();
        cy.pick("page", "name").type("$bad");
        cy.pick("page").contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // can't start with an underscore
        cy.pick("page", "name").clear();
        cy.pick("page", "name").type("_nein");
        cy.pick("page").contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // can't start with a dash
        cy.pick("page", "name").clear();
        cy.pick("page", "name").type("-nope");
        cy.pick("page").contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );

        // has to be at least two characters long
        cy.pick("page", "name").clear();
        cy.pick("page", "name").type("x");
        cy.pick("page").contains(
          "Project name should match ^[a-z0-9][a-z0-9_-]+$"
        );
      });
    });

    context("new repository", () => {
      it("prevents the user from picking an invalid directory", () => {
        // shows a validation message when new project path is empty
        cy.pick("page", "new-project")
          .contains("Pick a directory for the new project")
          .should("exist");

        withPlatinumStub(() => {
          cy.pick("new-project", "choose-path-button").click();
          cy.pick("create-project-button").click();

          cy.pick("page", "new-project")
            .contains("The directory should be empty")
            .should("exist");
        });
      });
    });

    context("existing repository", () => {
      it("prevents the user from picking an invalid directory", () => {
        cy.pick("page", "existing-project").click();

        // shows a validation message when existing project path is empty
        cy.pick("page", "existing-project")
          .contains("Pick a directory with an existing repository")
          .should("exist");

        withEmptyRepositoryStub(() => {
          cy.pick("existing-project", "choose-path-button").click();

          // shows a validation message when an empty directory is chosen
          cy.pick("page", "existing-project")
            .contains("The directory should contain a git repository")
            .should("exist");
        });

        withPlatinumStub(() => {
          cy.pick("existing-project", "choose-path-button").click();
          cy.pick("create-project-button").click();
          cy.pick("profile").click();

          cy.pick("profile-context-menu").click();
          cy.pick("dropdown-menu", "new-project").click();
          cy.pick("page", "name").type("another-project");
          cy.pick("page", "existing-project").click();
          cy.pick("existing-project", "choose-path-button").click();

          cy.pick("page", "existing-project")
            .contains("This repository is already managed by Radicle")
            .should("exist");
        });
      });
    });

    context("form", () => {
      it("prevents the user from submitting invalid data", () => {
        // shows a validation message when new project path is empty
        cy.pick("page", "new-project")
          .contains("Pick a directory for the new project")
          .should("exist");

        cy.pick("page", "existing-project").click();
        // shows a validation message when existing project path is empty
        cy.pick("page", "existing-project")
          .contains("Pick a directory with an existing repository")
          .should("exist");
      });
    });
  });

  context("happy paths", () => {
    it("creates a new project from an empty directory", () => {
      withEmptyRepositoryStub(() => {
        cy.pick("profile-context-menu").click();
        cy.pick("dropdown-menu", "new-project").click();

        cy.pick("name").type("new-fancy-project");
        cy.pick("description").type("My new fancy project");

        cy.pick("new-project").click();
        cy.pick("new-project", "choose-path-button").click();
        cy.pick("create-project-button").click();

        cy.pick("project-screen", "topbar", "project-avatar").contains(
          "new-fancy-project"
        );

        cy.pick("notification").contains(
          "Project new-fancy-project successfully created"
        );

        cy.pick("profile").click();
        cy.pick("profile-screen", "project-list").contains("new-fancy-project");
        cy.pick("profile-screen", "project-list").contains(
          "My new fancy project"
        );
      });
    });

    it("creates a new project from an existing repository", () => {
      withPlatinumStub(() => {
        cy.pick("profile-context-menu").click();
        cy.pick("dropdown-menu", "new-project").click();

        cy.pick("name").type("git-platinum-copy");
        cy.pick("description").type("Best project");

        cy.pick("existing-project").click();
        cy.pick("existing-project", "choose-path-button").click();
        cy.pick("create-project-button").click();
        cy.pick("project-screen", "topbar", "project-avatar").contains(
          "git-platinum-copy"
        );

        cy.pick("project-screen").contains(
          "This repository is a data source for the Upstream front-end tests"
        );

        cy.pick("notification").contains(
          "Project git-platinum-copy successfully created"
        );

        cy.pick("profile").click();
        cy.pick("profile-screen", "project-list").contains("git-platinum-copy");
        cy.pick("profile-screen", "project-list").contains("Best project");
      });
    });
  });
});
