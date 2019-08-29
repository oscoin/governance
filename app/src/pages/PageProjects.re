open AppStore;
open Atom;
open DesignSystem;
open Molecule;
open Source;
open StoreProjects;
open Particle;

module Styles = {
  open Css;

  let projectHeading = style([marginBottom(px(48))]);

  let listItem =
    style([
      borderBottom(px(1), solid, Color.lightGray()),
      padding(px(13)),
      hover([backgroundColor(Color.almostWhite())]),
      lastChild([borderBottomWidth(px(0))]),
    ]);
};

module List = {
  [@react.component]
  let make = (~projects: array(project)) => {
    let ps =
      Array.map(
        project =>
          <li className=Styles.listItem key={project.address}>
            <Link page={Router.Project(project.address)}>
              <ProjectCard
                imgUrl={project.imgUrl}
                name={project.name}
                description={project.description}
              />
            </Link>
          </li>,
        projects,
      );

    <ul> {React.array(ps)} </ul>;
  };
};

[@react.component]
let make = () => {
  let state = Store.useSelector(state => state.projectsState);
  let dispatch = Store.useDispatch();

  if (state.projects == None) {
    dispatch(StoreMiddleware.Thunk(ThunkProjects.fetchProjects)) |> ignore;
  };

  <El style=Positioning.gridMediumCentered>
    <div className=Styles.projectHeading>
      <El style=Layout.flex>
        <El style=Positioning.flexLeft>
          <Title.Huge> {React.string("Explore")} </Title.Huge>
        </El>
        <El style=Positioning.flexRight>
          <Link page=Router.RegisterProject>
            <Button> {React.string("Register project")} </Button>
          </Link>
        </El>
      </El>
    </div>
    {
      switch (state.error, state.loading, state.projects) {
      | (Some(error), _, _) =>
        <div className="error"> {React.string("ERROR: " ++ error)} </div>
      | (None, false, Some(projects)) => <List projects />
      | (None, true, _) => <div> {React.string("Loading...")} </div>
      | _ => <div> {React.string("Not loading...")} </div>
      }
    }
  </El>;
};
