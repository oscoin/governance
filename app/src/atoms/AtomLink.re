open Router;
open Css;

module Styles = {
  let link = style([]);
};

[@react.component]
let make = (~page: page, ~style=?, ~children=?) => {
  let content =
    switch (children) {
    | Some(child) => child
    | None => React.string(nameOfPage(page))
    };

  let style =
    switch (style) {
    | Some(style) => merge([Styles.link, style])
    | None => Styles.link
    };

  <a onClick={navigateOfPage(page)} className=style> content </a>;
};
