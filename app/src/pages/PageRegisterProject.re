open Atom;
open DesignSystem;

module Styles = {
  open Css;

  let content = style([textAlign(center)]) << Positioning.gridNarrowCentered;

  let buttonContainer = style([display(`flex), justifyContent(flexEnd)]);
};

[@react.component]
let make = () => {
  open Router;

  let dispatch = Store.useDispatch();
  let (name, _setName) = React.useState(() => "mvp");
  let (description, _setDescription) =
    React.useState(() => "minimal viable product");
  let (imgUrl, _setImgUrl) = React.useState(() => "");

  let registerCallback = _ =>
    dispatch(
      StoreMiddleware.Thunk(
        ThunkProjects.registerProject(name, description, imgUrl),
      ),
    );

  <El style=Styles.content>
    <Title.Big style={margin(0, 0, 16, 0)}>
      {React.string("Register a project")}
    </Title.Big>
    <Text> {React.string("Register a project on the network")} </Text>
    <El style={margin(48, 0, 24, 0)}>
      <Input
        style={margin(0, 0, 16, 0)}
        placeholder="Enter the project name"
      />
      <Input
        style={margin(0, 0, 16, 0)}
        placeholder="Enter your project description"
      />
      <Input placeholder="Add a project picture" />
    </El>
    <El style=Styles.buttonContainer>
      <Button.Cancel onClick={navigateToPage(Projects)}>
        {React.string("Cancel")}
      </Button.Cancel>
      <Button.Secondary onClick=registerCallback>
        {React.string("Register")}
      </Button.Secondary>
    </El>
  </El>;
};
