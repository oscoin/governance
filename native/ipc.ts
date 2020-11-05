// eslint-disable-next-line @typescript-eslint/triple-slash-reference,spaced-comment
/// <reference path="../native/preload.d.ts" />

export const CLIPBOARD_WRITETEXT = "IPC_CLIPBOARD_WRITETEXT";
export const DIALOG_SHOWOPENDIALOG = "IPC_DIALOG_SHOWOPENDIALOG";
export const GET_VERSION = "GET_VERSION";
export const OPEN_PATH = "IPC_OPEN_PATH";

// We have to be able to select empty directories when we create new
// projects. Unfortunately we can't use the HTML5 open dialog via
// <input type="file"> for this. Although it lets us select directories,
// it doesn't fire an event when an empty directory is selected.
//
// The workaround is to use the electron native open dialog. As a bonus we
// can configure it to allow users to create new directories.
export const getDirectoryPath = (): Promise<string> =>
  window.electron.ipcRenderer.invoke(DIALOG_SHOWOPENDIALOG);

export const getVersion = (): Promise<string> =>
  window.electron.ipcRenderer.invoke(GET_VERSION);

export const copyToClipboard = (text: string): Promise<void> =>
  window.electron.ipcRenderer.invoke(CLIPBOARD_WRITETEXT, text);

export const openPath = (path: string): Promise<void> =>
  window.electron.ipcRenderer.invoke(OPEN_PATH, path);

// Informs whether it's running in a development environment.
export const isDev = (): boolean => {
  return window.electron.isDev;
};

// Informs whether it's running in experimental mode, where
// features under construction are enabled and can thus be used.
// This option can only be enabled iff `isDev()` as we should only
// want to toggle it while in development mode.
export const isExperimental = (): boolean => {
  return isDev() && window.electron.isExperimental;
};
