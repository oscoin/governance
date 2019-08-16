open Source.Project;
open Router;
open Atom;

module List = {
  [@react.component]
  let make = (~projects: array(project)) => {
    let ps =
      Array.map(
        project =>
          <li key={project.address}>
            <Link page={Project(project.address)}>
              <Title> {React.string(project.name)} </Title>
              <p> {React.string(project.description)} </p>
              <img src={project.imgUrl} />
            </Link>
          </li>,
        projects,
      );

    <ul> {React.array(ps)} </ul>;
  };
};

type action =
  | ProjectsFetched(array(project));

type state =
  | Loading
  | Fetched(array(project))
  | Failed(string);

[@react.component]
let make = () => {
  let (state, dispatch) =
    React.useReducer(
      (_state, action) =>
        switch (action) {
        | ProjectsFetched(ps) => Fetched(ps)
        },
      Loading,
    );

  React.useEffect0(() => {
    getProjects()
    |> Js.Promise.then_(projects =>
         ProjectsFetched(projects) |> dispatch |> Js.Promise.resolve
       )
    |> ignore;

    None;
  });

  <>
    <div>
      <Title.Huge> {React.string("Explore")} </Title.Huge>
      <Button> {React.string("Register project")} </Button>
    </div>
    {
      switch (state) {
      | Loading => <div> {React.string("Loading...")} </div>
      | Fetched(projects) => <List projects />
      | Failed(_error) => <div className="error" />
      }
    }
  </>;
};
