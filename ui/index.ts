import qs from "qs";

import { backendAddressStore } from "./src/api";
import App from "./App.svelte";

// Our integration tests simulate environments where multiple Upstream clients
// interact with each other. However, Cypress is limited to interacting only
// with a single browser, multiple tabs within a browser aren't possible
// either.
//
// As a workaround to this limitation we allow passing in a backend address via
// a query parameter. This way we can tell the UI to talk to a specific
// backend and we're not violating the same-origin policy that Cypress imposes.
//
// When talking to a specific node within the tests we have to first set the
// `auth-token` cookie and then launch the Cypress browser, starting the UI
// pointing to our specified backend like so:
//
//     cy.setCookie("auth-token", node.authToken);
//     cy.visit("./public/index.html?backend=localhost:17000");

const queryString = qs.parse(window.location.search.replace("?", ""));

if (queryString.backend) {
  backendAddressStore.set(queryString.backend as string);
}

const app = new App({
  target: document.body,
});

export default app;
