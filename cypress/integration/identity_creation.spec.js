context("identity creation", () => {
  const validUser = {
    handle: "rafalca",
    shareableEntityIdentifier: "rafalca@123abcd.git",
    fallbackAvatar: "🎀",
  };

  beforeEach(() => {
    cy.nukeSessionState();
    cy.nukeRegistryState();
    cy.visit("./public/index.html");
  });

  context("modal", () => {
    it("can't be closed by pressing escape key", () => {
      cy.pick("get-started-button").should("exist");
      cy.get("body").type("{esc}");
      cy.pick("get-started-button").should("exist");
    });
  });

  context("navigation", () => {
    it("is possible to step through the identity creation flow", () => {
      // Intro screen
      cy.pick("get-started-button").click();

      // Enter details screen
      cy.pick("form", "handle").type(validUser.handle);
      cy.pick("create-id-button").click();

      // Confirmation screen
      cy.get(
        `[data-cy="identity-card"] img[alt=${validUser.fallbackAvatar}]`
      ).should("exist");
      cy.pick("identity-card")
        .contains(validUser.shareableEntityIdentifier)
        .should("exist");

      // Land on profile screen
      cy.pick("go-to-profile-button").click();
      cy.pick("profile-avatar").contains(validUser.handle);
    });

    it("is possible to directly register your identity after creating it", () => {
      cy.pick("get-started-button").click();

      cy.pick("form", "handle").type(validUser.handle);
      cy.pick("create-id-button").click();
      cy.pick("register-identity-link").click();

      cy.contains("Register your handle").should("exist");
      cy.pick("next-button").click();
      cy.pick("submit-button").click();
      cy.pick("profile-screen", "profile-avatar").contains(validUser.handle);
      cy.pick("profile-screen", "profile-avatar", "registered-badge").should(
        "exist"
      );
    });

    context(
      "when clicking cancel, close or hitting esc before the identity is created",
      () => {
        it("sends the user back to the intro screen", () => {
          cy.pick("get-started-button").click();
          cy.pick("cancel-button").click();

          // We should land back on the intro screen
          cy.pick("get-started-button").click();

          // Now try to close the modal via the "x" button
          cy.pick("modal-close-button").click();

          // We should land back on the intro screen
          cy.pick("get-started-button").click();

          // Now try the escape key
          cy.get("body").type("{esc}");

          // We should land back on the intro screen
          cy.pick("get-started-button").should("exist");
        });
      }
    );

    context(
      "when clicking the modal close button on the success screen",
      () => {
        it("lands the user on the profile screen", () => {
          cy.pick("get-started-button").click();

          cy.pick("form", "handle").type(validUser.handle);
          cy.pick("create-id-button").click();

          cy.pick("identity-card")
            .contains(validUser.shareableEntityIdentifier)
            .should("exist");

          // Land on profile screen
          cy.pick("modal-close-button").click();
          cy.pick("profile-avatar").contains(validUser.handle);
        });
      }
    );

    context("when pressing escape on the success screen", () => {
      it("lands the user on the profile screen", () => {
        cy.pick("get-started-button").click();

        cy.pick("form", "handle").type(validUser.handle);
        cy.pick("create-id-button").click();

        cy.pick("identity-card")
          .contains(validUser.shareableEntityIdentifier)
          .should("exist");

        // Now try the escape key
        cy.get("body").type("{esc}");

        // Land on profile screen
        cy.pick("profile-avatar").contains(validUser.handle);
      });
    });
  });

  context("validations", () => {
    beforeEach(() => {
      cy.pick("get-started-button").click();
      cy.pick("form", "handle").type("_rafalca");
      cy.pick("create-id-button").click();
    });

    context("handle", () => {
      const validationError = "Handle should match ^[a-z0-9][a-z0-9_-]+$";

      it("prevents the user from submitting an invalid handle", () => {
        // handle is required
        cy.pick("form", "handle").clear();
        cy.pick("form").contains("You must provide a handle");

        // no spaces
        cy.pick("form", "handle").type("no spaces");
        cy.pick("form").contains(validationError);

        // no special characters
        cy.pick("form", "handle").clear();
        cy.pick("form", "handle").type("$bad");
        cy.pick("form").contains(validationError);

        // can't start with an underscore
        cy.pick("form", "handle").clear();
        cy.pick("form", "handle").type("_nein");
        cy.pick("form").contains(validationError);

        // can't start with a dash
        cy.pick("form", "handle").clear();
        cy.pick("form", "handle").type("-não");
        cy.pick("form").contains(validationError);

        // has to be at least two characters long
        cy.pick("form", "handle").clear();
        cy.pick("form", "handle").type("x");
        cy.pick("form").contains(validationError);
      });
    });
  });
});
