import * as api from "./api";

// TYPES
export interface EmojiAvatar {
  background: RGBValue;
  emoji: string;
}

export interface RGBValue {
  r: number;
  g: number;
  b: number;
}

export interface RemoteAvatar {
  url: string;
}

export type Avatar = EmojiAvatar | RemoteAvatar;

export enum Usage {
  Any = "any",
  Identity = "identity",
}

// EVENTS
export const getAvatar = (usage: Usage, id: string): Promise<EmojiAvatar> =>
  api.get<EmojiAvatar>(`avatars/${id}?usage=${usage}`);
