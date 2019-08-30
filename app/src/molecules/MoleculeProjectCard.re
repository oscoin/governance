open Atom;
open DesignSystem;

module Styles = {
  open Css;

  let item = style([display(`flex), flex(`num(1.0)), width(pct(100.0))]);

  let description =
    style([
      display(`flex),
      flexDirection(`column),
      justifyContent(`center),
    ]);

  let image =
    style([height(px(64)), marginRight(px(21)), width(px(64))]);

  let imageContainer = style([marginRight(px(22)), display(inherit_)]);
};

module Template = {
  [@react.component]
  let make = (~children, ~imgUrl=?, ~description) => {
    let image =
      switch (imgUrl) {
      | Some(imgUrl) => <img className=Styles.image src=imgUrl />
      | None =>
        <El style=Styles.imageContainer>
          <Icon.ProjectAvatarPlaceholder />
        </El>
      };

    <div className=Styles.item>
      image
      <div className=Styles.description>
        children
        <Text> {React.string(description)} </Text>
      </div>
    </div>;
  };
};

[@react.component]
let make = (~imgUrl=?, ~name, ~description) =>
  <Template ?imgUrl description>
    <Title> {React.string(name)} </Title>
  </Template>;

module Alternate = {
  [@react.component]
  let make = (~imgUrl=?, ~name, ~description) =>
    <Template ?imgUrl description>
      <Title.Big style={margin(0, 0, 6, 0)}>
        {React.string(name)}
      </Title.Big>
    </Template>;
};
