// eslint-disable-next-line @typescript-eslint/triple-slash-reference,spaced-comment
/// <reference path="../../native/preload.d.ts" />
import { RendererMessage, MainMessage } from "../../native/ipc-types";
import { EventEmitter } from "events";
import * as sinon from "sinon";

// Stubs for Electron IPC message handlers.
//
// `ipcRenderer.invoke(msg, params)` uses the `msg` argument to look
// up the stub and call it.
interface ElectronStubs {
  [RendererMessage.GET_VERSION]: sinon.SinonStub;
  [RendererMessage.DIALOG_SHOWOPENDIALOG]: sinon.SinonStub;
  [RendererMessage.OPEN_PATH]: sinon.SinonStub;
  sendMessage: (message: MainMessage) => void;
}

declare global {
  interface Window {
    electronStubs: ElectronStubs;
  }
}

// Stub the Electron API on the window object and add an
// `ElectronStubs` object that can be obtained with `getStubs()`.
//
// See `../../native/preload.js`.
export function setup(window: Window): void {
  const ipcRendererMessages = new EventEmitter();
  const electronStubs: ElectronStubs = {
    [RendererMessage.GET_VERSION]: sinon
      .stub()
      .returns(Promise.resolve("v1.2.3")),
    [RendererMessage.DIALOG_SHOWOPENDIALOG]: sinon
      .stub()
      .throws(new Error("not implemented")),
    [RendererMessage.OPEN_PATH]: sinon
      .stub()
      .throws(new Error("not implemented")),
    sendMessage: (message: MainMessage) => {
      ipcRendererMessages.emit("message", undefined, message);
    },
  };

  window.electronStubs = electronStubs;

  window.electron = {
    ipcRenderer: {
      invoke: (msg, params) => {
        return electronStubs[msg](params);
      },
      on: ipcRendererMessages.on.bind(ipcRendererMessages),
    },
    isDev: true,
    isExperimental: true,
  };
}

// Get the `ElectronStubs` object to stub and observe interactions of
// the app with the main process.
export function getStubs(): Cypress.Chainable<ElectronStubs> {
  return cy.window().then(w => w.electronStubs);
}
