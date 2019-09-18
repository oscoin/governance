type account = {
  avatarUrl: string,
  keyName: string,
};

type fetchAccountResult = Belt.Result.t(option(account), string);

type createAccountResult = Belt.Result.t(account, string);

type source = {
  createAccount: (string, string) => Js.Promise.t(createAccountResult),
  fetchAccount: unit => Js.Promise.t(fetchAccountResult),
};

module StorageLabel = {
  let keyName = "keyName";
  let avatarUrl = "avatarUrl";
};

let createLocalSource = () => {
  let createAccount = (keyName, avatarUrl) =>
    Js.Promise.make((~resolve, ~reject as _) => {
      Dom.Storage.(localStorage |> setItem(StorageLabel.keyName, keyName));
      Dom.Storage.(
        localStorage |> setItem(StorageLabel.avatarUrl, avatarUrl)
      );

      let account = {keyName, avatarUrl};
      resolve(. Belt.Result.Ok(account));
    });

  let fetchAccount = () =>
    Js.Promise.make((~resolve, ~reject as _) => {
      let keyName =
        Dom.Storage.(localStorage |> getItem(StorageLabel.keyName));
      let avatarUrl =
        Dom.Storage.(localStorage |> getItem(StorageLabel.avatarUrl));

      let account =
        switch (keyName) {
        | None => None
        | Some(keyName) =>
          Some({
            keyName,
            avatarUrl: Belt.Option.getWithDefault(avatarUrl, ""),
          })
        };

      resolve(. Belt.Result.Ok(account));
    });

  {createAccount, fetchAccount};
};
