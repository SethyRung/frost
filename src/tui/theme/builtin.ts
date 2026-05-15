import type { ThemeJson } from "./types";

import auraTheme from "./themes/aura.json";
import ayuTheme from "./themes/ayu.json";
import carbonfoxTheme from "./themes/carbonfox.json";
import catppuccinTheme from "./themes/catppuccin.json";
import catppuccinFrappeTheme from "./themes/catppuccin-frappe.json";
import catppuccinMacchiatoTheme from "./themes/catppuccin-macchiato.json";
import cobalt2Theme from "./themes/cobalt2.json";
import cursorTheme from "./themes/cursor.json";
import draculaTheme from "./themes/dracula.json";
import everforestTheme from "./themes/everforest.json";
import flexokiTheme from "./themes/flexoki.json";
import githubTheme from "./themes/github.json";
import gruvboxTheme from "./themes/gruvbox.json";
import kanagawaTheme from "./themes/kanagawa.json";
import lucentOrngTheme from "./themes/lucent-orng.json";
import materialTheme from "./themes/material.json";
import matrixTheme from "./themes/matrix.json";
import mercuryTheme from "./themes/mercury.json";
import monokaiTheme from "./themes/monokai.json";
import nightowlTheme from "./themes/nightowl.json";
import nordTheme from "./themes/nord.json";
import oneDarkTheme from "./themes/one-dark.json";
import opencodeTheme from "./themes/opencode.json";
import orngTheme from "./themes/orng.json";
import osakaJadeTheme from "./themes/osaka-jade.json";
import palenightTheme from "./themes/palenight.json";
import rosepineTheme from "./themes/rosepine.json";
import solarizedTheme from "./themes/solarized.json";
import synthwave84Theme from "./themes/synthwave84.json";
import tokyonightTheme from "./themes/tokyonight.json";
import vercelTheme from "./themes/vercel.json";
import vesperTheme from "./themes/vesper.json";
import zenburnTheme from "./themes/zenburn.json";

export const DEFAULT_THEME_ID = "opencode";

export const DEFAULT_THEMES: Record<string, ThemeJson> = {
  aura: auraTheme,
  ayu: ayuTheme,
  carbonfox: carbonfoxTheme,
  catppuccin: catppuccinTheme,
  "catppuccin-frappe": catppuccinFrappeTheme,
  "catppuccin-macchiato": catppuccinMacchiatoTheme,
  cobalt2: cobalt2Theme,
  cursor: cursorTheme,
  dracula: draculaTheme,
  everforest: everforestTheme,
  flexoki: flexokiTheme,
  github: githubTheme,
  gruvbox: gruvboxTheme,
  kanagawa: kanagawaTheme,
  "lucent-orng": lucentOrngTheme,
  material: materialTheme,
  matrix: matrixTheme,
  mercury: mercuryTheme,
  monokai: monokaiTheme,
  nightowl: nightowlTheme,
  nord: nordTheme,
  "one-dark": oneDarkTheme,
  opencode: opencodeTheme,
  orng: orngTheme,
  "osaka-jade": osakaJadeTheme,
  palenight: palenightTheme,
  rosepine: rosepineTheme,
  solarized: solarizedTheme,
  synthwave84: synthwave84Theme,
  tokyonight: tokyonightTheme,
  vercel: vercelTheme,
  vesper: vesperTheme,
  zenburn: zenburnTheme,
};

export function getBuiltinThemeIds(): string[] {
  return Object.keys(DEFAULT_THEMES);
}
