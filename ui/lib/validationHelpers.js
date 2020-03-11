// TODO: remove and replace with validation queries for proxy

// Single word
export const SINGLE_WORD_MATCH = new RegExp("^[a-z0-9][a-z0-9_-]+$", "i");

// General name match
export const NAME_MATCH = new RegExp("^[a-z0-9 ]+$", "i");

export const IMAGE_FILENAME = new RegExp(
  "^(https|http)://\\S+.(gif|jpe?g|tiff|png|webp|bmp)$",
  "i"
);
