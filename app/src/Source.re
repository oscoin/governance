/* We use the Random module for fake address creation. */
Random.self_init();

type address = string;

type account = {
  avatarUrl: string,
  keyName: string,
};

type fetchAccountResult = Belt.Result.t(option(account), string);

type project = {
  address,
  name: string,
  description: string,
  imgUrl: string,
  members: array(account),
};

type createAccountResult = Belt.Result.t(account, string);
type fetchProjectResult = Belt.Result.t(project, string);
type fetchProjectsResult = Belt.Result.t(array(project), string);
type registerProjectResult = Belt.Result.t(project, string);

type source = {
  createAccount: (string, string) => Js.Promise.t(createAccountResult),
  fetchAccount: unit => Js.Promise.t(fetchAccountResult),
  fetchProject: address => Js.Promise.t(fetchProjectResult),
  fetchProjects: unit => Js.Promise.t(fetchProjectsResult),
  registerProject:
    (~name: string, ~description: string, ~imgUrl: string) =>
    Js.Promise.t(registerProjectResult),
};

let createLocalSource = () => {
  let localAccount = ref(None);
  let localProjects =
    ref([|
      {
        address: "monokel",
        name: "monokel",
        description: "A looking glass into the future.",
        imgUrl: "https://res.cloudinary.com/juliendonck/image/upload/v1557488019/Frame_2_bhz6eq.svg",
        members: [|
          {
            keyName: "xla",
            avatarUrl: "https://avatars0.githubusercontent.com/u/1585",
          },
        |],
      },
      {
        address: "monadic",
        name: "Monadic",
        description: "Open source organization of amazing things",
        imgUrl: "https://res.cloudinary.com/juliendonck/image/upload/v1549554598/monadic-icon_myhdjk.svg",
        members: [|
          {
            keyName: "cloudhead",
            avatarUrl: "https://avatars1.githubusercontent.com/u/40774",
          },
          {
            keyName: "lftherios",
            avatarUrl: "https://avatars3.githubusercontent.com/u/853825",
          },
          {
            keyName: "juliendonck",
            avatarUrl: "https://avatars2.githubusercontent.com/u/2326909",
          },
        |],
      },
      {
        address: "oscoin",
        name: "open source coin",
        description: "Infrastructure for the open source community",
        imgUrl: "https://avatars0.githubusercontent.com/u/31632242",
        members: [|
          {
            keyName: "geigerzaehler",
            avatarUrl: "https://avatars2.githubusercontent.com/u/3919579",
          },
          {
            keyName: "rockbmb",
            avatarUrl: "https://avatars2.githubusercontent.com/u/16455833",
          },
          {
            keyName: "rudolfs",
            avatarUrl: "https://avatars1.githubusercontent.com/u/158411",
          },
        |],
      },
      {
        address: "radicle",
        name: "radicle",
        description: "Decentralized open source collaboration",
        imgUrl: "https://avatars0.githubusercontent.com/u/48290027",
        members: [|
          {
            keyName: "jkarni",
            avatarUrl: "https://avatars3.githubusercontent.com/u/1657498",
          },
        |],
      },
    |]);

  let createAccount = (keyName, avatarUrl) =>
    Js.Promise.make((~resolve, ~reject as _) => {
      let account = {keyName, avatarUrl};
      localAccount := Some(account);

      resolve(. Belt.Result.Ok(account));
    });

  let fetchAccount = () =>
    Js.Promise.make((~resolve, ~reject as _) =>
      resolve(. Belt.Result.Ok(localAccount^))
    );

  let fetchProject = addr =>
    Js.Promise.make((~resolve, ~reject as _) => {
      let maybeProj = Belt.Array.getBy(localProjects^, p => addr == p.address);

      switch (maybeProj) {
      | Some(project) => resolve(. Belt.Result.Ok(project))
      | None => resolve(. Belt.Result.Error("Not Found"))
      };
    });

  let fetchProjects = () =>
    Js.Promise.make((~resolve, ~reject as _) =>
      Js.Global.setTimeout(
        () => resolve(. Belt.Result.Ok(localProjects^)),
        1000,
      )
      |> ignore
    );

  let registerProject = (~name: string, ~description: string, ~imgUrl: string) =>
    Js.Promise.make((~resolve, ~reject as _) =>
      switch (localAccount^) {
      | Some(account) =>
        let project = {
          address: Printf.sprintf("%X", Random.bits()),
          name,
          description,
          imgUrl,
          members: [|account|],
        };

        localProjects := Array.append(localProjects^, [|project|]);

        resolve(. Belt.Result.Ok(project));
      | None => resolve(. Belt.Result.Error("no account present"))
      }
    );

  {createAccount, fetchAccount, fetchProject, fetchProjects, registerProject};
};
