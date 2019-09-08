module Styles = {
  open Css;

  global(
    "body",
    [
      color(Particle.Color.black()),
      unsafe(" -webkit-font-smoothing", "antialiased"),
      unsafe(" -moz-osx-font-smoothing", "grayscale"),
      ...Particle.Font.text,
    ],
  );

  global(
    "a",
    [
      color(Particle.Color.black()),
      cursor(`pointer),
      textDecoration(none),
    ],
  );
};

[@react.component]
let make = () => {
  open DesignSystem;
  open Page;
  open Router;

  let httpLink =
    ApolloLinks.createHttpLink(~uri="http://localhost:8080/graphql", ());
  let client =
    ReasonApollo.createApolloClient(
      ~link=httpLink,
      ~cache=ApolloInMemoryCache.createInMemoryCache(),
      (),
    );

  let page =
    switch (currentPage()) {
    | Root => <Generic title="Home of Oscoin" />
    | Projects => <Projects />
    | Styleguide => <Styleguide />
    | RegisterProject => <RegisterProject />
    | Project(address) => <Project address />
    | NotFound(_path) => <Generic title="Not Found" />
    };

  currentPage() == Router.Styleguide ?
    page :
    <Store.Provider>
      <ReasonApolloHooks.ApolloProvider client>
        <El style=Layout.grid>
          <El style={Positioning.gridWideCentered << margin(32, 0, 0, 0)}>
            <Topbar />
          </El>
          page
        </El>
      </ReasonApolloHooks.ApolloProvider>
    </Store.Provider>;
};
